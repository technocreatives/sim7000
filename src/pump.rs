use crate::{
    at_command::unsolicited::NewSmsIndex, modem::power::PowerSignalListener, BuildIo, PowerState,
    SplitIo, StateSignal,
};
use core::{future::Future, str::from_utf8};
use embassy_futures::select::{select3, Either3};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Receiver, Sender},
    pipe::Pipe,
    signal::Signal,
};
use embassy_time::{with_timeout, Duration};
use embedded_io_async::{Read, Write};
use futures::{select_biased, FutureExt};
use heapless::Vec;

use crate::at_command::{
    unsolicited::{
        GnssReport, NetworkRegistration, PowerDown, RegistrationStatus, Urc, VoltageWarning,
    },
    AtParseLine, ResponseCode,
};
use crate::log;
use crate::modem::{ModemContext, RawAtCommand, TcpContext};
use crate::read::ModemReader;
use crate::Error;

pub const PUMP_COUNT: usize = 3;

pub trait Pump {
    type Err;

    fn pump(&mut self) -> impl Future<Output = Result<(), Self::Err>>;
}

pub struct RxPump<'context> {
    pub(crate) reader: ModemReader<'context>,
    pub(crate) generic_response: Sender<'context, CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) tcp: &'context TcpContext,
    pub(crate) gnss: &'context Signal<CriticalSectionRawMutex, GnssReport>,
    pub(crate) voltage_warning: &'context Signal<CriticalSectionRawMutex, VoltageWarning>,
    pub(crate) registration_events:
        &'context StateSignal<CriticalSectionRawMutex, NetworkRegistration>,
    pub(crate) sms_indices: Sender<'context, CriticalSectionRawMutex, NewSmsIndex, 5>,
}

impl<'context> Pump for RxPump<'context> {
    type Err = Error;

    async fn pump(&mut self) -> Result<(), Self::Err> {
        let line = self.reader.read_line().await?;

        if line.is_empty() {
            log::warn!("received empty line from modem");
        }

        if let Ok(message) = Urc::from_line(&line) {
            // First, check if it's an unsolicited message

            log::debug!("Got URC: {:?}", line.as_str());
            match message {
                Urc::NetworkRegistration(registration) => {
                    log::info!("registration status: {:?}", registration);
                    self.registration_events.signal(registration);
                }
                Urc::ReceiveHeader(header) => {
                    let mut length = header.length;
                    let connection = header.connection;
                    log::debug!("Reading {} bytes from modem", length);
                    while length > 0 {
                        log::debug!("remaining read: {}", length);
                        let mut buf = Vec::<u8, 365>::new();
                        buf.resize_default(usize::min(length, buf.capacity()))
                            .unwrap();
                        self.reader.read_exact(&mut buf).await?;
                        length -= buf.len();
                        log::debug!(
                            "Sending {} bytes to tcp connection {}",
                            buf.len(),
                            connection
                        );
                        self.tcp.slots[connection].peek().rx.write_all(&buf).await;
                        log::debug!("Bytes sent to tcp connection {}", connection);
                    }
                    log::debug!("Done sending to tcp connection {}", connection);
                }
                Urc::Cmti(message) => {
                    if let Err(e) = self.sms_indices.try_send(message) {
                        log::error!("Failed to send SMS index: {:?}", e);
                    }
                }
                Urc::ConnectionMessage(message) => {
                    let slot = &self.tcp.slots[message.index];
                    slot.peek().events.send(message.message);
                }
                Urc::GnssReport(report) => {
                    self.gnss.signal(report);
                }
                Urc::VoltageWarning(warning) => {
                    self.voltage_warning.signal(warning);
                }
                Urc::PowerDown(PowerDown::UnderVoltage) => {
                    self.voltage_warning.signal(VoltageWarning::UnderVoltage);
                }
                Urc::PowerDown(PowerDown::OverVoltage) => {
                    self.voltage_warning.signal(VoltageWarning::OverVoltage);
                }
                _ => log::warn!("Unhandled URC: {:?}", message),
            }
        } else if let Ok(mut response) = ResponseCode::from_line(&line) {
            // If it's not a URC, try to parse it as a regular response code

            // Sms messages are a bit of a special case,
            // first comes the info and then the message on a new line
            // and a sms message can't be unambiguously parsed seperatly
            if let ResponseCode::SmsMessage(sms) = &mut response {
                log::info!("Got SMS from: {:?}, reading message", sms.sender);
                let line = self.reader.read_line().await?;

                if line.is_empty() {
                    log::warn!("received empty line from modem");
                }

                sms.message = line[..line.len()].into();
            }

            log::debug!("Got generic response: {:?}", line.as_str());
            if with_timeout(
                Duration::from_secs(10),
                self.generic_response.send(response),
            )
            .await
            .is_err()
            {
                log::error!("message queue send timed out");
            }
        } else {
            // The modem likely sent us gibberish we could not understand.
            // TODO: We might want to trigger a reboot when the modem starts acting like this.
            log::error!("Got unknown response: {:?}", line.as_str());
        }

        Ok(())
    }
}

pub struct TxPump<'context> {
    pub(crate) writer: &'context Pipe<CriticalSectionRawMutex, 2048>,
    pub(crate) commands: Receiver<'context, CriticalSectionRawMutex, RawAtCommand, 4>,
}

