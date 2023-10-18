use crate::at_command::{AtParseErr, AtParseLine};

/// Network registration status
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
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "+CREG" {
            return Err("Missing '+CREG'".into());
        }

        let len = 1 + rest.chars().filter(|&c| c == ',').count();

        let mut fields = rest.split(',');

        // skip the <n> field if it exists
        match len {
            // URC variant
            // <stat>[,<lac>,<ci>,<netact>]
            1 | 4 => {}

            // Regular variant
            // <n>,<stat>[,<lac>,<ci>,<netact>]
            2 | 5 => {
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
            .and_then(|f| u32::from_str_radix(f.trim_matches('"'), 16).ok());

        Ok(NetworkRegistration { status, lac, ci })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lte_urc() {
        let creg_str = "+CREG: 5,\"FFFE\",\"1A8670B\",7";
        let creg = NetworkRegistration::from_line(creg_str).expect("Parse CREG");

        let expected = NetworkRegistration {
            status: RegistrationStatus::RegisteredRoaming,
            lac: Some(65534),
            ci: Some(27813643),
        };

        assert_eq!(expected, creg);
    }

    #[test]
    fn parse_gsm_urc() {
        let creg_str = "+CREG: 5,\"28A0\",\"2776\",0";
        let creg = NetworkRegistration::from_line(creg_str).expect("Parse CREG");

        let expected = NetworkRegistration {
            status: RegistrationStatus::RegisteredRoaming,
            lac: Some(10400),
            ci: Some(10102),
        };

        assert_eq!(expected, creg);
    }
}
