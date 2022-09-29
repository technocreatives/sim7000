use heapless::String;

use super::{AtRequest, GenericOk};

pub struct At;

impl AtRequest for At {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT\r".into()
    }
}
