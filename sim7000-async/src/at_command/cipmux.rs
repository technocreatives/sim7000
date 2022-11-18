use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIPMUX=...
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
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
