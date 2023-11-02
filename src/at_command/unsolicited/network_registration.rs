use crate::at_command::{AtParseErr, AtParseLine};

use super::{cereg::CEReg, cgreg::CGReg, creg::CReg};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkRegistration {
    pub status: RegistrationStatus,

    /// Location area code
    pub lac: Option<u16>,

    /// Cell ID
    pub ci: Option<u32>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RegistrationStatus {
    NotRegistered,
    RegisteredHome,
    Searching,
    RegistrationDenied,
    Unknown,
    RegisteredRoaming,
}

impl AtParseLine for NetworkRegistration {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, _rest) = line.split_once(": ").ok_or("Missing ': '")?;
        match message {
            "+CREG" => CReg::parse(line),
            "+CGREG" => CGReg::parse(line),
            "+CEREG" => CEReg::parse(line),
            _ => Err("Missing any of '+CREG', '+CGREG' or '+CEREG'".into()),
        }
    }
}
