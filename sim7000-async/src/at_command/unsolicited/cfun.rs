use crate::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
/// Indicates phone functionality
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CFun;

impl ATParseLine for CFun {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CFUN:", CFun)
    }
}
