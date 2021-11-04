use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Csclk;

impl AtCommand for Csclk {
    const COMMAND: &'static str = "AT+CSCLK";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum ClockMode {
    Disable = 0,
    Enable = 1,
}

impl AtEncode for ClockMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtWrite<'_> for Csclk {
    type Input = ClockMode;
    type Output = ();
}
