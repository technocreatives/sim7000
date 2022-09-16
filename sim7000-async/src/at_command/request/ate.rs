use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// ATE1 / ATE0
pub struct SetEcho(pub bool);

impl ATRequest for SetEcho {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        if self.0 { "ATE1\r" } else { "ATE0\r" }.into()
    }
}
