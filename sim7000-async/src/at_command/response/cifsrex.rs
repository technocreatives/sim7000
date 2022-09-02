use crate::{
    at_command::{ATParseErr, ATParseLine},
    util::collect_array,
};

use super::{ATResponse, ResponseCode};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IpExt {
    pub addr: [u8; 4],
}

impl ATParseLine for IpExt {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let addr = line
            .strip_prefix("+CIFSREX: ")
            .ok_or("Missing '+CIFSREX: '")?;
        let addr = collect_array(addr.splitn(4, '.').filter_map(|seg| seg.parse().ok()))
            .ok_or("Failed to parse IP segment")?;

        Ok(IpExt { addr })
    }
}

impl ATResponse for IpExt {
    fn from_generic(code: ResponseCode) -> Result<Self, ResponseCode> {
        match code {
            ResponseCode::IpExt(ip_ext) => Ok(ip_ext),
            _ => Err(code),
        }
    }
}
