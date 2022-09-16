use crate::at_command::{ATParseErr, ATParseLine};
use crate::util::collect_array;

/// Indicates whether the app network is active
#[derive(Debug)]
pub struct IncomingConnection {
    // core::net::IpAddr doesn't exist, very sad.
    // TODO: find out if modem supports ipv6
    pub remote_ip: [u8; 4],
}

impl ATParseLine for IncomingConnection {
    fn from_line(line: &str) -> Result<Self, ATParseErr> {
        let (message, ip) = line.split_once(": ").ok_or("Missing ': '")?;
        if message != "REMOTE IP" {
            return Err("Missing 'REMOTE IP'".into());
        }

        Ok(IncomingConnection {
            remote_ip: collect_array(ip.splitn(4, '.').filter_map(|segment| segment.parse().ok()))
                .ok_or("Couldn't parse IP addr")?,
        })
    }
}
