use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ToggleXtra {
    Disable = 0,
    Enable = 1,
}

/// AT+CGNSXTRA=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GnssXtra(pub ToggleXtra);

impl AtRequest for GnssXtra {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSXTRA={}\r", self.0 as u8).unwrap();
        buf
    }
}
