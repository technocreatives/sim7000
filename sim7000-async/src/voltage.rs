use embassy_util::channel::signal::Signal;

use crate::at_command::unsolicited::VoltageWarning;
use crate::slot::Slot;

pub struct VoltageWarner<'c> {
    pub(crate) signal: &'c Signal<VoltageWarning>,
    pub(crate) slot: &'c Slot<Signal<VoltageWarning>>,
}

impl<'c> VoltageWarner<'c> {
    pub(crate) fn take(slot: &'c Slot<Signal<VoltageWarning>>) -> Option<Self> {
        let signal = slot.claim()?;
        signal.reset();
        Some(VoltageWarner { signal, slot })
    }

    /// Wait for any voltage warning
    pub async fn warning(&self) -> VoltageWarning {
        self.signal.wait().await
    }
}

impl Drop for VoltageWarner<'_> {
    fn drop(&mut self) {
        self.slot.release();
    }
}
