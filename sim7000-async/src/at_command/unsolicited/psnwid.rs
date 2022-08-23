use crate::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
#[derive(Debug)]
pub struct Pdnwid;

impl ATParseLine for Pdnwid {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "*PSNWID:", Pdnwid)
    }
}
