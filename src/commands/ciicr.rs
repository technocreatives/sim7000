use super::{AtCommand, AtDecode, AtExecute, Decoder};
use crate::{Error, SerialReadTimeout};

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
        timeout_ms: u32,
    ) -> Result<Self, Error<B::SerialError>> {
        Ok(match decoder.remainder_str(timeout_ms)? {
            "OK" => GprsResult::Success,
            "ERROR" => GprsResult::Failure,
            _ => return Err(crate::Error::DecodingFailed),
        })
    }
}

impl AtExecute for Ciicr {
    type Output = GprsResult;
}
