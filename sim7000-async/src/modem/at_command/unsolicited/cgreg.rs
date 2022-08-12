use crate::modem::at_command::{ATParseErr, ATParseLine};

/// Network registration status
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegistrationStatus {
    NotRegistered,
    RegisteredHome,
    Searching,
    RegistrationDenied,
    Unknown,
    RegisteredRoaming,
}

impl ATParseLine for RegistrationStatus {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, rest) = line.split_once(": ").ok_or(ATParseErr)?;
        if message != "+CGREG" {
            return Err(ATParseErr);
        }

        let stat = rest.split(',').nth(1).ok_or(ATParseErr)?.parse::<i32>()?;

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
