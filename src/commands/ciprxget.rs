use embedded_time::duration::Milliseconds;

use crate::{drain_relay, Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtWrite, Decoder, Encoder};

pub struct Ciprxget;

impl AtCommand for Ciprxget {
    const COMMAND: &'static str = "AT+CIPRXGET";
}

#[repr(i32)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetworkReceiveMode {
    Disable,
    Enable,
    GetBytes(u16),
    GetHex(u16),
    QueryUnread,
}

pub struct NetworkReceiveResponse {
    pub mode: NetworkReceiveMode,
    pub bytes: Option<heapless::Vec<u8, 1460>>,
}

// This decode impl can only be called when mode is 2 or greater.
impl AtDecode for NetworkReceiveResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        // The datasheet makes it look like there can only be one +CIPRXGET response to an AT+CIPRXGET command,
        // but this is not the case. The sim7000 can respond with +CIPRXGET: 1 to indicate its still waiting to
        // receive data. +CIPRXGET: 2,... will come on a later line
        let recv_len = loop {
            decoder.expect_str("+CIPRXGET: ", timeout)?;

            let mode = decoder.decode_scalar(timeout)?;
            match mode {
                1 => {
                    decoder.end_line();
                }
                2 => {
                    decoder.expect_str(",", timeout)?;
                    // According to the specification the amount to read should actually be the 3rd number.
                    // But the chip does not follow specification. The third number seems to be the amount
                    // remaining in the buffer, but not the amount that it will actually respond with on the
                    // UART.
                    break decoder.decode_scalar(timeout)?;
                }
                _ => return Err(crate::Error::DecodingFailed),
            }
        };

        let mut buffer = heapless::Vec::new();
        buffer
            .resize(recv_len as usize, 0)
            .map_err(|_| Error::BufferOverflow)?;

        if recv_len > 0 {
            decoder.read_exact(&mut buffer, timeout)?;
        }

        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(NetworkReceiveResponse {
            mode: NetworkReceiveMode::GetBytes(recv_len as u16),
            bytes: Some(buffer),
        })
    }
}

impl AtEncode for NetworkReceiveMode {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        match self {
            NetworkReceiveMode::Disable => {
                encoder.encode_scalar(0)?;
            }
            NetworkReceiveMode::Enable => {
                encoder.encode_scalar(1)?;
            }
            NetworkReceiveMode::GetBytes(req_len) => {
                encoder.encode_scalar(2)?;
                encoder.encode_str(",")?;
                encoder.encode_scalar(*req_len as i32)?;
            }
            NetworkReceiveMode::GetHex(req_len) => {
                encoder.encode_scalar(3)?;
                encoder.encode_str(",")?;
                encoder.encode_scalar(*req_len as i32)?;
            }
            NetworkReceiveMode::QueryUnread => {
                encoder.encode_scalar(4)?;
            }
        }

        Ok(())
    }
}

impl AtWrite<'_> for Ciprxget {
    type Input = NetworkReceiveMode;
    type Output = NetworkReceiveResponse;

    fn write<B: SerialReadTimeout + SerialWrite>(
        &self,
        parameter: Self::Input,
        serial: &mut B,
        timeout: Milliseconds,
    ) -> Result<Self::Output, Error<B::SerialError>> {
        drain_relay(serial, Milliseconds(0))?;

        let mut encoder = Encoder::new(serial);
        encoder.encode_str(<Self as AtCommand>::COMMAND)?;
        encoder.encode_str("=")?;

        parameter.encode(&mut encoder)?;
        serial.write(b"\r")?;

        let mut decoder = Decoder::new(serial);

        if parameter == NetworkReceiveMode::Disable || parameter == NetworkReceiveMode::Enable {
            <() as AtDecode>::decode(&mut decoder, timeout)?;
            return Ok(NetworkReceiveResponse {
                mode: parameter,
                bytes: None,
            });
        }
        Self::Output::decode(&mut decoder, timeout)
    }
}

#[cfg(test)]
mod test {
    use embedded_time::duration::Milliseconds;

    use crate::{commands::AtWrite, test::MockSerial};

    use super::{Ciprxget, NetworkReceiveMode};

    #[test]
    fn test_rx_ready() {
        let mut mock = MockSerial::build()
            .expect_write(b"AT+CIPRXGET=2,4\r")
            .expect_read(b"\r\n+CIPRXGET: 2,4,4\r\n1234")
            .expect_read(b"\r\nOK\r\n")
            .finalize();

        let response = Ciprxget
            .write(
                NetworkReceiveMode::GetBytes(4),
                &mut mock,
                Milliseconds(1000),
            )
            .unwrap();

        assert_eq!(response.bytes.unwrap(), b"1234")
    }

    #[test]
    fn test_rx_wait() {
        let mut mock = MockSerial::build()
            .expect_write(b"AT+CIPRXGET=2,4\r")
            .expect_read(b"\r\n+CIPRXGET: 1\r\n")
            .expect_read(b"\r\n+CIPRXGET: 1\r\n")
            .expect_read(b"\r\n+CIPRXGET: 2,4,4\r\n")
            .expect_read(&[0, 1, 2, 3])
            .expect_read(b"\r\nOK\r\n")
            .finalize();

        let response = Ciprxget
            .write(
                NetworkReceiveMode::GetBytes(4),
                &mut mock,
                Milliseconds(1000),
            )
            .unwrap();

        assert_eq!(response.bytes.unwrap(), &[0, 1, 2, 3])
    }
}
