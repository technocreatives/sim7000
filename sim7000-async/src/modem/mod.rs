mod command;
mod context;

use core::fmt::Write as FmtWrite;
use embassy_executor::time::{with_timeout, Duration, Timer};
use embassy_util::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::mpmc::{Receiver, Sender},
    channel::signal::Signal,
};
use heapless::{String, Vec};

use crate::{
    read::{ModemReader, Read},
    tcp::{TcpMessage, TcpStream},
    write::Write,
    Error, ModemPower, RegistrationStatus,
};
pub use command::Command;
pub use context::*;

pub struct Uninitialized;
pub struct Disabled;
pub struct Enabled;
pub struct Sleeping;

pub struct Modem<'c, P> {
    context: &'c ModemContext,
    power: P,
}

impl<'c, P: ModemPower> Modem<'c, P> {
    pub async fn new<R: Read, W: Write>(
        rx: R,
        tx: W,
        power: P,
        context: &'c ModemContext,
    ) -> Result<(Modem<'c, P>, TxPump<'c, W>, RxPump<'c, R>), Error> {
        let modem = Modem { context, power };

        let rx_pump = RxPump {
            reader: ModemReader::new(rx),
            generic_response: context.generic_response.sender(),
            tcp: &context.tcp,
            registration_events: &context.registration_events,
        };

        let tx_pump = TxPump {
            writer: tx,
            commands: context.commands.receiver(),
        };

        Ok((modem, tx_pump, rx_pump))
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.power.disable().await;
        self.power.enable().await;

        for _ in 0..5 {
            match with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+IFC=2,2\r").await
            })
            .await
            {
                Ok(Ok(_)) => break,
                _ => {}
            }
        }
        self.run_raw_command("AT+CSCLK=0\r").await?;
        self.run_raw_command("AT\r").await?;
        self.run_raw_command("AT+IPR=115200\r").await?;
        self.run_raw_command("AT+IFC=2,2\r").await?;
        self.run_raw_command("AT+CMEE=1\r").await?;
        self.run_raw_command("AT+CNMP=38\r").await?;
        self.run_raw_command("AT+CMNB=1\r").await?;
        self.run_raw_command("AT+CFGRI=1\r").await?;
        for _ in 0..5 {
            match self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await {
                Ok(_) => break,
                _ => Timer::after(Duration::from_millis(200 as u64)).await,
            }
        }
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;

        self.power.disable().await;
        Ok(())
    }

    pub async fn activate(&mut self) -> Result<(), Error> {
        self.power.enable().await;
        for _ in 0..5 {
            match with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+IFC=2,2\r").await
            })
            .await
            {
                Ok(Ok(_)) => break,
                _ => {}
            }
        }
        self.run_raw_command("ATE0\r").await?;

        self.run_raw_command("AT+CGREG=2\r").await?;
        self.wait_for_registration().await?;
        self.run_raw_command("AT+CIPMUX=1\r").await?;
        //self.run_raw_command("AT+CIPSPRT=0\r").await?;
        self.run_raw_command("AT+CIPSHUT\r").await.unwrap();

        self.authenticate().await?;
        Ok(())
    }

    async fn wait_for_registration(&mut self) -> Result<(), Error> {
        loop {
            match with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+CGREG?\r").await
            })
            .await
            {
                Err(_) => continue,
                _ => {}
            }
            match self.context.registration_events.wait().await {
                RegistrationStatus::RegisteredHome | RegistrationStatus::RegisteredRoaming => {
                    break;
                }
                _ => Timer::after(Duration::from_millis(200 as u64)).await,
            }
        }

        Ok(())
    }

    async fn authenticate(&mut self) -> Result<(), Error> {
        self.run_raw_command("AT+CSTT=\"iot.1nce.net\",\"\",\"\"\r")
            .await?;
        self.run_raw_command("AT+CIICR\r").await?;

        Ok(())
    }

    pub async fn run_raw_command(&self, command: &str) -> Result<Vec<String<32>, 4>, Error> {
        log::info!("Sending command {}", command);
        let lock = self.context.command_mutex.lock().await;
        self.context.commands.send(command.into()).await;

        let mut responses = Vec::new();
        loop {
            match self.context.generic_response.recv().await.as_str() {
                "OK" | "SHUT OK" => break,
                "ERROR" => return Err(Error::SimError),
                res if res.starts_with("+CME ERROR") => return Err(Error::SimError),
                res => {
                    responses
                        .push(res.into())
                        .map_err(|_| Error::BufferOverflow)?;
                }
            }
        }
        drop(lock);
        Ok(responses)
    }

    pub async fn connect_tcp(&mut self, host: &str, port: u16) -> TcpStream<'c> {
        let tcp_context = self.context.tcp.claim().unwrap();
        self.context.command_mutex.lock().await;
        self.context.commands.send("AT+CIFSR\r".into()).await;

        let mut buf = heapless::String::<256>::new();
        write!(
            buf,
            "AT+CIPSTART={},\"TCP\",\"{}\",\"{}\"\r",
            tcp_context.ordinal(),
            host,
            port
        )
        .unwrap();
        self.run_raw_command(buf.as_str()).await.unwrap(); // TODO: remove unwrap

        loop {
            match tcp_context.events().recv().await {
                crate::tcp::TcpMessage::Connected => break,
                crate::tcp::TcpMessage::ConnectionFailed => panic!(),
                _ => {}
            }
        }

        TcpStream {
            token: tcp_context,
            command_mutex: &self.context.command_mutex,
            commands: &self.context.commands,
            generic_response: &self.context.generic_response,
            closed: false,
            buffer: Vec::new(),
        }
    }
}

