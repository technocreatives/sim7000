use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Cnmp;

impl AtCommand for Cnmp {
    const COMMAND: &'static str = "AT+CNMP";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum PreferredConnectionMode {
    Automatic = 2,
    Gsm = 13,
    Lte = 38,
    GsmAndLte = 51,
}

impl AtEncode for PreferredConnectionMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtWrite<'_> for Cnmp {
    type Input = PreferredConnectionMode;
    type Output = ();
}
