use heapless::String;

use crate::at_command::response::{GenericOk, SystemInfo};

use super::ATRequest;

pub struct GetSystemInfo;

impl ATRequest for GetSystemInfo {
    type Response = (SystemInfo, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CPSI?\r".into()
    }
}
