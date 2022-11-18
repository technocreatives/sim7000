use heapless::String;

use super::{AtRequest, GenericOk};

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct At;

impl AtRequest for At {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT\r".into()
    }
}
