use embedded_time::duration::Milliseconds;

use crate::{SerialReadTimeout, Error, SerialWrite, drain_relay};

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

impl AtDecode for NetworkReceiveResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CIPRXGET: ", timeout)?;

        match decoder.decode_scalar(timeout)? {
            1 => Ok(NetworkReceiveResponse {
                mode: NetworkReceiveMode::Enable,
                bytes: None,
            }),
            2 => {
                decoder.expect_str(",", timeout)?;
                let req_len = decoder.decode_scalar(timeout)?;
                decoder.expect_str(",", timeout)?;
                let _read_bytes = decoder.decode_scalar(timeout)?;
                decoder.end_line();

                // according to the specification the amount read should actually be `read_bytes`.
                // But the chip does not follow specification. `read_bytes` is not actually used.
                let actual_read = req_len;
                let mut buffer = heapless::Vec::new();
                buffer
                    .resize(actual_read as usize, 0)
                    .map_err(|_| Error::BufferOverflow)?;

                if actual_read > 0 {
                    decoder.read_exact(&mut buffer, timeout)?;
                }

                decoder.end_line();
                decoder.expect_empty(timeout)?;

                decoder.end_line();
                decoder.expect_str("OK", timeout)?;

                Ok(NetworkReceiveResponse {
                    mode: NetworkReceiveMode::GetBytes(actual_read as u16),
                    bytes: Some(buffer),
                })
            }
            _ => Err(Error::DecodingFailed),
        }
    }
}

impl AtEncode for NetworkReceiveMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
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

        // Wait 200ms for an echo to appear.
        let echoed = drain_relay(serial, Milliseconds(200))?;

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
