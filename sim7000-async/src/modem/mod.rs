mod context;

use embassy::{blocking_mutex::raw::CriticalSectionRawMutex, channel::{Sender, Receiver, Signal}, time::{Duration, Timer}, mutex::Mutex};
use embedded_hal::digital::blocking::OutputPin;
use heapless::{Vec, String};
use core::fmt::Write as FmtWrite;

use crate::{read::{Read, ModemReader}, Error, ModemPower, PowerState, write::Write, RegistrationStatus, single_arc::SingletonArcGuard, tcp::{TcpStream, TcpMessage}};
pub use context::*;
pub struct Modem<'c, P, W> {
    context: &'c ModemContext<W>,
    power: P,
    tx: SingletonArcGuard<'c, Mutex<CriticalSectionRawMutex, W>>,
}

impl<'c, P: ModemPower, W: Write> Modem<'c, P, W> {
    pub async fn new<R: Read>(
        rx: R,
        tx: W,
        power: P,
        context: &'c ModemContext<W>,
    ) -> Result<(Modem<'c, P, W>, RxPump<'c, R>), Error<W::Error>> {
        let tx = context.transmit.get_or_init(move || Mutex::new(tx));
        
        let modem = Modem { context, power, tx };

        let pump = RxPump {
            reader: ModemReader::new(rx),
            generic_response: context.generic_response.sender(),
            tcp: &context.tcp,
            registration_events: &context.registration_events,
        };

        Ok((modem, pump))
    }

    pub async fn init(&mut self) -> Result<(), Error<W::Error>> {
        self.power.disable().await;
        self.power.enable().await;

        for _ in 0..5 {
            match embassy::time::with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+IFC=2,2\r").await
            }).await {
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
                _ => {Timer::after(Duration::from_millis(200 as u64)).await}
            }
        }
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;


        self.power.disable().await;
        Ok(())
    }

    pub async fn activate(&mut self) -> Result<(), Error<W::Error>> {
        self.power.enable().await;
        for _ in 0..5 {
            match embassy::time::with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+IFC=2,2\r").await
            }).await {
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

    async fn wait_for_registration(&mut self) -> Result<(), Error<W::Error>> {
        loop {
            match embassy::time::with_timeout(Duration::from_millis(2000), async {
                self.run_raw_command("AT+CGREG?\r").await
            }).await {
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

    async fn authenticate(&mut self) -> Result<(), Error<W::Error>> {
        self.run_raw_command("AT+CSTT=\"iot.1nce.net\",\"\",\"\"\r").await?;
        self.run_raw_command("AT+CIICR\r").await?;

        Ok(())
    }

    pub async fn run_raw_command(&self, command: &str) -> Result<Vec<String<32>, 4>, Error<W::Error>> {
        log::info!("Sending command {}", command);
        let mut tx = self.tx.lock().await;
        tx.write_all(command.as_bytes()).await?;
        tx.flush().await?;

        let mut responses = Vec::new();
        loop {
            match self.context.generic_response.recv().await.as_str() {
                "OK" | "SHUT OK" => break,
                "ERROR" => return Err(Error::SimError),
                res if res.starts_with("+CME ERROR") => return Err(Error::SimError),
                res => {responses.push(res.into());}
            }
        }
        drop(tx);
        Ok(responses)
    }

    pub async fn connect_tcp(&mut self, host: &str, port: u16) -> TcpStream<'c, W> {
        let tcp_context = self.context.tcp.claim().unwrap();
        self.tx.lock().await.write_all(b"AT+CIFSR\r").await.unwrap();

        let mut buf = heapless::String::<256>::new();
        write!(buf, "AT+CIPSTART={},\"TCP\",\"{}\",\"{}\"\r", tcp_context.ordinal(), host, port).unwrap();
        self.run_raw_command(buf.as_str()).await.unwrap();
        
        loop {
            match tcp_context.events().recv().await {
                crate::tcp::TcpMessage::Connected => break,
                crate::tcp::TcpMessage::ConnectionFailed => panic!(),
                _ => {}
            }
        }

        TcpStream {
            token: tcp_context,
            tx: self.tx.clone(),
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
    pub async fn pump(&mut self) -> Result<(), Error<R::Error>> {
        let line = self.reader.read_line().await?;
        log::info!("Sending response line {}", line);

        if line.starts_with("+CGREG:") {
            let stat = match line.split(&[' ', ',']).nth(2).unwrap().parse::<i32>().unwrap() {
                1 => RegistrationStatus::RegisteredHome,
                2 => RegistrationStatus::Searching,
                3 => RegistrationStatus::RegistrationDenied,
                4 => RegistrationStatus::Unknown,
                5 => RegistrationStatus::RegisteredRoaming,
                _ => RegistrationStatus::NotRegistered,
            };
            self.registration_events.signal(stat);
        } else if line.starts_with("+RECEIVE,") {
            let mut length = line.split(&[',', ':']).nth(2).unwrap().parse::<usize>().unwrap();
            let connection = line.split(&[',', ':']).nth(1).unwrap().parse::<usize>().unwrap();

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
        } else if line.ends_with(", SEND OK") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::SendSuccess).await;
        } else if line.ends_with(", SEND FAIL") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::SendFail).await;
        } else if line.ends_with(", CONNECT OK") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::Connected).await;
        } else if line.ends_with(", CONNECT FAIL") {
            let connection = line.split(&[',']).nth(0).unwrap().parse::<usize>().unwrap();
            self.tcp.events[connection].send(TcpMessage::ConnectionFailed).await;
        }
        else {
            match self.generic_response.try_send(line) {
                Ok(_) => {},
                Err(_) => log::info!("message queue full"),
            }
        }
        Ok(())
    }
}

pub struct RegistrationHandler<'context> {
    context: &'context Signal<RegistrationStatus>,
}

impl<'context> RegistrationHandler<'context> {
    pub async fn pump(&mut self) {
        match self.context.wait().await {
            RegistrationStatus::NotRegistered | RegistrationStatus::Searching | RegistrationStatus::RegistrationDenied | RegistrationStatus::Unknown => todo!(),
            RegistrationStatus::RegisteredHome => todo!(),
            RegistrationStatus::RegisteredRoaming => todo!(),
        }
    }
}