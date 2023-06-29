use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+CLTS=...
///
/// Enable or disable the network time USCs
///
/// * `*PSNWID`
/// * `*PSUTTZ`
/// * `+CTZV`
/// * `DST`
///
/// Also called "Get local timestamp".
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TimestampUsc {
    Enable,
    Disable,
}

impl AtRequest for TimestampUsc {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        match self {
            TimestampUsc::Enable => "AT+CLTS=1\r",
            TimestampUsc::Disable => "AT+CLTS=0\r",
        }
        .into()
    }
}
