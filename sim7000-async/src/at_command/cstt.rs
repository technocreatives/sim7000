use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CSTT=...
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct StartTask {
    pub apn: String<50>,
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
