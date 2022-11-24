use core::any::type_name;
use core::ops::Not;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::log;

pub(crate) struct Slot<T: 'static> {
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
    pub fn claim(&self) -> Option<&T> {
        self.is_claimed
            .fetch_or(true, Ordering::Relaxed)
            .not()
            .then(|| &self.inner)
    }

    /// Look in the slot without claiming it
    pub fn peek(&self) -> &T {
        &self.inner
    }

    /// Release the claim on the slot
    pub fn release(&self) {
        if !self.is_claimed.fetch_and(false, Ordering::Relaxed) {
            log::error!("Tried to release unclaimed Slot<{:?}>", type_name::<T>());
        }
    }

    pub fn is_free(&self) -> bool {
        !self.is_claimed.fetch_or(false, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn slot() {
        let slot = Slot::new(());

        for _ in 0..3 {
            assert!(slot.is_free());
            assert!(slot.is_free());
            assert!(slot.claim().is_some());
            assert!(!slot.is_free());
            assert!(!slot.is_free());
            assert!(slot.claim().is_none());
            assert!(!slot.is_free());
            slot.release();
        }
    }
}
