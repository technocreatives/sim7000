use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub enum CMEErrorMode {
    Disable = 0,
    Numeric = 1,
    Verbose = 2,
}

/// AT+CMEE=...
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct ConfigureCMEErrors(pub CMEErrorMode);

impl AtRequest for ConfigureCMEErrors {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CMEE={}\r", self.0 as u8).unwrap();
        buf
    }
}
