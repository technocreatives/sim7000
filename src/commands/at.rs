use super::{AtCommand, AtExecute};

pub struct At;

impl AtCommand for At {
    const COMMAND: &'static str = "AT";
}

impl AtExecute for At {
    type Output = ();
}
