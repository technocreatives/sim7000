use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CIPSHUT
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(not(feature = "defmt"), derive(Debug))]
pub struct ShutConnections;

impl AtRequest for ShutConnections {
    type Response = GenericOk; // TODO: should have its own type
    fn encode(&self) -> String<256> {
        "AT+CIPSHUT\r".into()
    }
}
