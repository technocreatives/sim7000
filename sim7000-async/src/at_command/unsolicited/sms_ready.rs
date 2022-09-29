use crate::at_command::{AtParseErr, AtParseLine};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmsReady;

impl AtParseLine for SmsReady {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        line.eq("SMS Ready")
            .then(|| SmsReady)
            .ok_or_else(|| "Missing 'SMS Ready'".into())
    }
}
