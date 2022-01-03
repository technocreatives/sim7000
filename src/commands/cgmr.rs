use crate::{Error, SerialReadTimeout};

use super::{AtCommand, AtDecode, AtExecute, Decoder};
use embedded_time::duration::Milliseconds;

pub struct Cgmr;

impl AtCommand for Cgmr {
    const COMMAND: &'static str = "AT+CGMR";
}

impl AtExecute for Cgmr {
    type Output = CgmrResponse;
}

#[derive(Debug, Clone)]
pub struct CgmrResponse {
    pub firmware: heapless::String<256>,
}

impl AtDecode for CgmrResponse {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("Revision:", timeout)?;

        let result = CgmrResponse {
            firmware: decoder.remainder_str(timeout)?.into(),
        };

        decoder.end_line();

        decoder.expect_str("OK", timeout)?;

        Ok(result)
    }
}
