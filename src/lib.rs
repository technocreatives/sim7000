#![no_std]
#![allow(clippy::unnecessary_lazy_evaluations)]
#![allow(clippy::single_component_path_imports)]
// large enum variants are unavoidable in no_std, since we can't box things
#![allow(clippy::large_enum_variant, clippy::result_large_err)]

// TODO: at_command should probably be moved to its own crate
pub mod at_command;
mod drop;
mod error;
pub mod gnss;
pub mod modem;
pub mod pump;
pub mod read;
pub mod slot;
pub mod tcp;
mod util;
pub mod voltage;

pub use util::*;

#[cfg(all(feature = "log", feature = "defmt"))]
compile_error!("'log' and 'defmt' features are mutually exclusive");
#[cfg(not(any(feature = "log", feature = "defmt")))]
compile_error!("please enable a logging feature, e.g. 'log' or 'defmt'");
#[cfg(feature = "defmt")]
pub(crate) use defmt as log;
use embedded_io_async::{Read, Write};
#[cfg(feature = "log")]
pub(crate) use log;

pub use error::Error;
pub use modem::power::PowerState;

use core::future::Future;

pub trait SerialError {
    type Error: core::fmt::Debug;
}

/// This trait is for building a `BuildIo::IO` that implements [SplitIo].
///
/// The purpose of the trait is to let the use of this library to plug in UART driver types from
/// whatever HAL they're using. The trait provides the ability for the `RawIoPump` to
/// construct/destruct (enable/disable) the UART IO.
pub trait BuildIo {
    type IO<'d>: SplitIo
    where
        Self: 'd;

    /// Construct a `BuildIo::IO` that implements [SplitIo].
    fn build(&mut self) -> Self::IO<'_>;
}

/// Split self into a reader and a writer. See documentation on [SplitIo::split].
pub trait SplitIo: Sized {
    type Reader<'u>: Read
    where
        Self: 'u;
    type Writer<'u>: Write
    where
        Self: 'u;

    /// Split self into a reader and a writer.
    ///
    /// **NOTE**: This method **must not** be called with None. Implementations are allowed to panic
    /// on None. This method takes a `&mut Option<Self>` so that implementations can choose to
    /// borrow `Self`, or to take ownership of it. This is to maintain compatibility with as many
    /// HALs as possible.
    fn split(this: &mut Option<Self>) -> (Self::Reader<'_>, Self::Writer<'_>);
}

pub trait ModemPower {
    /// Power on the modem, e.g. by pulling on the modem power key pin.
    fn enable(&mut self) -> impl Future<Output = ()>;

    /// Power off the modem, e.g. by pulling on the modem power key pin.
    fn disable(&mut self) -> impl Future<Output = ()>;

    /// Put the modem to sleep, e.g. by pulling on the modem DTR pin.
    fn sleep(&mut self) -> impl Future<Output = ()>;

    /// Wake the modem from sleep, e.g. by pulling on the modem DTR pin.
    fn wake(&mut self) -> impl Future<Output = ()>;

    /// Reset the modem, e.g. by pulling on the modem reset pin.
    fn reset(&mut self) -> impl Future<Output = ()>;

    /// Get the curren power state of the modem, e.g. by looking at the modem status pin.
    fn state(&mut self) -> PowerState;
}

/// This macro sets up a modem for use, statically allocating pump tasks and channels.
///
/// You can call `Modem::new` directly if you want more control over initialization.
///
/// Here's an abridged example, see `samples` for a more complete example:
///
/// ```ignore
/// use embassy_executor::Spawner;
/// use sim7000_async::{BuildIo, ModemPower};
///
/// let spawner: Spawner;
///
/// struct MyUart { /* --snip-- */}
/// impl BuildIo for MyUart { /* --snip-- */ }
/// let uart: MyUart;
///
/// struct MyPowerPins { /* --snip-- */ }
/// impl ModemPower for MyPowerPins { /* --snip-- */ }
/// let power_pins: MyPowerPins;
///
/// spawn_modem! {
///   &spawner,
///   MyUart as uart,
///   power_pins,
///   tcp_slots: 5, // optional argument. may not exceed max.
/// };
/// ```
///
/// Note that `tcp_slots` is an optional argument that sets how many TCP sockets may be open
/// concurrently. Default is [MAX_TCP_SLOTS](tcp::MAX_TCP_SLOTS), and the value may not exceed this.
/// Each slot consumes a approx [TCP_RX_BUF_LEN](tcp::TCP_RX_BUF_LEN) bytes of RAM.
#[macro_export]
macro_rules! spawn_modem {
    (
        $spawner:expr,
        $io_ty:ty as $io:expr,
        $power_pins:expr,
        tcp_slots: $tcp_slots:expr $(,)?
     ) => {{
        const __TCP_SLOT_COUNT: usize = $tcp_slots;

        const ASSERT_TCP_SLOTS_WITHIN_LIMIT: usize = ::sim7000_async::tcp::MAX_TCP_SLOTS - __TCP_SLOT_COUNT;

        static SIM7000_TCP_SLOTS: [::sim7000_async::slot::Slot<::sim7000_async::modem::TcpSlot>; __TCP_SLOT_COUNT] = {
            use ::sim7000_async::{slot::Slot, modem::TcpSlot};
            #[allow(clippy::declare_interior_mutable_const)]
            const NEW_SLOT: Slot<TcpSlot> = Slot::new(TcpSlot::new());
            [NEW_SLOT; __TCP_SLOT_COUNT]
        };

        static SIM7000_CONTEXT: ::sim7000_async::modem::ModemContext =
            ::sim7000_async::modem::ModemContext::new(::sim7000_async::modem::TcpContext::new(&SIM7000_TCP_SLOTS));

        let spawner: &Spawner = $spawner;
        let (modem, io_pump, tx_pump, rx_pump, drop_pump) =
            ::sim7000_async::modem::Modem::new($io, $power_pins, &SIM7000_CONTEXT)
                .await
                .expect("Failed to create Modem");

        mod __sim7000_tasks {
            use super::*;
            use ::sim7000_async::pump_task;
            pump_task!(tx_pump, ::sim7000_async::pump::TxPump<'static>);
            pump_task!(rx_pump, ::sim7000_async::pump::RxPump<'static>);
            pump_task!(drop_pump, ::sim7000_async::pump::DropPump<'static>);
            pump_task!(io_pump, ::sim7000_async::pump::RawIoPump<'static, $io_ty>);
        }

        spawner.must_spawn(__sim7000_tasks::tx_pump(tx_pump));
        spawner.must_spawn(__sim7000_tasks::rx_pump(rx_pump));
        spawner.must_spawn(__sim7000_tasks::drop_pump(drop_pump));
        spawner.must_spawn(__sim7000_tasks::io_pump(io_pump));

        modem
    }};
    (
        $spawner:expr,
        $io_ty:ty as $io:expr,
        $power_pins:expr $(,)?
     ) => {spawn_modem!($spawner, $io_ty as $io, $power_pins, tcp_slots: 8)};
}
