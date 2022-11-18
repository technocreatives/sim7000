use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CBATCHK=...
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct EnableVBatCheck(pub bool);

impl AtRequest for EnableVBatCheck {
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
