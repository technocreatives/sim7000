use embedded_time::duration::Milliseconds;

use crate::{Error, SerialReadTimeout, SerialWrite};

use super::{AtCommand, AtDecode, AtEncode, AtRead, AtWrite, Decoder, Encoder};

pub struct Cgnspwr;

impl AtCommand for Cgnspwr {
    const COMMAND: &'static str = "AT+CGNSPWR";
}

#[repr(i32)]
#[derive(Clone, Copy)]
pub enum PowerStatus {
    Off = 0,
    On = 1,
}

impl AtDecode for PowerStatus {
    fn decode<B: SerialReadTimeout>(
        decoder: &mut Decoder<B>,
        timeout: Milliseconds,
    ) -> Result<Self, Error<B::SerialError>> {
        decoder.expect_str("+CGNSPWR: ", timeout)?;

        let mode = match decoder
            .remainder_str(timeout)?
            .parse::<i32>()
            .map_err(|_| crate::Error::DecodingFailed)?
        {
            0 => PowerStatus::Off,
            1 => PowerStatus::On,
            _ => return Err(crate::Error::DecodingFailed),
        };

        decoder.end_line();
        decoder.expect_str("OK", timeout)?;

        Ok(mode)
    }
}

impl AtEncode for PowerStatus {
    fn encode<B: SerialWrite>(
        &self,
        encoder: &mut Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtRead for Cgnspwr {
    type Output = PowerStatus;
}

impl AtWrite<'_> for Cgnspwr {
    type Input = PowerStatus;
    type Output = ();
}
