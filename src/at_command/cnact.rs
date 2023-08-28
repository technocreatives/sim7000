use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CnactMode {
    Deactive = 0,
    Active = 1,
    AutoActive = 2,
}

/// AT+CNACT=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetAppNetwork {
    pub mode: CnactMode,
    pub apn: String<63>,
}

impl AtRequest for SetAppNetwork {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(buf, "AT+CNACT={},{:?}\r", self.mode as u8, self.apn).unwrap();
        buf
    }
}
