use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CSCS=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetTeCharacterSet(pub CharacterSet);

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CharacterSet {
    GSM,
    UCS2,
    IRA,
}

impl AtRequest for SetTeCharacterSet {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let character_set = match self.0 {
            CharacterSet::GSM => "GSM",
            CharacterSet::UCS2 => "USC2",
            CharacterSet::IRA => "IRA",
        };

        let mut buf = String::new();
        write!(buf, "AT+CSCS=\"{}\"\r", character_set).unwrap();
        buf
    }
}
