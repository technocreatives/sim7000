use core::fmt::Write;
use heapless::String;

use super::{AtRequest, WritePrompt};

/// AT+CIPSEND
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IpSend {
    pub connection: usize,
    pub data_length: usize,
}

impl AtRequest for IpSend {
    type Response = WritePrompt;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CIPSEND={},{}\r", self.connection, self.data_length).unwrap();
        buf
    }
}
