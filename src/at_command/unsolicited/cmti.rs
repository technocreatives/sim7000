use heapless::String;

use crate::at_command::{AtParseErr, AtParseLine};

// stub type
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct NewSmsIndex {
    pub memory: String<2>,
    pub index: u8,
}

impl AtParseLine for NewSmsIndex {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "+CMTI" {
            return Err("Missing '+CMTI'".into());
        }

        let (memory, index) = rest.split_once(',').ok_or("Missing ','")?;

        Ok(Self {
            memory: memory.trim_matches('\"').into(),
            index: index.parse()?,
        })
    }
}
