use crate::at_command::{AtParseErr, AtParseLine};

/// Sim7000 indicates that it has powered on with a fixed baud rate
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ready;

impl AtParseLine for Ready {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        line.eq("RDY")
            .then(|| Ready)
            .ok_or_else(|| "Missing 'RDY'".into())
    }
}
