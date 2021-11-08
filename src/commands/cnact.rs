use crate::Error;

use super::{AtCommand, AtEncode, AtWrite};

pub struct Cnact;

impl AtCommand for Cnact {
    const COMMAND: &'static str = "AT+CNACT";
}

impl<'a> AtWrite<'a> for Cnact {
    type Input = AppNetworkParams<'a>;

    type Output = ();
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum AppNetworkMode {
    Disable = 0,
    Enable = 1,
    Auto = 2,
}

pub struct AppNetworkParams<'a> {
    pub mode: AppNetworkMode,
    pub apn: &'a str,
}

impl AtEncode for AppNetworkParams<'_> {
    fn encode<B: crate::SerialWrite>(
        &self,
        encoder: &mut super::Encoder<B>,
    ) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(self.mode as i32)?;
        encoder.encode_str(",")?;
        encoder.encode_str("\"")?;
        encoder.encode_str(self.apn)?;
        encoder.encode_str("\"")
    }
}
