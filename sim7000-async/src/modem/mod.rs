mod context;

use embassy::{blocking_mutex::raw::CriticalSectionRawMutex, channel::{Sender, Receiver, Signal}, time::{Duration, Timer}, mutex::Mutex};
use embedded_hal::digital::blocking::OutputPin;
use heapless::{Vec, String};
use core::fmt::Write as FmtWrite;

use crate::{read::{Read, ModemReader}, ModemContext, Error, ModemPower, PowerState, write::Write, RegistrationStatus, single_arc::SingletonArcGuard, tcp::TcpStream};

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
            tcp_1_channel: context.tcp_1_channel.sender(),
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
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;
        self.run_raw_command("AT+CEDRXS=1,4,\"0000\"\r").await?;
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
        self.run_raw_command("AT+CIPSPRT=0\r").await?;
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

    pub async fn run_raw_command(&mut self, command: &str) -> Result<Vec<String<32>, 4>, Error<W::Error>> {
        log::info!("Sending command {}", command);
        let mut tx = self.tx.lock().await;
        tx.write_all(command.as_bytes()).await?;
        tx.flush().await?;

        let mut responses = Vec::new();
        loop {
            match self.context.generic_response.recv().await.as_str() {
                "OK" | "SHUT OK" => break,
                res if res.starts_with("+CME ERROR") | "ERROR" => return Err(Error::SimError),
                res => {responses.push(res.into());}
            }
        }
        drop(tx);
        Ok(responses)
    }

    pub async fn connect_tcp(&mut self) -> TcpStream<'c, W> {
        self.tx.lock().await.write_all(b"AT+CIFSR\r").await.unwrap();
        self.run_raw_command("AT+CIPSTART=0,\"TCP\",\"example.com\",\"80\"\r").await.unwrap();
        loop {
            match self.context.generic_response.recv().await.as_str() {
                "0, CONNECT OK" => break,
                "0, CONNECT FAIL" => panic!(),
                _ => {}
            }
        }
        TcpStream {
            instance: 0,
            tx: self.tx.clone()
        }
    }
}

pub struct RxPump<'context, R> {
    reader: ModemReader<R>,
    generic_response: Sender<'context, CriticalSectionRawMutex, heapless::String<256>, 1>,
    tcp_1_channel: Sender<'context, CriticalSectionRawMutex, heapless::Vec<u8, 365>, 8>,
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
            while length > 0 {
                log::debug!("remaining read: {}", length);
                let mut buf = Vec::new();
                buf.resize_default(usize::min(length, 365)).unwrap();
                self.reader.read_exact(&mut buf).await?;
                length -= buf.len();
                self.tcp_1_channel.send(buf).await;
            }
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