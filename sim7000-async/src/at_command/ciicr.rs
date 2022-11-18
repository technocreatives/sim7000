use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIICR
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct StartGprs;

impl AtRequest for StartGprs {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        "AT+CIICR\r".into()
    }
}
