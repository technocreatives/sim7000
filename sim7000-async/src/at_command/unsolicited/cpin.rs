use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Indicates SIM password requirements
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CPin;

impl AtParseLine for CPin {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "+CPIN:", CPin)
    }
}
