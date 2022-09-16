use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

pub struct At;

impl ATRequest for At {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT\r".into()
    }
}
