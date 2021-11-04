use super::{AtCommand, AtWrite};

pub struct Ipr;

impl AtCommand for Ipr {
    const COMMAND: &'static str = "AT+IPR";
}

impl AtWrite<'_> for Ipr {
    type Input = i32;
    type Output = ();
}
