use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIICR
pub struct StartGprs;

impl AtRequest for StartGprs {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+CIICR\r".into()
    }
}
