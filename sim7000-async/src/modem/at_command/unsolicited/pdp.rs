use crate::modem::at_command::{ATParseErr, ATParseLine};

#[derive(Debug, PartialEq, Eq)]
pub struct GprsDisconnected;

impl ATParseLine for GprsDisconnected {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("+PDP: DEACT")
            .then(|| GprsDisconnected)
            .ok_or(ATParseErr)
    }
}
