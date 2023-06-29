use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Daylight savings time
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum Dst {
    NoAdjustment = 0,
    _1hour = 1,
    _2hours = 2,
}

impl AtParseLine for Dst {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "DST" {
            return Err("Missing +DST prefix".into());
        }

        Ok(match rest {
            "0" => Dst::NoAdjustment,
            "1" => Dst::_1hour,
            "2" => Dst::_2hours,
            _ => return Err("Invalid DST value".into()),
        })
    }
}
