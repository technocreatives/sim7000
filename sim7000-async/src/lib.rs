#![no_std]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

// TODO: at_command should probably be moved to its own crate
pub mod at_command;
mod drop;
pub mod gnss;
pub mod modem;
pub mod pump;
pub mod read;
pub mod single_arc;
mod slot;
pub mod tcp;
mod util;
pub mod voltage;
pub mod write;

use at_command::response::SimError;
use core::future::Future;

pub trait SerialError {
    type Error: core::fmt::Debug;
}

#[derive(Debug)]
pub enum Error {
    InvalidUtf8,
    BufferOverflow,
    Sim(SimError),
    Timeout,
    Serial,
}

#[derive(PartialEq, Eq)]
pub enum PowerState {
    On,
    Off,
    Sleeping,
}

pub trait ModemPower {
    type EnableFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;
    type DisableFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;
    type SleepFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;
    type WakeFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;
    type ResetFuture<'a>: Future<Output = ()> + 'a
    where
        Self: 'a;
    fn enable<'a>(&'a mut self) -> Self::EnableFuture<'a>;
    fn disable<'a>(&'a mut self) -> Self::DisableFuture<'a>;
    fn sleep<'a>(&'a mut self) -> Self::SleepFuture<'a>;
    fn wake<'a>(&'a mut self) -> Self::WakeFuture<'a>;
    fn reset<'a>(&'a mut self) -> Self::ResetFuture<'a>;
    fn state<'a>(&'a mut self) -> PowerState;
}

/// This macro sets up a modem for use, statically allocating pump tasks and channels.
///
/// You can call `Modem::new` directly if you want more control over initialization.
#[macro_export]
macro_rules! spawn_modem {
    // TODO: the "as" keyword hack is a bit weird.
    ($spawner:expr, $read_ty:ty as $read:expr, $write_ty:ty as $write:expr, $power_pins:expr) => {{
        static CONTEXT: ModemContext = ::sim7000_async::modem::ModemContext::new();

        let spawner: &Spawner = $spawner;
        let (modem, tx_pump, rx_pump, drop_pump) =
            ::sim7000_async::modem::Modem::new($read, $write, $power_pins, &CONTEXT)
                .await
                .expect("Failed to create Modem");

        mod __tasks {
            use super::*;
            use ::sim7000_async::pump_task;
            pump_task!(tx_pump, ::sim7000_async::pump::TxPump<'static, $write_ty>);
            pump_task!(rx_pump, ::sim7000_async::pump::RxPump<'static, $read_ty>);
            pump_task!(drop_pump, ::sim7000_async::pump::DropPump<'static>);
        }

        spawner.must_spawn(__tasks::tx_pump(tx_pump));
        spawner.must_spawn(__tasks::rx_pump(rx_pump));
        spawner.must_spawn(__tasks::drop_pump(drop_pump));

        modem
    }};
}
