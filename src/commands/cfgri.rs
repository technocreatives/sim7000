use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Cfgri;

impl AtCommand for Cfgri {
    const COMMAND: &'static str = "AT+CFGRI";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum RiMode {
    Off = 0,
    All = 1,
    TcpOnly = 2,
}

impl AtEncode for RiMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtWrite<'_> for Cfgri {
    type Input = RiMode;
    type Output = ();
}
