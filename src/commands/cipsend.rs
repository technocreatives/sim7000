use crate::{Error, SerialReadTimeout, SerialWrite};
use embedded_time::duration::Milliseconds;

use super::{AtCommand, AtDecode, AtEncode, AtWrite, Decoder, Encoder};

pub struct Cipsend;

impl AtCommand for Cipsend {
    const COMMAND: &'static str = "AT+CIPSEND";
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    fn write<B: SerialReadTimeout + SerialWrite>(
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

        serial.write(b"\r")?;

        let mut buf = [0u8; 4];
        serial
            .read_exact(&mut buf[..4], timeout)?
            .ok_or(crate::Error::Timeout)?;

        if buf != *b"\r\n> " {
            return Err(crate::Error::DecodingFailed);
        }

        let mut encoder = Encoder::new(serial);
        parameter.encode(&mut encoder)?;

        // Pray to god ECHO is disabled, there is no way to handle it here.
        let mut decoder = Decoder::new(serial);

        Self::Output::decode(&mut decoder, timeout)
    }
}

#[cfg(test)]
mod test {
    use embedded_time::duration::Milliseconds;

    use crate::{
        commands::{AtWrite, SendResult},
        test::MockSerial,
    };

    use super::Cipsend;

    #[test]
    fn test_send_ok() {
        let data = b"hello, world!";
        let mut mock = MockSerial::build()
            .expect_write(format!("AT+CIPSEND={}\r", data.len()).as_bytes())
            .expect_read(b"\r\n> ")
            .expect_write(data)
            .expect_read(b"\r\nSEND OK\r\n")
            .finalize();

        let output = Cipsend.write(data, &mut mock, Milliseconds(1000)).unwrap();
        assert_eq!(output, SendResult::Success);
    }

    #[test]
    fn test_send_failed() {
        let data = b"hello, world!";
        let mut mock = MockSerial::build()
            .expect_write(format!("AT+CIPSEND={}\r", data.len()).as_bytes())
            .expect_read(b"\r\n> ")
            .expect_write(data)
            .expect_read(b"\r\nSEND FAIL\r\n")
            .finalize();

        let output = Cipsend.write(data, &mut mock, Milliseconds(1000)).unwrap();
        assert_eq!(output, SendResult::Failure);
    }
}
