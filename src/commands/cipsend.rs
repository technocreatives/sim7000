use embedded_time::duration::Milliseconds;
use crate::{Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtWrite, Decoder, Encoder};

pub struct Cipsend;

impl AtCommand for Cipsend {
    const COMMAND: &'static str = "AT+CIPSEND";
}

#[derive(Clone, Copy)]
pub enum SendResult {
    Failure,
    Success,
}

impl AtDecode for SendResult {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        let status = match decoder.remainder_str(timeout)? {
            "SEND OK" => SendResult::Success,
            "SEND FAIL" => SendResult::Failure,
            _ => return Err(crate::Error::DecodingFailed),
        };

        Ok(status)
    }
}

impl<'a> AtWrite<'a> for Cipsend {
    type Input = &'a [u8];
    type Output = SendResult;

    fn write<
        B: SerialReadTimeout + SerialWrite,
    >(
        &self,
        parameter: Self::Input,
        serial: &mut B,
        timeout: Milliseconds,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        crate::drain_relay(serial, Milliseconds(0))?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("=")?;

        encoder.encode_scalar(parameter.len() as i32)?;

        // Wait 200ms for an echo to appear.
        let echoed = crate::drain_relay(serial, Milliseconds(200))?;

        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        // Drain the echoed newline
        if echoed {
            decoder.expect_empty(timeout)?;
            decoder.end_line();
        }

        // Drain the newline that starts every command
        decoder.expect_empty(timeout)?;
        decoder.end_line();

        let mut buf = [0u8; 2];
        serial
            .read_exact(&mut buf[..2], timeout)?
            .ok_or(crate::Error::Timeout)?;

        if buf != *b"> " {
            return Err(crate::Error::DecodingFailed);
        }

        let mut encoder = Encoder::new(serial);
        parameter.encode(&mut encoder)?;

        // Pray to god ECHO is disabled, there is no way to handle it here.
        let mut decoder = Decoder::new(serial);

        decoder.expect_empty(timeout)?;
        decoder.end_line();

        Self::Output::decode(&mut decoder, timeout)
    }
}
