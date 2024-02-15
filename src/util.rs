use core::{
    cell::RefCell,
    fmt::Debug,
    future::{poll_fn, Future},
    task::Poll,
};

use embassy_sync::{blocking_mutex, blocking_mutex::raw::RawMutex, waitqueue::WakerRegistration};
use heapless::Deque;

use crate::at_command::AtParseErr;

#[track_caller]
pub(crate) fn collect_array<T: Default + Copy, const N: usize>(
    mut iter: impl Iterator<Item = T>,
) -> Option<[T; N]> {
    let mut out = [T::default(); N];
    for item in out.iter_mut() {
        *item = iter.next()?
    }
    Some(out)
}

/// A naive float parsing implementation, made to be less bloated than the FromStr impl for f64s.
///
/// Only supports basic decimal numbers that look like `-?\d+(.\d*)?`.
/// Does very litte sanity checking, may produce nonsensical results if given malformed strings.
pub(crate) fn parse_f64(s: &str) -> Result<f64, &'static str> {
    let (int, frac) = s
        .find('.')
        .map(|decimal_place| {
            let (int, frac) = s.split_at(decimal_place);
            let frac = &frac[1..];
            let frac = frac.trim_end_matches('0');

            (int, frac)
        })
        .unwrap_or((s, ""));

    const PARSE_ERR: &str = "float parse error";
    const OVERFLOW_ERR: &str = "float parse overflow";

    let decimal_place = frac.len() as u32;

    let whole: i64 = if int.is_empty() { Ok(0) } else { int.parse() }.map_err(|_| PARSE_ERR)?;
    let frac: u64 = if frac.is_empty() { Ok(0) } else { frac.parse() }.map_err(|_| PARSE_ERR)?;

    let mut frac = i64::try_from(frac).map_err(|_| OVERFLOW_ERR)?;
    if whole.is_negative() {
        frac = -frac;
    }
    let (whole, frac) = (whole as f64, frac as f64);

    let pow = 10u32.pow(decimal_place) as f64;
    let num = whole + frac / pow;
    Ok(num)
}

/// Shorthand for [parse_f64] as `f32`.
pub(crate) fn parse_f32(s: &str) -> Result<f32, AtParseErr> {
    Ok(parse_f64(s)? as f32)
}

/// A signal with that keeps track of the last value signaled.
pub struct StateSignal<M: RawMutex, T> {
    inner: blocking_mutex::Mutex<M, RefCell<StateSignalInner<T>>>,
}

struct StateSignalInner<T> {
    item: T,
    waker: WakerRegistration,
}

impl<M: RawMutex, T: Clone> StateSignal<M, T> {
    pub const fn new(item: T) -> Self {
        StateSignal {
            inner: blocking_mutex::Mutex::new(RefCell::new(StateSignalInner {
                item,
                waker: WakerRegistration::new(),
            })),
        }
    }

    /// Set the state of the signal and wake anyone calling [StateSignal::compare_wait].
    pub fn signal(&self, item: T) {
        self.inner.lock(|s| {
            let mut s = s.borrow_mut();
            s.item = item;
            s.waker.wake();
        })
    }

    /// Get the current state.
    pub fn current(&self) -> T {
        self.inner.lock(|s| s.borrow().item.clone())
    }

    /// Wait until someone calls [StateSignal::signal].
    pub async fn wait(&self) -> T {
        self.compare_wait(|_| true).await
    }

    /// Call `f` with the current state, and whenever the state changes, until `f` returns `true`.
    ///
    /// Returns the current state at which `f` returned true.
    pub fn compare_wait<'a>(
        &'a self,
        mut f: impl FnMut(&T) -> bool + 'a,
    ) -> impl Future<Output = T> + 'a {
        poll_fn(move |cx| {
            self.inner.lock(|s| {
                let mut s = s.borrow_mut();
                let satisfied = f(&s.item);
                if satisfied {
                    Poll::Ready(s.item.clone())
                } else {
                    let waker_register = &mut s.waker;
                    waker_register.register(cx.waker());
                    Poll::Pending
                }
            })
        })
    }
}

/// A fixed-capacity channel, backed by a ringbuffer.
///
/// This channel drops old messages if you try to send something while the channel is full.
pub struct RingChannel<M: RawMutex, T, const N: usize> {
    state: blocking_mutex::Mutex<M, RefCell<State<T, N>>>,
}

struct State<T, const N: usize> {
    overflowed: bool,
    waker: WakerRegistration,
    buf: Deque<T, N>,
}

/// The [RingChannel] overflowed since the last call to [RingChannel::recv].
pub struct Lagged;

impl<M: RawMutex, T: Debug, const N: usize> RingChannel<M, T, N> {
    /// Create a new [RingChannel].
    pub const fn new() -> Self {
        Self {
            state: blocking_mutex::Mutex::new(RefCell::new(State {
                overflowed: false,
                waker: WakerRegistration::new(),
                buf: Deque::new(),
            })),
        }
    }

    /// Send a message on the channel, immediately.
    ///
    /// If the channel is full, the oldest message will be dropped.
    pub fn send(&self, message: T) {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            if let Err(message) = s.buf.push_back(message) {
                let _ = s.buf.pop_front().expect("buffer is full");
                s.buf.push_back(message).expect("buffer is not full");
                s.overflowed = true;
            }
            s.waker.wake();
        });
    }

    /// Wait for a message to be received on the channel.
    ///
    /// Returns `Err(Lagged)` if the channel has overflowed since the last call to `recv`.
    /// Subsequent calls will return `Ok(T)` (assuming the channel didn't overflow again).
    pub fn recv(&self) -> impl Future<Output = Result<T, Lagged>> + '_ {
        poll_fn(|cx| {
            self.state.lock(|s| {
                let mut s = s.borrow_mut();
                if s.overflowed {
                    s.overflowed = false;
                    Poll::Ready(Err(Lagged))
                } else if let Some(message) = s.buf.pop_front() {
                    Poll::Ready(Ok(message))
                } else {
                    s.waker.register(cx.waker());
                    Poll::Pending
                }
            })
        })
    }

    /// Remove all messages that has not yet been received.
    pub fn clear(&self) {
        self.state.lock(|s| {
            let mut s = s.borrow_mut();
            s.buf.clear();
            s.overflowed = false;
        });
    }
}

#[cfg(test)]
mod test {
    use core::fmt::Write;
    use heapless::String;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    // we use ints here because quickcheck with floats is broken
    fn parse_f64(int: i16, frac: u16) {
        let mut s = String::<128>::new();
        write!(&mut s, "{int}.{frac}").expect("buffer overflow when stringifying float");
        let s = s.trim();

        let parsed = super::parse_f64(s).expect("failed to parse f64");
        let parsed2 = s.parse().unwrap();
        assert_eq!(parsed, parsed2);
    }

    #[quickcheck]
    // we use ints here because quickcheck with floats is broken
    fn parse_f32(int: i16, frac: u16) {
        let mut s = String::<128>::new();
        write!(&mut s, "{int}.{frac}").expect("buffer overflow when stringifying float");
        let s = s.trim();

        let parsed = super::parse_f32(s).expect("failed to parse f32");
        let parsed2 = s.parse().unwrap();
        assert_eq!(parsed, parsed2);
    }
}
