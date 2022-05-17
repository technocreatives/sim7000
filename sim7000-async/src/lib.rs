#![no_std]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

pub mod read;
pub mod write;
pub mod modem;
pub mod tcp;
pub mod single_arc;

use core::{future::Future, sync::atomic::AtomicU8};
use embassy::{
    mutex::Mutex,
    blocking_mutex::{raw::CriticalSectionRawMutex},
    channel::{Channel, Signal},
};
use embedded_hal::digital::blocking::OutputPin;
use embedded_hal_async::digital::Wait;
use single_arc::SingletonArc;

pub trait SerialError {
    type Error: core::fmt::Debug;
}

#[derive(Debug)]
pub enum Error<S: core::fmt::Debug> {
    InvalidUtf8,
    SerialError(S),
    BufferOverflow,
    SimError,
    Timeout
}

impl<S: core::fmt::Debug> From<S> for Error<S> {
    fn from(value: S) -> Self {
        Error::SerialError(value)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum RegistrationStatus {
    NotRegistered,
    RegisteredHome,
    Searching,
    RegistrationDenied,
    Unknown,
    RegisteredRoaming,
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

// cliff notes: one in flight command at a time, control via one-shots and signals. For getting data instead favor URCs that continually spam data
