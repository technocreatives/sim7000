#![no_std]
#![feature(generic_associated_types)]

pub mod read;
pub mod write;
pub mod modem;

use core::future::Future;
use embassy::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::Channel,
};
use embedded_hal::digital::blocking::OutputPin;
use embedded_hal_async::digital::Wait;

pub trait SerialError {
    type Error: core::fmt::Debug;
}

#[derive(Debug)]
pub enum Error<S: core::fmt::Debug> {
    InvalidUtf8,
    SerialError(S),
    BufferOverflow,
}

impl<S: core::fmt::Debug> From<S> for Error<S> {
    fn from(value: S) -> Self {
        Error::SerialError(value)
    }
}

pub struct ModemContext {
    generic_response: Channel<CriticalSectionRawMutex, heapless::String<256>, 1>,
    tcp_1_channel: Channel<CriticalSectionRawMutex, heapless::Vec<u8, 256>, 2>,
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
    fn sleep<'a>(&mut self) -> Self::SleepFuture<'a>;
    fn wake<'a>(&mut self) -> Self::WakeFuture<'a>;
    fn reset<'a>(&mut self) -> Self::ResetFuture<'a>;
    fn state<'a>(&mut self) -> PowerState;
}

// cliff notes: one in flight command at a time, control via one-shots and signals. For getting data instead favor URCs that continually spam data
