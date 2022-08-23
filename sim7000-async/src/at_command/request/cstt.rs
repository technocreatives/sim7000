use core::fmt::Write;
use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CSTT=...
pub struct StartTask {
    pub apn: String<50>,
    pub username: String<50>,
    pub password: String<50>,
}

impl ATRequest for StartTask {
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
