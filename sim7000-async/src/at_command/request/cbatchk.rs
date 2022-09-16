use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CBATCHK=...
pub struct EnableVBatCheck(pub bool);

impl ATRequest for EnableVBatCheck {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 {
            "AT+CBATCHK=1\r"
        } else {
            "AT+CBATCHK=0\r"
        }
        .into()
    }
}
