use heapless::String;

use super::{AtRequest, GenericOk};

/// ATE1 / ATE0
pub struct SetEcho(pub bool);

impl AtRequest for SetEcho {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 { "ATE1\r" } else { "ATE0\r" }.into()
    }
}
