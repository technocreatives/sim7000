use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

pub struct SetSlowClock(pub bool);

impl ATRequest for SetSlowClock {
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
