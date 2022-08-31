use crate::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
/// Indicates SIM password requirements
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CPin;

impl ATParseLine for CPin {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CPIN:", CPin)
    }
}
