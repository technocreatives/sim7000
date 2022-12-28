use heapless::String;

use super::{AtRequest, GenericOk};

/// ATE1 / ATE0
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetEcho(pub bool);

impl AtRequest for SetEcho {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 { "ATE1\r" } else { "ATE0\r" }.into()
    }
}
