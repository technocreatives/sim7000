use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CSMS=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SelectMessageService;

impl AtRequest for SelectMessageService {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CSMS=0\r").unwrap();
        buf
    }
}