pub struct RxPump<'context, R> {
    reader: ModemReader<R>,
    generic_response: Sender<'context, CriticalSectionRawMutex, heapless::String<256>, 1>,
    tcp: &'context TcpContext,
    registration_events: &'context Signal<RegistrationStatus>,
}

impl<'context, R: Read> RxPump<'context, R> {
    pub async fn pump(&mut self) -> Result<(), Error> {
        let line = self.reader.read_line().await?;
        log::info!("Sending response line {}", line);

        if line.starts_with("+CGREG:") {
            let stat = match line
                .split(&[' ', ','])
                .nth(2)
                .unwrap()
                .parse::<i32>()
                .unwrap()
            {
                1 => RegistrationStatus::RegisteredHome,
                2 => RegistrationStatus::Searching,
                3 => RegistrationStatus::RegistrationDenied,
                4 => RegistrationStatus::Unknown,
                5 => RegistrationStatus::RegisteredRoaming,
                _ => RegistrationStatus::NotRegistered,
            };
            self.registration_events.signal(stat);
        } else if line.starts_with("+RECEIVE,") {
            let mut length = line
                .split(&[',', ':'])
                .nth(2)
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let connection = line
                .split(&[',', ':'])
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .unwrap();

            while length > 0 {
                log::debug!("remaining read: {}", length);
                let mut buf = Vec::new();
                buf.resize_default(usize::min(length, 365)).unwrap();
                self.reader.read_exact(&mut buf).await?;
                length -= buf.len();
                self.tcp.rx[connection].send(buf).await;
            }
        } else if line.ends_with(", CLOSED") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::Closed).await;
        } else if line.ends_with(", CLOSE OK") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::Closed).await;
        } else if line.ends_with(", SEND OK") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection]
                .send(TcpMessage::SendSuccess)
                .await;
        } else if line.ends_with(", SEND FAIL") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::SendFail).await;
        } else if line.ends_with(", CONNECT OK") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection]
                .send(TcpMessage::Connected)
                .await;
        } else if line.ends_with(", CONNECT FAIL") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection]
                .send(TcpMessage::ConnectionFailed)
                .await;
        } else {
            match self.generic_response.try_send(line) {
                Ok(_) => {}
                Err(_) => log::info!("message queue full"),
            }
        }
        Ok(())
    }
}

pub struct TxPump<'context, W> {
    writer: W,
    commands: Receiver<'context, CriticalSectionRawMutex, Command, 4>,
}

impl<'context, W: Write> TxPump<'context, W> {
    pub async fn pump(&mut self) -> Result<(), W::Error> {
        let command = self.commands.recv().await;
        self.writer.write_all(command.as_bytes()).await?;

        Ok(())
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
