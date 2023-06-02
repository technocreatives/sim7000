use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CRing;

impl AtParseLine for CRing {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "+CRING:", CRing)
    }
}
