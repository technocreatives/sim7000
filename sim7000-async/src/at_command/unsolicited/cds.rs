use crate::modem::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
#[derive(Debug)]
pub struct Cds;

impl ATParseLine for Cds {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CDS:", Cds)
    }
}
