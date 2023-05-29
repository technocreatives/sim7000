use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CGNSHOR
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DesiredPositionAccuracy(pub u32);

impl AtRequest for DesiredPositionAccuracy {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSHOR={}\r", self.0).unwrap();
        buf
    }
}
