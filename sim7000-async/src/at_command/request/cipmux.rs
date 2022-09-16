use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CIPMUX=...
pub struct EnableMultiIpConnection(pub bool);

impl ATRequest for EnableMultiIpConnection {
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
