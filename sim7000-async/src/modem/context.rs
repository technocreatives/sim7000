use embassy::{channel::{Channel, Signal}, blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use heapless::{String, Vec};

use crate::{single_arc::SingletonArc, RegistrationStatus};

pub struct ModemContext<T> {
    pub(crate) generic_response: Channel<CriticalSectionRawMutex, String<256>, 1>,
    pub(crate) tcp_1_channel: Channel<CriticalSectionRawMutex, Vec<u8, 365>, 8>,
    pub(crate) registration_events: Signal<RegistrationStatus>,
    pub(crate) transmit: SingletonArc<Mutex<CriticalSectionRawMutex, T>>,
}

impl<R> ModemContext<R> {
    pub const fn new() -> Self {
        ModemContext { generic_response: Channel::new(), tcp_1_channel: Channel::new(), registration_events: Signal::new(), transmit: SingletonArc::new() }
    }
}