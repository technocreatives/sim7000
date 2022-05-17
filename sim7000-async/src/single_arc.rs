use core::{sync::atomic::{AtomicU8, Ordering}, ops::Deref, cell::UnsafeCell};

pub struct SingletonArc<T> {
    inner: UnsafeCell<Option<T>>,
    refcount: AtomicU8,
}

unsafe impl<T: Send> Send for SingletonArc<T> {}
unsafe impl<T: Send> Sync for SingletonArc<T> {}

impl<T> SingletonArc<T> {
    pub const fn new() -> Self {
        SingletonArc {
            inner: UnsafeCell::new(None),
            refcount: AtomicU8::new(0),
        }
    }

    pub fn get_or_init<F: FnOnce() -> T>(&self, f: F) -> SingletonArcGuard<T> {
        critical_section::with(|_| {
            // Safety: we are in a critical section, so no other code can be running at this time.
            let ptr = unsafe { &mut *self.inner.get() };
            if ptr.is_none() {
                *ptr = Some(f());
            }
        },);

        if self.refcount.fetch_add(1, Ordering::SeqCst) == u8::MAX {
            panic!("Refcount wrapped around, too many references to this instance")
        }

        SingletonArcGuard { inner: self }
    }
}

pub struct SingletonArcGuard<'s, T> {
    inner: &'s SingletonArc<T>
}

impl<'s, T> Deref for SingletonArcGuard<'s, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: its only UB to create an immutable reference if someone holds a mutable reference. The only time a mutable reference to the cell is taken is during the last drop, at which point deref will not be called.
        let ptr = unsafe {&*(self.inner.inner.get() as *const Option<T>)};
        // Safety: creating an instance of this struct increments the refcount by one, the option is only set to None when there are no more instances of this struct.
        unsafe {ptr.as_ref().unwrap_unchecked() }
    }
}

impl<'s, T> Clone for SingletonArcGuard<'s, T> {
    fn clone(&self) -> Self {
        if self.inner.refcount.fetch_add(1, Ordering::SeqCst) == u8::MAX {
            panic!("Refcount wrapped around, too many references to this instance")
        }
        Self { inner: self.inner.clone() }
    }
} 

impl<'s, T> Drop for SingletonArcGuard<'s, T> {
    fn drop(&mut self) {
        if self.inner.refcount.fetch_sub(1, Ordering::Relaxed) == 1 {
            critical_section::with(|_| {
                    // Safety: we are in a critical section, so no other code can be running at this time.
                    let ptr = unsafe { &mut *self.inner.inner.get() };
                    *ptr = None;
            },);
        }
    }
}