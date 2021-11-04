use super::{AtCommand, AtDecode, AtExecute, Decoder};
use crate::{Error, SerialReadTimeout};
use embedded_time::duration::Milliseconds;

pub struct Ciicr;

impl AtCommand for Ciicr {
    const COMMAND: &'static str = "AT+CIICR";
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GprsResult {
    Failure,
    Success,
}

impl AtDecode for GprsResult {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        Ok(match decoder.remainder_str(timeout)? {
            "OK" => GprsResult::Success,
            "ERROR" => GprsResult::Failure,
            _ => return Err(crate::Error::DecodingFailed),
        })
    }
}

impl AtExecute for Ciicr {
    type Output = GprsResult;
}
