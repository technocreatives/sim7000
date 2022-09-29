use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Indicates phone functionality
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CFun;

impl AtParseLine for CFun {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "+CFUN:", CFun)
    }
}
