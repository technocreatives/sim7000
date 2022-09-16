use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum NbMode {
    CatM = 1,
    NbIot = 2,
    Both = 3,
}

/// AT+CMNB=...
pub struct SetNbMode(pub NbMode);

impl ATRequest for SetNbMode {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CMNB={}\r", self.0 as u8).unwrap();
        buf
    }
}
