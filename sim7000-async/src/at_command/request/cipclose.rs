use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::CloseOk;

/// AT+CIPCLOSE=...
pub struct CloseConnection {
    pub connection: usize,
}

impl ATRequest for CloseConnection {
    type Response = CloseOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CIPCLOSE={}\r", self.connection).unwrap();
        buf
    }
}
