use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIPSHUT
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ShutConnections;

impl AtRequest for ShutConnections {
    type Response = GenericOk; // TODO: should have its own type
    fn encode(&self) -> String<256> {
        "AT+CIPSHUT\r".into()
    }
}
