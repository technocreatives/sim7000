use crate::Modem;
use core::mem::drop;
use core::str::from_utf8;
use embassy_executor::executor::{SpawnError, Spawner};
use heapless::Vec;
use sim7000_async::{read::Read, tcp::TcpStream, write::Write};

pub async fn spawn_ping_tcpbin(spawner: &Spawner, modem: &mut Modem) -> Result<(), SpawnError> {
    log::info!("Connecting to tcpbin.com");
    let stream = modem.connect_tcp("tcpbin.com", 4242).await.unwrap();
    spawner.spawn(ping_tcpbin(stream))
}

#[embassy_executor::task]
async fn ping_tcpbin(mut stream: TcpStream<'static>) {
    log::info!("Sending Marco");
    const MARCO: &str = "\nFOOBARBAZBOPSHOP\n";
    stream
        .write_all(MARCO.as_bytes())
        .await
        .expect("Failed to write to tcp stream");

    log::info!("Reading Polo");
    let mut buf = [0u8; MARCO.len()];

    if let Err(e) = stream.read_exact(&mut buf).await {
        return log::error!("Failed to read polo from stream: {e:?}");
    }

    let polo = match from_utf8(&buf) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Response was not utf8: {e}");
            return;
        }
    };

    log::info!(r#"Got response {polo:?}"#,);

    stream.close().await;
}

pub async fn get_quote_of_the_day(modem: &mut Modem) -> Result<(), ()> {
    log::info!("Getting Quote of the Day");
    let mut stream = modem.connect_tcp("djxmmx.net", 17).await.map_err(drop)?;
    let mut buf = Vec::<u8, 1024>::new();
    loop {
        let mut tmp = [0u8; 128];
        let n = stream
            .read(&mut tmp)
            .await
            .expect("Failed to read QotD from stream");
        if n == 0 {
            break;
        }

        if buf.extend_from_slice(&tmp[..n]).is_err() {
            log::error!("buffer full");
            return Err(());
        }
    }

    let quote = match from_utf8(&buf) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Response was not utf8: {e}");
            return Err(());
        }
    };

    log::info!("Quote of the Day:\r\n{quote}",);

    stream.close().await;
    Ok(())
}
