use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CSCLK=<1 or 0>
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
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
