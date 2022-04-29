#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_nrf::{
    interrupt::{self, UARTE0_UART0},
    uarte, Peripherals,
};
use rtt_target::{rprintln, rtt_init_print};
use sim7000_async::{ModemContext, ModemPower, modem::{Modem, RxPump}};
use core::future::Future;

extern crate panic_reset;

static MODEM_CONTEXT: ModemContext = ModemContext::new();

#[embassy::main]
async fn main(_spawner: Spawner, p: Peripherals) {
    rtt_init_print!();
    let irq = interrupt::take!(UARTE0_UART0);
    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;
    let (tx, rx) = embassy_nrf::uarte::UarteWithIdle::new_with_rtscts(
        p.UARTE0, p.TIMER0, p.PPI_CH0, p.PPI_CH1, irq, p.P0_20, p.P0_24, p.P0_08, p.P0_11, config,
    )
    .split();

    let (modem, pump) = sim7000_async::modem::Modem::new(AppUarteRead(rx), AppUarteWrite(tx), ModemPowerPins, &MODEM_CONTEXT).await.unwrap();
    loop {
        Timer::after(Duration::from_millis(300)).await;
        rprintln!("PING");
    }
}

#[repr(transparent)]
struct AppUarteRead<'d>(embassy_nrf::uarte::UarteRxWithIdle<'d, embassy_nrf::peripherals::UARTE0, embassy_nrf::peripherals::TIMER0>);

impl<'d> sim7000_async::SerialError for AppUarteRead<'d> {
    type Error = embassy_nrf::uarte::Error;
}

impl<'d> sim7000_async::read::Read for AppUarteRead<'d> {
    type ReadFuture<'a> = impl Future<Output = Result<usize, Self::Error>> + 'a
    where
    Self: 'a;

    type ReadExactFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> Self::ReadExactFuture<'a> {
        self.0.read(buf)
    }

    fn read<'a>(&'a mut self, read: &'a mut [u8]) -> Self::ReadFuture<'a> {
        self.0.read_until_idle(read)
    }
}

struct AppUarteWrite<'d>(embassy_nrf::uarte::UarteTx<'d, embassy_nrf::peripherals::UARTE0>);

impl<'d> sim7000_async::SerialError for AppUarteWrite<'d> {
    type Error = embassy_nrf::uarte::Error;
}

impl<'d> sim7000_async::write::Write for AppUarteWrite<'d> {
    type WriteAllFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    type FlushFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a
    where
        Self: 'a;

    fn write_all<'a>(&'a mut self, words: &'a [u8]) -> Self::WriteAllFuture<'a> {
        self.0.write(words)
    }

    fn flush<'a>(&'a mut self) -> Self::FlushFuture<'a> {
        async { Ok(()) }
    }
}

struct ModemPowerPins;

impl ModemPower for ModemPowerPins {
    type EnableFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;
    type DisableFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;
    type SleepFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;
    type WakeFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;
    type ResetFuture<'a> = impl Future<Output = ()> + 'a
    where
        Self: 'a;

    fn enable<'a>(&'a mut self) -> Self::EnableFuture<'a> {
        async { () }
    }

    fn disable<'a>(&'a mut self) -> Self::DisableFuture<'a> {
        async { () }
    }

    fn sleep<'a>(&mut self) -> Self::SleepFuture<'a> {
        async { () }
    }

    fn wake<'a>(&mut self) -> Self::WakeFuture<'a> {
        async { () }
    }

    fn reset<'a>(&mut self) -> Self::ResetFuture<'a> {
        async { () }
    }

    fn state<'a>(&mut self) -> sim7000_async::PowerState {
        sim7000_async::PowerState::Off
    }
}