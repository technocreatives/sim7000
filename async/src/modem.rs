use embassy::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Sender};
use embedded_hal::digital::blocking::OutputPin;
use heapless::{Vec, String};

use crate::{read::{Read, ModemReader}, ModemContext, Error, ModemPower, PowerState, write::Write};

pub struct Modem<'c, P, W> {
    context: &'c ModemContext,
    power: P,
    tx: W,
}

impl<'c, P: ModemPower, W: Write> Modem<'c, P, W> {
    pub async fn new<R: Read>(
        rx: R,
        tx: W,
        power: P,
        context: &'c ModemContext,
    ) -> (Modem<'c, P, W>, RxPump<'c, R>) {
        let mut modem = Modem { context, power, tx };

        let pump = RxPump {
            reader: ModemReader::new(rx),
            generic_response: context.generic_response.sender(),
            tcp_1_channel: context.tcp_1_channel.sender(),
        };
        
        modem.power.disable().await;

        (modem, pump)
    }

    pub async fn run_raw_command(&mut self, command: &str) -> Result<Vec<String<32>, 4>, Error<W::Error>> {
        self.tx.write_all(command.as_bytes()).await?;
        self.tx.flush().await?;

        let mut responses = Vec::new();
        loop {
            match self.context.generic_response.recv().await.as_str() {
                "OK" | "ERROR" => break,
                res if res.starts_with("+CME ERROR") => break,
                res => {responses.push(res.into());}
            }
        }
        Ok(responses)
    }
}

pub struct RxPump<'context, R> {
    reader: ModemReader<R>,
    generic_response: Sender<'context, CriticalSectionRawMutex, heapless::String<256>, 1>,
    tcp_1_channel: Sender<'context, CriticalSectionRawMutex, heapless::Vec<u8, 256>, 2>,
}

impl<'context, R: Read> RxPump<'context, R> {
    pub async fn pump(&mut self) -> Result<(), Error<R::Error>> {
        let line = self.reader.read_line().await?;

        self.generic_response.send(line).await;
        Ok(())
    }
}