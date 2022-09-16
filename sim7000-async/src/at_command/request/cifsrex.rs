use heapless::String;

use super::ATRequest;
use crate::at_command::response::{GenericOk, IpExt};

/// AT+CIFSREX
pub struct GetLocalIpExt;

impl ATRequest for GetLocalIpExt {
    type Response = (IpExt, GenericOk);
    fn encode(&self) -> String<256> {
        "AT+CIFSREX\r".into()
    }
}
