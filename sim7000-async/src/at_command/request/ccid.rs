use heapless::String;

use crate::at_command::response::{GenericOk, Iccid};

use super::ATRequest;

pub struct ShowIccid;

impl ATRequest for ShowIccid {
    type Response = (Iccid, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CCID\r".into()
    }
}
