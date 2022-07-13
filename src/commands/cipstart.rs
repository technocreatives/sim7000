use crate::{Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtWrite, ConnectionState, Decoder, Encoder};

pub struct Cipstart;

impl AtCommand for Cipstart {
    const COMMAND: &'static str = "AT+CIPSTART";
}

pub struct TcpConnectionParams<'a> {
    pub mode: &'static str,
    pub host: &'a str,
    pub port: u16,
}

impl<'a> AtEncode for TcpConnectionParams<'a> {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_str("\"")?;
        encoder.encode_str(self.mode)?;
        encoder.encode_str("\",\"")?;
        encoder.encode_str(self.host)?;
        encoder.encode_str("\",\"")?;
        encoder.encode_scalar(self.port as i32)?;
        encoder.encode_str("\"")
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConnectionResult {
    Failure,
    Success,
}

impl AtDecode for ConnectionResult {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout_ms)?;
        decoder.end_line();

        let status = match decoder.remainder_str(timeout_ms)? {
            "CONNECT OK" => ConnectionResult::Success,
            "ALREADY CONNECT" => ConnectionResult::Success,
            _ => {
                decoder.expect_str("STATE: ", timeout_ms)?;
                let _ = ConnectionState::try_from(decoder.remainder_str(timeout_ms)?)
                    .map_err(|_| crate::Error::DecodingFailed)?;
                decoder.end_line();
                decoder.expect_str("CONNECT FAIL", timeout_ms)?;
                ConnectionResult::Failure
            }
        };

        Ok(status)
    }
}

impl<'a> AtWrite<'a> for Cipstart {
    type Input = TcpConnectionParams<'a>;
    type Output = ConnectionResult;
}
