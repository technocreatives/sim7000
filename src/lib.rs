#![no_std]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]
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
mod slot;
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

pub trait BuildIo {
    type IO<'d>: SplitIo<'d>
    where
        Self: 'd;
    fn build(&mut self) -> Self::IO<'_>;
}

pub trait SplitIo<'a> {
    type Reader<'u>: Read
    where
        Self: 'u;
    type Writer<'u>: Write
    where
        Self: 'u;

    fn split(self) -> (Self::Reader<'a>, Self::Writer<'a>);
}

pub trait ModemPower {
    fn enable(&mut self) -> impl Future<Output = ()>;
    fn disable(&mut self) -> impl Future<Output = ()>;
    fn sleep(&mut self) -> impl Future<Output = ()>;
    fn wake(&mut self) -> impl Future<Output = ()>;
    fn reset(&mut self) -> impl Future<Output = ()>;
    fn state(&mut self) -> PowerState;
}

/// This macro sets up a modem for use, statically allocating pump tasks and channels.
///
/// You can call `Modem::new` directly if you want more control over initialization.
#[macro_export]
macro_rules! spawn_modem {
    // TODO: the "as" keyword hack is a bit weird.
    ($spawner:expr, $io_ty:ty as $io:expr, $power_pins:expr $(,)?) => {{
        static CONTEXT: ::sim7000_async::modem::ModemContext =
            ::sim7000_async::modem::ModemContext::new();

        let spawner: &Spawner = $spawner;
        let (modem, io_pump, tx_pump, rx_pump, drop_pump) =
            ::sim7000_async::modem::Modem::new($io, $power_pins, &CONTEXT)
                .await
                .expect("Failed to create Modem");

        mod __tasks {
            use super::*;
            use ::sim7000_async::pump_task;
            pump_task!(tx_pump, ::sim7000_async::pump::TxPump<'static>);
            pump_task!(rx_pump, ::sim7000_async::pump::RxPump<'static>);
            pump_task!(drop_pump, ::sim7000_async::pump::DropPump<'static>);
            pump_task!(io_pump, ::sim7000_async::pump::RawIoPump<'static, $io_ty>);
        }

        spawner.must_spawn(__tasks::tx_pump(tx_pump));
        spawner.must_spawn(__tasks::rx_pump(rx_pump));
        spawner.must_spawn(__tasks::drop_pump(drop_pump));
        spawner.must_spawn(__tasks::io_pump(io_pump));

        modem
    }};
}
