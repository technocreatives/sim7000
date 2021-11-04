use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute, Decoder};

pub struct Cipshut;

impl AtCommand for Cipshut {
    const COMMAND: &'static str = "AT+CIPSHUT";
}

#[derive(Clone, Copy)]
pub enum DisconnectResult {
    Failure,
    Success,
}

impl AtDecode for DisconnectResult {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        let status = match decoder.remainder_str(timeout)? {
            "SHUT OK" => DisconnectResult::Success,
            "ERROR" => DisconnectResult::Failure,
            _ => return Err(crate::Error::DecodingFailed),
        };

        Ok(status)
    }
}

impl AtExecute for Cipshut {
    type Output = DisconnectResult;
}
