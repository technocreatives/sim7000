use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtWrite, ConnectionState, Decoder, Encoder};

pub struct Cipstart;

impl AtCommand for Cipstart {
    const COMMAND: &'static str = "AT+CIPSTART";
}

pub struct TcpConnectionParams {
    pub mode: &'static str,
    pub host: &'static str,
    pub port: u16,
}

impl AtEncode for TcpConnectionParams {
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
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("OK", timeout)?;
        decoder.end_line();

        let status = match decoder.remainder_str(timeout)? {
            "CONNECT OK" => ConnectionResult::Success,
            "ALREADY CONNECT" => ConnectionResult::Success,
            _ => {
                decoder.expect_str("STATE: ", timeout)?;
                let _ = ConnectionState::try_from(decoder.remainder_str(timeout)?)
                    .map_err(|_| crate::Error::DecodingFailed)?;
                decoder.end_line();
                decoder.expect_str("CONNECT FAIL", timeout)?;
                ConnectionResult::Failure
            }
        };

        Ok(status)
    }
}

impl AtWrite<'_> for Cipstart {
    type Input = TcpConnectionParams;
    type Output = ConnectionResult;
}
