use crate::{at_command::{AtParseErr, AtParseLine}, collect_array};

/// Refresh network time and timezone
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Psuttz;

impl AtParseLine for Psuttz {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "*PSUTTZ" {
            return Err("Missing *PSUTTZ prefix".into());
        }

        log::warn!("unimplemented: {:?}", line);

        let [year, month, day, hour, min, sec, timezone,dst] =
            collect_array(rest.splitn(7, ',')).ok_or("Missing ','")?;

        Ok(Psuttz)
    }
}
