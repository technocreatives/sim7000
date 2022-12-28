use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIICR
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StartGprs;

impl AtRequest for StartGprs {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+CIICR\r".into()
    }
}
