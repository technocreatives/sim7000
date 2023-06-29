use crate::at_command::{stub_parser_prefix, AtParseErr, AtParseLine};

// stub type
/// Network time zone
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ctzv;

impl AtParseLine for Ctzv {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "+CTZV" {
            return Err("Missing +CTZV prefix".into());
        }

        let timezone = rest; // TODO: how to parse this?
        log::warn!("unimplemented: {:?}", line);

        Ok(Ctzv)
    }
}
