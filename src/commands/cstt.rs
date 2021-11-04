use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Cstt;

impl AtCommand for Cstt {
    const COMMAND: &'static str = "AT+CSTT";
}

#[derive(Clone, Copy)]
pub struct CsttParams {
    pub apn: &'static str,
    pub username: &'static str,
    pub password: &'static str,
}

impl AtEncode for CsttParams {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_str("\"")?;
        encoder.encode_str(self.apn)?;
        encoder.encode_str("\",\"")?;
        encoder.encode_str(self.username)?;
        encoder.encode_str("\",\"")?;
        encoder.encode_str(self.password)?;
        encoder.encode_str("\"")
    }
}

impl AtWrite<'_> for Cstt {
    type Input = CsttParams;
    type Output = ();
}
