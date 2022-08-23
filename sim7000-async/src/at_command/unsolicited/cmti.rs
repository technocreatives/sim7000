use crate::modem::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
#[derive(Debug)]
pub struct Cmti;

impl ATParseLine for Cmti {
    fn parse_from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CMTI:", Cmti)
    }
}
