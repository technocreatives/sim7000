use crate::{Error, SerialWrite};

use super::{AtCommand, AtEncode, AtWrite, Encoder};

pub struct Ifc;

impl AtCommand for Ifc {
    const COMMAND: &'static str = "AT+IFC";
}

#[repr(i32)]
#[derive(Copy, Eq, Clone, PartialEq, Debug)]
pub enum FlowControlMode {
    None = 0,
    Software = 1,
    Hardware = 2,
}

#[derive(Copy, Clone)]
pub struct FlowControl {
    pub te_flow: FlowControlMode,
    pub ta_flow: FlowControlMode,
}

impl AtEncode for FlowControl {
    fn encode<B: SerialWrite>(&self, encoder: &mut Encoder<B>) -> Result<(), Error<B::SerialError>> {
        encoder.encode_scalar(self.te_flow as i32)?;
        encoder.encode_str(",")?;
        encoder.encode_scalar(self.ta_flow as i32)
    }
}

impl AtWrite<'_> for Ifc {
    type Input = FlowControl;
    type Output = ();
}
