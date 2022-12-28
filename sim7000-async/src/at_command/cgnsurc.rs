use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CGNSURC=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConfigureGnssUrc {
    /// Send URC report every <n> GNSS fix.
    /// Set to 0 to disable.
    pub period: u8,
}

impl AtRequest for ConfigureGnssUrc {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSURC={}\r", self.period).unwrap();
        buf
    }
}
