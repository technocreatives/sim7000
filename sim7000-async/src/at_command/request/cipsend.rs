use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::WritePrompt;

/// AT+CIPSEND
pub struct IpSend {
    pub connection: usize,
    pub data_length: usize,
}

impl ATRequest for IpSend {
    type Response = WritePrompt;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CIPSEND={},{}\r", self.connection, self.data_length).unwrap();
        buf
    }
}
