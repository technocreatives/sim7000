use core::any::type_name;
use core::ops::Not;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::log;

pub struct Slot<T: 'static> {
    is_claimed: AtomicBool,
    inner: T,
}

impl<T: 'static> Slot<T> {
    pub const fn new(inner: T) -> Self {
        Slot {
            inner,
            is_claimed: AtomicBool::new(false),
        }
    }

    /// Try to claim the slot, returns None if the slot has already been claimed
    pub(crate) fn claim(&self) -> Option<&T> {
        self.is_claimed
            .fetch_or(true, Ordering::Relaxed)
            .not()
            .then(|| &self.inner)
    }

    /// Look in the slot without claiming it
    pub(crate) fn peek(&self) -> &T {
        &self.inner
    }

    /// Release the claim on the slot
    pub(crate) fn release(&self) {
        if !self.is_claimed.fetch_and(false, Ordering::Relaxed) {
            log::error!("Tried to release unclaimed Slot<{:?}>", type_name::<T>());
        }
    }

    pub(crate) fn is_claimed(&self) -> bool {
        self.is_claimed.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn slot() {
        let slot = Slot::new(());

        for _ in 0..3 {
            assert!(!slot.is_claimed());
            assert!(!slot.is_claimed());
            assert!(slot.claim().is_some());
            assert!(slot.is_claimed());
            assert!(slot.is_claimed());
            assert!(slot.claim().is_none());
            assert!(slot.is_claimed());
            slot.release();
        }
    }
}
