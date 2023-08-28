use crate::at_command::{AtParseErr, AtParseLine};

/// Network registration status
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NetworkRegistration {
    pub status: RegistrationStatus,

    /// Location area code
    pub lac: Option<u16>,

    /// Cell ID
    pub ci: Option<u16>,
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
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "+CGREG" {
            return Err("Missing '+CGREG'".into());
        }

        let len = 1 + rest.chars().filter(|&c| c == ',').count();

        let mut fields = rest.split(',');

        // skip the <n> field if it exists
        match len {
            // URC variant
            // <stat>[,<lac>,<ci>,<netact>]
            1 | 4 => {}

            // Regular variant
            // <n>,<stat>[,<lac>,<ci>,<netact>[,[<Active-Time>],[<Periodic-RAU>],[<GPRS-READY-timer>]]]
            2 | 5 | 8 => {
                let _ = fields.next();
            }
            _ => return Err("Invalid number of elements".into()),
        };

        let status = fields.next().ok_or("Missing ','")?.parse::<i32>()?;

        let status = match status {
            1 => RegistrationStatus::RegisteredHome,
            2 => RegistrationStatus::Searching,
            3 => RegistrationStatus::RegistrationDenied,
            4 => RegistrationStatus::Unknown,
            5 => RegistrationStatus::RegisteredRoaming,
            _ => RegistrationStatus::NotRegistered,
        };

        let lac = fields
            .next()
            .and_then(|f| u16::from_str_radix(f.trim_matches('"'), 16).ok());
        let ci = fields
            .next()
            .and_then(|f| u16::from_str_radix(f.trim_matches('"'), 16).ok());

        Ok(NetworkRegistration { status, lac, ci })
    }
}
