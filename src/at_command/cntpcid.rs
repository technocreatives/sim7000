use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CNTPCID=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetGprsBearerProfileId(pub u8);

impl AtRequest for SetGprsBearerProfileId {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CNTPCID={}\r", self.0).unwrap();
        buf
    }
}
