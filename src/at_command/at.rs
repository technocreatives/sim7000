use heapless::String;

use super::{AtRequest, GenericOk};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct At;

impl AtRequest for At {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT\r".into()
    }
}
