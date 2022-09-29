use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Network time zone
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ctzv;

impl AtParseLine for Ctzv {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "+CTZV:", Ctzv)
    }
}
