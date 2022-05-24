#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
mod logger;

use core::future::Future;
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_nrf::{
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull},
    interrupt::{self, UARTE0_UART0},
    uarte, Peripherals,
};
use rtt_target::{rprintln, rtt_init_print};
use sim7000_async::{
    modem::{Modem, ModemContext, RxPump},
    write::Write,
    ModemPower, PowerState, read::Read,
};

extern crate panic_rtt_target;

static MODEM_CONTEXT: ModemContext<AppUarteWrite<'static>> = ModemContext::new();

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    rtt_init_logger!(BlockIfFull);
    logger::set_level(LevelFilter::Debug);
    let irq = interrupt::take!(UARTE0_UART0);
    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;
    let (tx, rx) = embassy_nrf::uarte::UarteWithIdle::new_with_rtscts(
        p.UARTE0, p.TIMER0, p.PPI_CH0, p.PPI_CH1, irq, p.P0_20, p.P0_24, p.P0_08, p.P0_11, config,
    )
    .split();

    let power_pins = ModemPowerPins {
        status: Input::new(p.P1_07.degrade(), Pull::None),
        power_key: Output::new(p.P1_03.degrade(), Level::Low, OutputDrive::Standard),
        dtr: Output::new(p.P1_05.degrade(), Level::Low, OutputDrive::Standard),
        reset: Output::new(p.P1_04.degrade(), Level::Low, OutputDrive::Standard),
        ri: Input::new(p.P1_15.degrade(), Pull::Up),
    };
    let (mut modem, pump) = sim7000_async::modem::Modem::new(
        AppUarteRead(rx),
        AppUarteWrite(tx),
        power_pins,
        &MODEM_CONTEXT,
    )
    .await
    .unwrap();
    spawner.must_spawn(rx_pump(pump));
    modem.init().await.unwrap();
    modem.activate().await.unwrap();
    Timer::after(Duration::from_millis(5000)).await;

    let mut tcp = modem.connect_tcp("tcpbin.com", 4242).await;
    tcp.write_all(b"\r\nFOOBARBAZBOPSHOP\r\n")
        .await
        .unwrap();
        rprintln!("READING");
        
    let mut buf = [0u8; 128];
    loop {
        let amount = tcp.read(&mut buf).await.unwrap();
        if amount == 0 {
            break;
        }

        rprintln!("{}", core::str::from_utf8(&buf[..amount]).unwrap());

    }
    loop {
        Timer::after(Duration::from_millis(300)).await;
        rprintln!("PING");
    }
}

#[embassy::task]
async fn rx_pump(mut pump: RxPump<'static, AppUarteRead<'static>>) {
    loop {
        if let Err(err) = pump.pump().await {
            log::error!("issue running modem receiver pump {:?}", err);
        }
    }
}

#[repr(transparent)]
struct AppUarteRead<'d>(
    embassy_nrf::uarte::UarteRxWithIdle<
        'd,
        embassy_nrf::peripherals::UARTE0,
        embassy_nrf::peripherals::TIMER0,
    >,
);

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
        async move {
            rprintln!("READ UNTIL IDLE");
            let result = match embassy::time::with_timeout(Duration::from_millis(1000), self.0.read_until_idle(read)).await {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => return Err(err),
                Err(_) => 0,
            };
            rprintln!("READ DONE");
            Ok(result)
        }
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

pub struct ModemPowerPins {
    pub status: Input<'static, AnyPin>,
    pub power_key: Output<'static, AnyPin>,
    pub dtr: Output<'static, AnyPin>,
    pub reset: Output<'static, AnyPin>,
    pub ri: Input<'static, AnyPin>,
}

impl ModemPowerPins {
    async fn press_power_key(&mut self, millis: u32) {
        self.power_key.set_low();
        Timer::after(Duration::from_millis(100)).await;

        //based on schematics the power key is active low on MCU side
        self.power_key.set_high();
        Timer::after(Duration::from_millis(millis as u64)).await;
        self.power_key.set_low();
        log::info!("power key pressed for {}ms", millis);
    }

    async fn reset(&mut self) {}

    fn is_enabled(&mut self) -> bool {
        let status = self.status.is_high();
        log::info!(
            "modem is currently {}",
            if status { "enabled" } else { "disabled" }
        );
        status
    }
}

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
        async {
            log::info!("enabling modem");
            //poor datasheet gives only min, not max timeout
            if self.is_enabled() {
                log::info!("modem was enabled already");
                return;
            }
            self.press_power_key(1100).await;
            while self.status.is_low() {
                Timer::after(Duration::from_millis(100)).await;
            }
            log::info!("modem enabled");
        }
    }

    fn disable<'a>(&'a mut self) -> Self::DisableFuture<'a> {
        async {
            log::info!("disabling modem");
            //poor datasheet gives only min, not max timeout
            if !self.is_enabled() {
                log::info!("modem was disabled already");
                return;
            }
            self.press_power_key(1300).await;
            while self.status.is_high() {
                Timer::after(Duration::from_millis(100)).await;
            }
            log::info!("modem disabled");
        }
    }

    fn sleep<'a>(&'a mut self) -> Self::SleepFuture<'a> {
        async {
            self.dtr.set_high();
        }
    }

    fn wake<'a>(&'a mut self) -> Self::WakeFuture<'a> {
        async {
            self.dtr.set_low();
        }
    }

    fn reset<'a>(&'a mut self) -> Self::ResetFuture<'a> {
        async {
            self.reset.set_high();
            // Reset pin needs to be held low for 252ms. Wait for 300ms to ensure it works.
            Timer::after(Duration::from_millis(300)).await;
            self.reset.set_low();
        }
    }

    fn state<'a>(&'a mut self) -> sim7000_async::PowerState {
        match self.status.is_high() {
            true => PowerState::On,
            false => PowerState::Off,
        }
    }
}
