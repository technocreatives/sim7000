use crate::modem::at_command::{stub_parser_prefix, ATParseErr, ATParseLine};

// stub type
/// Network time zone
#[derive(Debug)]
pub struct Ctzv;

impl ATParseLine for Ctzv {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        stub_parser_prefix(line, "+CTZV:", Ctzv)
    }
}