impl<'context> Pump for TxPump<'context> {
    type Err = Error;

    async fn pump(&mut self) -> Result<(), Self::Err> {
        let command = self.commands.receive().await;
        match &command {
            RawAtCommand::Text(text) => log::debug!("Write to modem: {:?}", text.as_str()),
            RawAtCommand::Binary(bytes) => log::debug!("Write {} bytes to modem", bytes.len()),
        }

        // `Writer` is infallible. It is fine to ignore these errors.
        let _ = self.writer.write_all(command.as_bytes()).await;
        let _ = self.writer.flush().await;

        Ok(())
    }
}

pub struct DropPump<'context> {
    pub(crate) context: &'context ModemContext,
    pub(crate) power_signal: PowerSignalListener<'context>,
    pub(crate) power_state: PowerState,
}

impl<'context> Pump for DropPump<'context> {
    type Err = Error;

    async fn pump(&mut self) -> Result<(), Self::Err> {
        select_biased! {
            power_state = self.power_signal.listen().fuse() => {
                self.power_state = power_state;
            }
            drop_message = self.context.drop_channel.receive().fuse() => {
                if self.power_state == PowerState::On {
                    // run drop command, abort if power state changes
                    let result = select_biased! {
                        power_state = self.power_signal.listen().fuse() => {
                            self.power_state = power_state;
                            Ok(())
                        }
                        result = async {
                            // run drop command
                            let runner = self.context.commands();
                            let mut runner = runner.lock().await;
                            drop_message.run(&mut runner).await
                        }.fuse() => result,
                    };

                    // clean up regardless of whether drop command succeeded
                    drop_message.clean_up(self.context);
                    result?;
                } else {
                    drop_message.clean_up(self.context);
                }
            },
        }

        Ok(())
    }
}

pub struct RawIoPump<'context, RW> {
    pub(crate) io: RW,
    /// sends data to the rx pump
    pub(crate) rx: &'context Pipe<CriticalSectionRawMutex, 2048>,
    /// reads data from the tx pump
    pub(crate) tx: &'context Pipe<CriticalSectionRawMutex, 2048>,
    pub(crate) power_signal: PowerSignalListener<'context>,
    pub(crate) power_state: PowerState,
}

impl<'context, RW: 'static + BuildIo> RawIoPump<'context, RW> {
    pub async fn high_power_pump(&mut self) -> Result<(), Error> {
        let mut io = Some(self.io.build());
        let (mut reader, mut writer) = RW::IO::split(&mut io);

        loop {
            let mut tx_buf = [0u8; 256];
            let mut rx_buf = [0u8; 256];

            match select3(
                self.tx.read(&mut tx_buf),
                reader.read(&mut rx_buf),
                self.power_signal.listen(),
            )
            .await
            {
                Either3::First(bytes) => {
                    writer
                        .write_all(&tx_buf[..bytes])
                        .await
                        .map_err(|_| Error::Serial)?;
                    writer.flush().await.map_err(|_| Error::Serial)?;
                }
                Either3::Second(result) => {
                    let bytes = result.map_err(|_| Error::Serial)?;

                    match from_utf8(&rx_buf[..bytes]) {
                        Ok(line) => log::trace!("BYTES READ {:?}", line),
                        Err(_) => log::trace!("READ INVALID {:?}", &rx_buf[..bytes]),
                    }

                    self.rx.write_all(&rx_buf[..bytes]).await;
                    // TODO: this flush is probably not needed
                    self.rx.flush().await.ok(/* infallible */);
                }
                Either3::Third(result) => {
                    self.power_state = result;
                    if self.power_state != PowerState::On {
                        break Ok(());
                    }
                }
            }
        }
    }

    pub async fn low_power_pump(&mut self) {
        self.power_state = self.power_signal.listen().await;
    }
}

impl<'context, RW: 'static + BuildIo> Pump for RawIoPump<'context, RW> {
    type Err = Error;

    async fn pump(&mut self) -> Result<(), Self::Err> {
        if self.power_state != PowerState::Off {
            self.high_power_pump().await?;
        } else {
            self.low_power_pump().await;
        }

        Ok(())
    }
}

pub struct RegistrationHandler<'context> {
    context: &'context Signal<CriticalSectionRawMutex, NetworkRegistration>,
}

impl<'context> RegistrationHandler<'context> {
    pub async fn pump(&mut self) {
        match self.context.wait().await.status {
            RegistrationStatus::NotRegistered
            | RegistrationStatus::Searching
            | RegistrationStatus::RegistrationDenied
            | RegistrationStatus::Unknown => todo!(),
            RegistrationStatus::RegisteredHome => todo!(),
            RegistrationStatus::RegisteredRoaming => todo!(),
        }
    }
}

#[macro_export]
macro_rules! pump_task {
    ($name:ident, $type:ty) => {
        #[embassy_executor::task]
        pub(crate) async fn $name(mut pump: $type) {
            use ::sim7000_async::pump::Pump;
            loop {
                if let Err(err) = pump.pump().await {
                    #[cfg(feature = "log")]
                    log::error!("Error pumping {} {:?}", stringify!($name), err);
                    #[cfg(feature = "defmt")]
                    defmt::error!("Error pumping {} {:?}", stringify!($name), err);
                }
            }
        }
    };
}
