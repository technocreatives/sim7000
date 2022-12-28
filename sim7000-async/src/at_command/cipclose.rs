use core::fmt::Write;
use heapless::String;

use super::{AtRequest, CloseOk};

/// AT+CIPCLOSE=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CloseConnection {
    pub connection: usize,
}

impl AtRequest for CloseConnection {
    type Response = CloseOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CIPCLOSE={}\r", self.connection).unwrap();
        buf
    }
}
