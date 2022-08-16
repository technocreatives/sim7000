#![allow(dead_code)]

use crate::Modem;
use core::future::Future;
use core::str::{from_utf8, Utf8Error};
use embassy_executor::executor::{SpawnError, Spawner};
use embassy_util::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_util::channel::mpmc::Channel;
use heapless::Vec;
use sim7000_async::{read::Read, tcp::TcpStream, write::Write};

#[derive(Debug)]
pub enum Error {
    Spawn(SpawnError),
    Sim(sim7000_async::Error),
    Tcp(sim7000_async::tcp::TcpError),
    Utf8(Utf8Error),
}

type TaskResponseChannel<T> = Channel<CriticalSectionRawMutex, Result<T, Error>, 1>;

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
                    log::info!("Sending Marco");
                    const MARCO: &str = "\nFOOBARBAZBOPSHOP\n";
                    stream.write_all(MARCO.as_bytes()).await?;

                    log::info!("Reading Polo");
                    let mut buf = [0u8; MARCO.len()];

                    stream.read_exact(&mut buf).await?;

                    let polo = from_utf8(&buf)?;

                    log::info!(r#"Got response {polo:?}"#,);

                    stream.close().await;

                    log::info!("ping_tcpbin done");
                    Ok(())
                }
                .await,
            )
            .await
    }

    log::info!("Connecting to tcpbin.com");
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
                        log::info!("QotD call read");
                        let mut tmp = [0u8; 128];
                        let n = stream.read(&mut tmp).await?;
                        log::info!("QotD read {n} bytes");
                        if n == 0 {
                            break;
                        }

                        if buf.extend_from_slice(&tmp[..n]).is_err() {
                            log::warn!("QotD buffer full");
                            break;
                        }
                    }

                    let quote = from_utf8(&buf)?;

                    log::info!("Quote of the Day:\r\n{quote}");

                    stream.close().await;

                    log::info!("QotD done");

                    Ok(())
                }
                .await,
            )
            .await
    }

    log::info!("Getting Quote of the Day");
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
