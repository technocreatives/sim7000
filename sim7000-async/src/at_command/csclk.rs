use heapless::String;

use super::{AtRequest, GenericOk};

pub struct SetSlowClock(pub bool);

impl AtRequest for SetSlowClock {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 {
            "AT+CSCLK=1\r"
        } else {
            "AT+CSCLK=0\r"
        }
        .into()
    }
}
