use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Cmee;

impl AtCommand for Cmee {
    const COMMAND: &'static str = "AT+CMEE";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum ErrorReportMode {
    Disable = 0,
    Enable = 1,
    Verbose = 2,
}

impl AtEncode for ErrorReportMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtWrite<'_> for Cmee {
    type Input = ErrorReportMode;
    type Output = ();
}
