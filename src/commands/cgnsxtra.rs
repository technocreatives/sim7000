use crate::Error;

use super::{AtCommand, AtEncode, AtWrite};

pub struct Cgnsxtra;

impl AtCommand for Cgnsxtra {
    const COMMAND: &'static str = "AT+CGNSXTRA";
}

impl AtWrite<'_> for Cgnsxtra {
    type Input = XtraMode;

    type Output = ();
}

#[repr(i32)]
#[derive(Clone, Copy, Debug)]
pub enum XtraMode {
    Disable = 0,
    Enable = 1,
}

impl AtEncode for XtraMode {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(*self as i32)
    }
}
