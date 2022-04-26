use super::{AtCommand, AtWrite};

pub struct Cntpcid;

impl AtCommand for Cntpcid {
    const COMMAND: &'static str = "AT+CNTPCID";
}

impl AtWrite<'_> for Cntpcid {
    type Input = i32;

    type Output = ();
}
