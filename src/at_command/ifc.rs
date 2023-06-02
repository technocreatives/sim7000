use core::fmt::Write;
use heapless::String;

use super::{AtRequest, GenericOk};

/// AT+IFC=...
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SetFlowControl {
    pub dce_by_dte: FlowControl,
    pub dte_by_dce: FlowControl,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FlowControl {
    NoFlowControl = 0,
    Software = 1,
    Hardware = 2,
}

impl AtRequest for SetFlowControl {
    type Response = GenericOk;
    fn encode(&self) -> String<256> {
        let mut buf = String::new();
        write!(
            buf,
            "AT+IFC={},{}\r",
            self.dce_by_dte as u8, self.dte_by_dce as u8
        )
        .unwrap();
        buf
    }
}
