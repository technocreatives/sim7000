use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CGNSPWR=...
pub struct SetGnssPower(pub bool);

impl AtRequest for SetGnssPower {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        let arg = if self.0 { "1" } else { "0" };
        write!(buf, "AT+CGNSPWR={arg}\r").unwrap();
        buf
    }
}
