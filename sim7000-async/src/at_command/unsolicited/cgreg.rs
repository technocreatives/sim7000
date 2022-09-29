use crate::at_command::{AtParseErr, AtParseLine};

/// Network registration status
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

impl AtParseLine for RegistrationStatus {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "+CGREG" {
            return Err("Missing '+CGREG'".into());
        }

        let stat = rest
            .split(',')
            .nth(1)
            .ok_or("Missing ','")?
            .parse::<i32>()?;

        Ok(match stat {
            1 => RegistrationStatus::RegisteredHome,
            2 => RegistrationStatus::Searching,
            3 => RegistrationStatus::RegistrationDenied,
            4 => RegistrationStatus::Unknown,
            5 => RegistrationStatus::RegisteredRoaming,
            _ => RegistrationStatus::NotRegistered,
        })
    }
}
