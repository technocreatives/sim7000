use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Cell Broadcast Message
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cbm;

impl AtParseLine for Cbm {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "+CBM:", Cbm)
    }
}
