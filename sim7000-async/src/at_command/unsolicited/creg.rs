use crate::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
#[derive(Debug)]
pub struct CReg;

impl ATParseLine for CReg {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CREG:", CReg)
    }
}
