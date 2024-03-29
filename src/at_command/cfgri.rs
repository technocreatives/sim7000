use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RiPinMode {
    Off = 0,
    On = 1,
    OnTcpIp = 2,
}

/// AT+CFGRI=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConfigureRiPin(pub RiPinMode);

impl AtRequest for ConfigureRiPin {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CFGRI={}\r", self.0 as u8).unwrap();
        buf
    }
}
