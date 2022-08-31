use crate::at_command::{ATParseErr, ATParseLine};

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GprsDisconnected;

impl ATParseLine for GprsDisconnected {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        line.eq("+PDP: DEACT")
            .then(|| GprsDisconnected)
            .ok_or_else(|| "Missing '+PDP: DEACT'".into())
    }
}
