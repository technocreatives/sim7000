use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIPSPRT=...
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SetCipSendPrompt {
    /// Send SEND OK but do not send "> " prompt
    ResponseNoPrompt = 0,

    /// Send SEND OK and send "> " prompt
    ResponseAndPrompt = 1,

    /// Do not send anything
    NoResponseNoPrompt = 2,
}

impl AtRequest for SetCipSendPrompt {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CIPSPRT={}\r", *self as u8).unwrap();
        buf
    }
}
