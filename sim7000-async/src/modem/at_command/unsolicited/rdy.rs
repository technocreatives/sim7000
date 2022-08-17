use crate::modem::at_command::{ATParseErr, ATParseLine};

/// Sim7000 indicates that it has powered on with a fixed baud rate
#[derive(Debug)]
pub struct Ready;

impl ATParseLine for Ready {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("RDY").then(|| Ready).ok_or("Missing 'RDY'".into())
    }
}
