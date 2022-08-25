use crate::write::Write;
use core::future::Future;
use embassy_executor::time::{with_timeout, Duration};
use embassy_util::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::mpmc::{Receiver, Sender},
    channel::signal::Signal,
};
use heapless::Vec;

use crate::at_command::{
    response::ResponseCode,
    unsolicited::{GnssReport, PowerDown, RegistrationStatus, Urc, VoltageWarning},
    ATParseLine,
};
use crate::modem::{ModemContext, RawAtCommand, TcpContext};
use crate::read::{ModemReader, Read};
use crate::Error;

pub const PUMP_COUNT: usize = 3;

pub trait Pump {
    type Err;
    type Fut<'a>: Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_>;
}

pub struct RxPump<'context, R> {
    pub(crate) reader: ModemReader<R>,
    pub(crate) generic_response: Sender<'context, CriticalSectionRawMutex, ResponseCode, 1>,
    pub(crate) tcp: &'context TcpContext,
    pub(crate) gnss: &'context Signal<GnssReport>,
    pub(crate) voltage_warning: &'context Signal<VoltageWarning>,
    pub(crate) registration_events: &'context Signal<RegistrationStatus>,
}

impl<'context, R: Read> Pump for RxPump<'context, R> {
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

                log::info!("Got URC: {line:?}");
                match message {
                    Urc::RegistrationStatus(status) => {
                        self.registration_events.signal(status);
                    }
                    Urc::ReceiveHeader(header) => {
                        let mut length = header.length;
                        let connection = header.connection;
                        log::info!("Reading {length} bytes from modem");
                        while length > 0 {
                            log::debug!("remaining read: {}", length);
                            let mut buf = Vec::new();
                            buf.resize_default(usize::min(length, 365)).unwrap();
                            self.reader.read_exact(&mut buf).await?;
                            length -= buf.len();
                            log::info!(
                                "Sending {} bytes to tcp connection {connection}",
                                buf.len()
                            );
                            self.tcp.slots[connection].peek().rx.send(buf).await;
                            log::info!("Bytes sent to tcp connection {connection}");
                        }
                        log::info!("Done sending to tcp connection {connection}");
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
                    _ => log::warn!("Unhandled URC: {message:?}"),
                }
            } else if let Ok(response) = ResponseCode::from_line(&line) {
                // If it's not a URC, try to parse it as a regular response code

                log::info!("Got generic response: {line:?}");
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
                log::error!("Got unknown response: {line:?}");
            }

            Ok(())
        }
    }
}

pub struct TxPump<'context, W> {
    pub(crate) writer: W,
    pub(crate) commands: Receiver<'context, CriticalSectionRawMutex, RawAtCommand, 4>,
}

impl<'context, W: Write> Pump for TxPump<'context, W> {
    type Err = W::Error;
    type Fut<'a> = impl Future<Output = Result<(), Self::Err>> + 'a
    where
        Self: 'a;

    fn pump(&mut self) -> Self::Fut<'_> {
        async {
            let command = self.commands.recv().await;
            match &command {
                RawAtCommand::Text(text) => log::info!("Write to modem: {text:?}"),
                RawAtCommand::Binary(bytes) => log::info!("Write {} bytes to modem", bytes.len()),
            }
            self.writer.write_all(command.as_bytes()).await?;

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
                    log::error!("Error pumping {} {:?}", stringify!($name), err);
                }
            }
        }
    };
}
