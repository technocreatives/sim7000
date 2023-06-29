use crate::log;
use crate::{
    at_command::{AtParseErr, AtParseLine},
    collect_array,
};

/// Refresh network name
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Psnwid;

impl AtParseLine for Psnwid {
    fn from_line(line: &str) -> Result<Self, AtParseErr> {
        let (message, rest) = line.split_once(": ").ok_or("Missing ': '")?;

        if message != "*PSNWID" {
            return Err("Missing *PSNWID prefix".into());
        }

        log::warn!("unimplemented: {:?}", line);

        let [mcc, mnc, full_network_name, full_network_name_ci, short_network_name, short_network_name_ci] =
            collect_array(rest.splitn(6, ',')).ok_or("Missing ','")?;

        Ok(Psnwid)
    }
}
