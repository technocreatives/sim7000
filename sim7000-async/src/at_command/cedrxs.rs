use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EDRXSetting {
    Disable = 0,
    Enable = 1,
    EnableWithAutoReport = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AcTType {
    CatM = 4,
    NbIot = 5,
}

/// AT+CEDRX=...
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConfigureEDRX {
    pub n: EDRXSetting,
    pub act_type: AcTType,
    pub requested_edrx_value: EdrxCycleLength,
}

/// The EDRX cycle length, in seconds.
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum EdrxCycleLength {
    _5 = 0x0,
    _10 = 0x1,
    _20 = 0x2,
    _40 = 0x3,
    _61 = 0x4,
    _81 = 0x5,
    _102 = 0x6,
    _122 = 0x7,
    _143 = 0x8,
    _163 = 0x9,
    _327 = 0xA,
    _655 = 0xB,
    _1310 = 0xC,
    _2621 = 0xD,
    _5242 = 0xE,
    _10485 = 0xF,
}

impl AtRequest for ConfigureEDRX {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CEDRXS={},{},\"{:04b}\"\r",
            self.n as u8, self.act_type as u8, self.requested_edrx_value as u8,
        )
        .unwrap();
        buf
    }
}
