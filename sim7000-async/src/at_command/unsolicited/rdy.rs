use crate::at_command::{ATParseErr, ATParseLine};

/// Sim7000 indicates that it has powered on with a fixed baud rate
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ready;

impl ATParseLine for Ready {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("RDY")
            .then(|| Ready)
            .ok_or_else(|| "Missing 'RDY'".into())
    }
}
