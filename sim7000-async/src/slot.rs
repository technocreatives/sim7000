use core::any::type_name;
use core::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct Slot<T: 'static> {
    is_free: AtomicBool,
    inner: T,
}

impl<T: 'static> Slot<T> {
    pub const fn new(inner: T) -> Self {
        Slot {
            inner,
            is_free: AtomicBool::new(true),
        }
    }

    /// Try to claim the slot, returns None if the slot has already been claimed
    pub fn claim(&self) -> Option<&T> {
        self.is_free
            .fetch_and(false, Ordering::Relaxed)
            .then(|| &self.inner)
    }

    /// Look in the slot without claiming it
    pub fn peek(&self) -> &T {
        &self.inner
    }

    /// Release the claim on the slot
    pub fn release(&self) {
        if self.is_free.fetch_or(true, Ordering::Relaxed) {
            log::error!("Tried to release unclaimed Slot<{:?}>", type_name::<T>());
        }
    }
}
