use crate::at_command::{ATParseErr, ATParseLine};

#[derive(Debug)]
pub struct SmsReady;

impl ATParseLine for SmsReady {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("SMS Ready")
            .then(|| SmsReady)
            .ok_or_else(|| "Missing 'SMS Ready'".into())
    }
}
