use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CIICR
pub struct StartGprs;

impl ATRequest for StartGprs {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+CIICR\r".into()
    }
}
