use core::sync::atomic::{AtomicBool, Ordering};

use embassy::{channel::{Channel, Signal}, blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use heapless::{String, Vec};

use crate::{single_arc::SingletonArc, RegistrationStatus, tcp::TcpMessage};

pub type TcpRxChannel = Channel<CriticalSectionRawMutex, Vec<u8, 365>, 8>;

pub struct ModemContext<T> {
    pub(crate) generic_response: Channel<CriticalSectionRawMutex, String<256>, 1>,
    pub(crate) tcp: TcpContext,
    pub(crate) registration_events: Signal<RegistrationStatus>,
    pub(crate) transmit: SingletonArc<Mutex<CriticalSectionRawMutex, T>>,
}


impl<R> ModemContext<R> {
    pub const fn new() -> Self {
        ModemContext { generic_response: Channel::new(), tcp: TcpContext::new(), registration_events: Signal::new(), transmit: SingletonArc::new() }
    }
}

pub struct TcpContext {
    pub(crate) rx: [TcpRxChannel; 8],
    pub(crate) events: [Channel<CriticalSectionRawMutex, TcpMessage, 4>; 8],
    pub(crate) slots: [AtomicBool; 8],
}

impl TcpContext {
    pub const fn new() -> Self {
        TcpContext { rx: [Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new()], events: [Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new(), Channel::new()], slots: [AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true), AtomicBool::new(true)] }
    }
}

impl TcpContext {
    pub fn claim(&self) -> Option<TcpToken> {
        for i in 0..7 {
            if self.slots[i].fetch_and(false, Ordering::Relaxed) {
                return Some(TcpToken {
                    ordinal: i,
                    rx: &self.rx[i],
                    events: &self.events[i],
                    slot: &self.slots[i],
                });
            }
        }

        None
    }
}


pub struct TcpToken<'c> {
    ordinal: usize,
    rx: &'c TcpRxChannel,
    events: &'c Channel<CriticalSectionRawMutex, TcpMessage, 4>,
    slot: &'c AtomicBool,
}

impl<'c> TcpToken<'c> {
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    pub fn rx(&self) -> &'c TcpRxChannel {
        self.rx
    }

    pub fn events(&self) -> &'c Channel<CriticalSectionRawMutex, TcpMessage, 4> {
        self.events
    }
}

impl<'c> Drop for TcpToken<'c> {
    fn drop(&mut self) {
        self.slot.fetch_or(true, Ordering::Relaxed);
    }
}