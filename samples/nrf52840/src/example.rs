use core::str::from_utf8;
use heapless::Vec;
use sim7000_async::{modem::Modem, read::Read, write::Write, ModemPower};

pub async fn ping_tcpbin<P: ModemPower>(modem: &mut Modem<'_, P>) -> Result<(), ()> {
    log::info!("Connecting to tcpbin.com");
    let mut stream = modem.connect_tcp("tcpbin.com", 4242).await;

    log::info!("Sending marco");
    let marco = "\r\nFOOBARBAZBOPSHOP\r\n";
    stream
        .write_all(marco.as_bytes())
        .await
        .expect("Failed to write to tcp stream");

    log::info!("Reading polo");
    let mut buf = [0u8; 128];

    let n = stream
        .read(&mut buf)
        .await
        .expect("Failed to read from tcp stream");

    let polo = match from_utf8(&buf[..n]) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Response was not utf8: {e}");
            return Err(());
        }
    };

    log::info!(r#"Got response "{polo}""#,);

    stream.close().await;
    Ok(())
}

pub async fn get_quote_of_the_day<P: ModemPower>(modem: &mut Modem<'_, P>) -> Result<(), ()> {
    log::info!("Getting Quote of the Day");
    let mut stream = modem.connect_tcp("djxmmx.net", 17).await;
    let mut buf = Vec::<u8, 1024>::new();
    loop {
        let mut tmp = [0u8; 128];
        let n = stream
            .read(&mut tmp)
            .await
            .expect("Failed to read from tcp stream");
        if n == 0 {
            break;
        }

        if buf.extend_from_slice(&tmp[..n]).is_err() {
            log::error!("buffer full");
            return Err(());
        }
    }

    log::info!(
        r#"Quote of the Day:\r\n{:?}"#,
        core::str::from_utf8(&buf).unwrap()
    );

    stream.close().await;
    Ok(())
}
