use crate::{BuildIo, SerialError, SplitIo};
use core::{future::Future, str::from_utf8};
use embassy_futures::select::{select, Either};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Receiver, Sender},
    pipe::{Reader, Writer},
    signal::Signal,
};
use embassy_time::{with_timeout, Duration};
use embedded_io::asynch::{Read, Write};
use heapless::Vec;

use crate::at_command::{
    response::ResponseCode,
    unsolicited::{GnssReport, PowerDown, RegistrationStatus, Urc, VoltageWarning},
    ATParseLine,
};
use crate::log;
use crate::modem::{ModemContext, RawAtCommand, TcpContext};
use crate::read::ModemReader;
use crate::Error;

pub const PUMP_COUNT: usize = 3;

pub trait Pump {
    type Err;
    type Fut<'a>: Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_>;
}

pub struct RxPump<'context> {
    pub(crate) reader: ModemReader<'context>,
    pub(crate) generic_response: Sender<'context, CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) tcp: &'context TcpContext,
    pub(crate) gnss: &'context Signal<GnssReport>,
    pub(crate) voltage_warning: &'context Signal<VoltageWarning>,
    pub(crate) registration_events: &'context Signal<RegistrationStatus>,
}

impl<'context> Pump for RxPump<'context> {
    type Err = Error;
    type Fut<'a> = impl Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_> {
        async {
            let line = self.reader.read_line().await?;

            if line.is_empty() {
                log::warn!("received empty line from modem");
            }

            if let Ok(message) = Urc::from_line(&line) {
                // First, check if it's an unsolicited message

                log::info!("Got URC: {:?}", line.as_str());
                match message {
                    Urc::RegistrationStatus(status) => {
                        self.registration_events.signal(status);
                    }
                    Urc::ReceiveHeader(header) => {
                        let mut length = header.length;
                        let connection = header.connection;
                        log::info!("Reading {} bytes from modem", length);
                        while length > 0 {
                            log::debug!("remaining read: {}", length);
                            let mut buf = Vec::new();
                            buf.resize_default(usize::min(length, 365)).unwrap();
                            self.reader.read_exact(&mut buf).await?;
                            length -= buf.len();
                            log::info!(
                                "Sending {} bytes to tcp connection {}",
                                buf.len(),
                                connection
                            );
                            self.tcp.slots[connection].peek().rx.send(buf).await;
                            log::info!("Bytes sent to tcp connection {}", connection);
                        }
                        log::info!("Done sending to tcp connection {}", connection);
                    }
                    Urc::ConnectionMessage(message) => {
                        self.tcp.slots[message.index]
                            .peek()
                            .events
                            .send(message.message)
                            .await;
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
            } else if let Ok(response) = ResponseCode::from_line(&line) {
                // If it's not a URC, try to parse it as a regular response code

                log::info!("Got generic response: {:?}", line.as_str());
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
}

pub struct TxPump<'context> {
    pub(crate) writer: Writer<'context, CriticalSectionRawMutex, 2048>,
    pub(crate) commands: Receiver<'context, CriticalSectionRawMutex, RawAtCommand, 4>,
}

impl<'context> Pump for TxPump<'context> {
    type Err = Error;
    type Fut<'a> = impl Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_> {
        async {
            let command = self.commands.recv().await;
            match &command {
                RawAtCommand::Text(text) => log::info!("Write to modem: {:?}", text.as_str()),
                RawAtCommand::Binary(bytes) => log::info!("Write {} bytes to modem", bytes.len()),
            }
            self.writer.write_all(command.as_bytes()).await;
            self.writer.flush().await;

            Ok(())
        }
    }
}

pub struct DropPump<'context> {
    pub(crate) context: &'context ModemContext,
}

impl<'context> Pump for DropPump<'context> {
    type Err = Error;
    type Fut<'a> = impl Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_> {
        async {
            let drop_message = self.context.drop_channel.recv().await;
            let runner = self.context.commands();
            let mut runner = runner.lock().await;
            drop_message.run(&mut runner).await?;
            drop(runner);
            drop_message.clean_up(self.context);

            Ok(())
        }
    }
}

pub struct RawIoPump<'context, RW> {
    pub(crate) io: RW,
    // sends data to the rx pump
    pub(crate) rx: Writer<'context, CriticalSectionRawMutex, 2048>,
    // reads data from the tx pump
    pub(crate) tx: Reader<'context, CriticalSectionRawMutex, 2048>,
}

impl<'context, RW: 'static + BuildIo> Pump for RawIoPump<'context, RW> {
    type Err = Error;
    type Fut<'a> = impl Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_> {
        async {
            let mut io = self.io.build();
            let (mut reader, mut writer) = io.split();
            loop {
                let mut tx_buf = [0u8; 256];
                let mut rx_buf = [0u8; 256];

                match select(self.tx.read(&mut tx_buf), reader.read(&mut rx_buf)).await {
                    Either::First(bytes) => {
                        writer
                            .write_all(&tx_buf[..bytes])
                            .await
                            .map_err(|_| Error::Serial)?;
    	                writer.flush().await;

                    }
                    Either::Second(result) => {
                        let bytes = result.map_err(|_| Error::Serial)?;

                        match from_utf8(&rx_buf[..bytes]).map_err(|_| "Not UTF-8") {
                            Ok(line) => log::debug!(
                                "BYTES READ {:?}",
                                line
                            ),
                            Err(err) => log::debug!(
                                "READ INVALID {:?}",
                                &rx_buf[..bytes]
                            ),
                        }

                        self.rx.write_all(&rx_buf[..bytes]).await;
                        self.rx.flush().await;
                    }
                }
            }
        }
    }
}

pub struct RegistrationHandler<'context> {
    context: &'context Signal<RegistrationStatus>,
}

impl<'context> RegistrationHandler<'context> {
    pub async fn pump(&mut self) {
        match self.context.wait().await {
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
