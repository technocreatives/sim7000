use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Daylight savings time
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dst;

impl AtParseLine for Dst {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        stub_parser_prefix(line, "DST:", Dst)
    }
}
