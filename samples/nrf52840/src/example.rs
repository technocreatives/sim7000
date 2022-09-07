#![allow(dead_code)]

use crate::Modem;
use core::future::Future;
use core::str::{from_utf8, Utf8Error};
use cortex_m::prelude::_embedded_hal_blocking_i2c_Read;
use embassy_executor::{SpawnError, Spawner};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embedded_io::{
    asynch::{Read, Write},
    blocking::ReadExactError,
};
use heapless::Vec;
use sim7000_async::{
    gnss::Gnss,
    tcp::{TcpError, TcpStream},
    voltage::VoltageWarner,
};

#[derive(Debug)]
pub enum Error {
    Spawn(SpawnError),
    Sim(sim7000_async::Error),
    Tcp(sim7000_async::tcp::TcpError),
    Utf8(Utf8Error),
}

type TaskResponseChannel<T> = Channel<CriticalSectionRawMutex, Result<T, Error>, 1>;

#[embassy_executor::task]
pub async fn voltage_warn(warner: VoltageWarner<'static>) {
    loop {
        let warning = warner.warning().await;
        defmt::warn!("Got voltage warning: {:?}", warning);
    }
}

#[embassy_executor::task]
pub async fn gnss(gnss: Gnss<'static>) {
    loop {
        let report = gnss.get_report().await;
        defmt::info!("GNSS report: {:?}", report);
    }
}

pub async fn ping_tcpbin(
    spawner: &Spawner,
    modem: &mut Modem,
) -> Result<impl Future<Output = Result<(), Error>>, Error> {
    static TASK_CHANNEL: TaskResponseChannel<()> = Channel::new();

    #[embassy_executor::task]
    async fn task(mut stream: TcpStream<'static>) {
        TASK_CHANNEL
            .send(
                async move {
                    defmt::info!("Sending Marco");
                    const MARCO: &str = "\nFOOBARBAZBOPSHOP\n";
                    stream.write_all(MARCO.as_bytes()).await?;

                    defmt::info!("Reading Polo");
                    let mut buf = [0u8; MARCO.len()];

                    stream.read_exact(&mut buf).await.map_err(|err| match err {
                        ReadExactError::Other(err) => err,
                        ReadExactError::UnexpectedEof => TcpError::Closed,
                    })?;

                    let polo = from_utf8(&buf)?;

                    defmt::info!(r#"Got response {:?}"#, polo);

                    Ok(())
                }
                .await,
            )
            .await
    }

    defmt::info!("Connecting to tcpbin.com");
    let stream = modem.connect_tcp("tcpbin.com", 4242).await?;

    spawner.spawn(task(stream))?;
    Ok(TASK_CHANNEL.recv())
}

pub async fn get_quote_of_the_day(
    spawner: &Spawner,
    modem: &mut Modem,
) -> Result<impl Future<Output = Result<(), Error>>, Error> {
    static TASK_CHANNEL: TaskResponseChannel<()> = Channel::new();

    #[embassy_executor::task]
    async fn task(mut stream: TcpStream<'static>) {
        TASK_CHANNEL
            .send(
                async move {
                    let mut buf = Vec::<u8, 1024>::new();

                    loop {
                        defmt::info!("QotD call read");
                        let mut tmp = [0u8; 128];
                        let n = stream.read(&mut tmp).await?;
                        defmt::info!("QotD read {} bytes", n);
                        if n == 0 {
                            break;
                        }

                        if buf.extend_from_slice(&tmp[..n]).is_err() {
                            defmt::warn!("QotD buffer full");
                            break;
                        }
                    }

                    let quote = from_utf8(&buf)?;

                    defmt::info!("Quote of the Day:\r\n{}", quote);

                    Ok(())
                }
                .await,
            )
            .await
    }

    defmt::info!("Getting Quote of the Day");
    let stream = modem.connect_tcp("djxmmx.net", 17).await?;

    spawner.spawn(task(stream))?;
    return Ok(TASK_CHANNEL.recv());
}

impl From<SpawnError> for Error {
    fn from(e: SpawnError) -> Self {
        Error::Spawn(e)
    }
}

impl From<sim7000_async::Error> for Error {
    fn from(e: sim7000_async::Error) -> Self {
        Error::Sim(e)
    }
}

impl From<sim7000_async::tcp::TcpError> for Error {
    fn from(e: sim7000_async::tcp::TcpError) -> Self {
        Error::Tcp(e)
    }
}

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl defmt::Format for Error {
    fn format(&self, f: defmt::Formatter) {
        use defmt::write;

        // Format as hexadecimal.
        match self {
            Error::Spawn(_) => write!(f, "SpawnError"),
            Error::Sim(e) => write!(f, "Sim({:?})", e),
            Error::Tcp(e) => write!(f, "Tcp({:?})", e),
            Error::Utf8(_) => write!(f, "Utf8Error"),
        }
    }
}
