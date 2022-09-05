use heapless::String;

use super::ATRequest;
use crate::at_command::response::{GenericOk, SignalQuality};

/// AT+CSQ
pub struct GetSignalQuality;

impl ATRequest for GetSignalQuality {
    type Response = (SignalQuality, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CSQ\r".into()
    }
}
