use crate::modem::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
#[derive(Debug)]
pub struct Cmt;

impl ATParseLine for Cmt {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CMT:", Cmt)
    }
}
