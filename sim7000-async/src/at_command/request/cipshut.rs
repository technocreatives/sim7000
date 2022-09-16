use heapless::String;

use super::ATRequest;
use crate::at_command::response::GenericOk;

/// AT+CIPSHUT
pub struct ShutConnections;

impl ATRequest for ShutConnections {
    type Response = GenericOk; // TODO: should have its own type
    fn encode(&self) -> String<256> {
        "AT+CIPSHUT\r".into()
    }
}
