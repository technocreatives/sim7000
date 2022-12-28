use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIPMUX=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct EnableMultiIpConnection(pub bool);

impl AtRequest for EnableMultiIpConnection {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 {
            "AT+CIPMUX=1\r"
        } else {
            "AT+CIPMUX=0\r"
        }
        .into()
    }
}
