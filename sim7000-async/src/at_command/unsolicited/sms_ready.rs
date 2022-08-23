use crate::at_command::{ATParseErr, ATParseLine};

/// Sim7000 indicates that it has powered on with a fixed baud rate
#[derive(Debug)]
pub struct SmsReady;

impl ATParseLine for SmsReady {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("SMS Ready")
            .then(|| SmsReady)
            .ok_or("Missing 'SMS Ready'".into())
    }
}
