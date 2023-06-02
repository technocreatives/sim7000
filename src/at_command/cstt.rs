use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CSTT=...
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StartTask {
    // The maximum length of an APN is 63 octets (bytes)
    pub apn: String<63>,
    pub username: String<50>,
    pub password: String<50>,
}

impl AtRequest for StartTask {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+CSTT={:?},{:?},{:?}\r",
            self.apn, self.username, self.password,
        )
        .unwrap();
        buf
    }
}
