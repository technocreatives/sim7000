use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum EDRXSetting {
    Disable = 0,
    Enable = 1,
    EnableWithAutoReport = 2,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AcTType {
    CatM = 4,
    NbIot = 5,
}

/// AT+CEDRX=...
#[derive(Clone, Copy)]
pub struct ConfigureEDRX {
    pub n: EDRXSetting,
    pub act_type: AcTType,
    pub requested_edrx_value: u8,
}

impl ATRequest for ConfigureEDRX {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CEDRXS={},{},\"{:04b}\"\r",
            self.n as u8,
            self.act_type as u8,
            self.requested_edrx_value & 0b1111,
        )
        .unwrap();
        buf
    }
}
