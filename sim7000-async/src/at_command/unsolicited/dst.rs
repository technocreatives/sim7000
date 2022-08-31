use crate::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
/// Daylight savings time
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dst;

impl ATParseLine for Dst {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "DST:", Dst)
    }
}
