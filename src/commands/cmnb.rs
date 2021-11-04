use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Cmnb;

impl AtCommand for Cmnb {
    const COMMAND: &'static str = "AT+CMNB";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum BandMode {
    CatM = 1,
    NbIot = 2,
    CatMnbIot = 3,
}

impl AtEncode for BandMode {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}

impl AtWrite<'_> for Cmnb {
    type Input = BandMode;
    type Output = ();
}
