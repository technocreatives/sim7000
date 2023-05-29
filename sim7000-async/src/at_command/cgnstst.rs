use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CGNSTST
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetNmeaOutput(pub u8, pub u8);

impl AtRequest for SetNmeaOutput {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSTST={},{}\r", self.0, self.1).unwrap();
        buf
    }
}
