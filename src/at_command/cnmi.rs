use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CNMI=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetSmsIndication {
    pub mode: SmsIndicationMode,
    /// mt
    pub routing: SmsMtMode,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum SmsIndicationMode {
    BufferInTa = 0,
    DiscardWhenLinkBusy = 1,
    BufferWhenLinkBusy = 2,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum SmsMtMode {
    NoRouting = 0,
    Index = 1,
    // Unimplemented URC
    // Direct = 2,
}

impl AtRequest for SetSmsIndication {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CNMI={},{},0,0,0\r",
            self.mode as u8, self.routing as u8
        )
        .unwrap();
        buf
    }
}
