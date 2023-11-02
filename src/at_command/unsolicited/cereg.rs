use super::{network_registration::RegistrationStatus, NetworkRegistration};
use crate::at_command::AtParseErr;

/// Network registration status
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CEReg;

impl CEReg {
    pub(crate) fn parse(line: &str) -> Result<NetworkRegistration, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "+CEREG" {
            return Err("Missing '+CEREG'".into());
        }

        let len = 1 + rest.chars().filter(|&c| c == ',').count();

        let mut fields = rest.split(',');

        // Warning: Horror show below.
        // according to simcom, the output from cereg should look something like
        // <stat>[,[<tac>],[<rac>],[<ci>],[<AcT>]]
        // or
        // <n>,<stat>[,[<tac>],[<rac>],[<ci>],[<AcT>][,,[,[<Active-Time>],[<Periodic-TAU>]]]]
        // depending on whether it's a URC or not.
        // but those grammars are horseshit, and can't be trusted. So i'm taking the easy path and ignoring everthing but the <stat> field
        let status: i32 = match len {
            // if we only have one field, it's the <stat> field. Parse it.
            1 => fields.next().ok_or("Missing ','")?.parse()?,

            2.. => {
                // If we have two or more fields, we have no idea what they are (see above).
                // But, if we manage to parse the second one as an int, we can assume it's the <stat> field.
                // Because the fields that can come after <stat> are strings.
                let first = fields.next().ok_or("Missing ','")?;
                let second = fields.next().ok_or("Missing ','")?;

                if second.chars().all(|c| c.is_ascii_digit()) {
                    second.parse()?
                } else {
                    first.parse()?
                }
            }
            _ => return Err("Invalid number of elements".into()),
        };

        let status = match status {
            1 => RegistrationStatus::RegisteredHome,
            2 => RegistrationStatus::Searching,
            3 => RegistrationStatus::RegistrationDenied,
            4 => RegistrationStatus::Unknown,
            5 => RegistrationStatus::RegisteredRoaming,
            _ => RegistrationStatus::NotRegistered,
        };

        Ok(NetworkRegistration {
            status,
            lac: None,
            ci: None,
        })
    }
}
