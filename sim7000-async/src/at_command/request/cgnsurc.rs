use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CGNSURC=...
pub struct ConfigureGnssUrc {
    /// Send URC report every <n> GNSS fix.
    /// Set to 0 to disable.
    pub period: u8,
}

impl ATRequest for ConfigureGnssUrc {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CGNSURC={}\r", self.period).unwrap();
        buf
    }
}
