#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

mod example;
mod logger;

use core::future::Future;
use embassy_executor::Spawner;
use embassy_nrf::{
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull},
    interrupt, uarte,
};
use embassy_time::{with_timeout, Duration, Timer};
use sim7000_async::{modem::ModemContext, spawn_modem, ModemPower, PowerState};

//#[cfg(debug_assertions)]
extern crate panic_rtt_target;

type Modem = sim7000_async::modem::Modem<'static, ModemPowerPins>;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    rtt_init_logger!(BlockIfFull);
    logger::set_level(LevelFilter::Debug);
    log::info!("Started");

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

    let mut modem = spawn_modem!(
        &spawner,
        AppUarteRead<'static> as AppUarteRead(rx),
        AppUarteWrite<'static> as AppUarteWrite(tx),
        power_pins
    );

    log::info!("Initializing modem");
    modem.init().await.unwrap();

    log::info!("Activating modem");
    modem.activate().await.unwrap();

    log::info!("sleeping 1s");
    Timer::after(Duration::from_millis(1000)).await;

    match modem.claim_voltage_warner().await {
        Some(warner) => spawner.must_spawn(example::voltage_warn(warner)),
        None => log::error!("Failed to take VoltageWarner handle"),
    }

    match modem.claim_gnss().await {
        Ok(Some(gnss)) => spawner.must_spawn(example::gnss(gnss)),
        Ok(None) => log::error!("Failed to take GNSS handle"),
        Err(e) => log::error!("Failed to subscribe to GNSS: {e:?}"),
    }

    log::info!("sleeping 5s");
    Timer::after(Duration::from_millis(5000)).await;

    for _ in 0..100 {
        log::info!("sleeping 1s");
        Timer::after(Duration::from_millis(1000)).await;

        log::info!("spawning tasks");
        let tcpbin_handle = example::ping_tcpbin(&spawner, &mut modem)
            .await
            .map_err(|e| log::error!("Failed to spawn ping_tcpbin: {e:?}"))
            .ok();

        let qotd_handle = example::get_quote_of_the_day(&spawner, &mut modem)
            .await
            .map_err(|e| log::error!("Failed to spawn Quote of the Day: {e:?}"))
            .ok();

        log::info!("await tcpbin");
        if let Some(handle) = tcpbin_handle {
            if let Err(e) = handle.await {
                log::error!("ping_tcpbin failed: {e:?}");
            }
        }

        log::info!("await QotD");
        if let Some(handle) = qotd_handle {
            if let Err(e) = handle.await {
                log::error!("get QotD failed: {e:?}");
            }
        }
    }

    log::info!("main() finished");
    loop {
        Timer::after(Duration::from_millis(1000)).await;
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
            log::trace!("Read until idle");
            let n = match with_timeout(Duration::from_millis(1000), self.0.read_until_idle(read))
                .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => return Err(err),
                Err(_) => 0,
            };

            if n > 0 {
                log::debug!("Read {n} bytes from modem uarte");
            }

            Ok(n)
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

    fn flush(&mut self) -> Self::FlushFuture<'_> {
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

    fn is_enabled(&self) -> bool {
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

    fn enable(&mut self) -> Self::EnableFuture<'_> {
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

    fn disable(&mut self) -> Self::DisableFuture<'_> {
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

    fn sleep(&mut self) -> Self::SleepFuture<'_> {
        async {
            self.dtr.set_high();
        }
    }

    fn wake(&mut self) -> Self::WakeFuture<'_> {
        async {
            self.dtr.set_low();
        }
    }

    fn reset(&mut self) -> Self::ResetFuture<'_> {
        async {
            self.reset.set_high();
            // Reset pin needs to be held low for 252ms. Wait for 300ms to ensure it works.
            Timer::after(Duration::from_millis(300)).await;
            self.reset.set_low();
        }
    }

    fn state(&mut self) -> sim7000_async::PowerState {
        match self.status.is_high() {
            true => PowerState::On,
            false => PowerState::Off,
        }
    }
}
