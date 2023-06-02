use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum NbMode {
    CatM = 1,
    NbIot = 2,
    Both = 3,
}

/// AT+CMNB=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetNbMode(pub NbMode);

impl AtRequest for SetNbMode {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CMNB={}\r", self.0 as u8).unwrap();
        buf
    }
}
