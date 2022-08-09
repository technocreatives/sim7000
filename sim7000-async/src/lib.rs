#![no_std]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

pub mod modem;
pub mod read;
pub mod single_arc;
pub mod tcp;
pub mod write;

use core::future::Future;

pub trait SerialError {
    type Error: core::fmt::Debug;
}

#[derive(Debug)]
pub enum Error {
    InvalidUtf8,
    BufferOverflow,
    SimError,
    Timeout,
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
