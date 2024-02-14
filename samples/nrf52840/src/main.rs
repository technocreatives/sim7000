#![no_std]
#![no_main]

mod example;

use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    buffered_uarte::{self, BufferedUarte, BufferedUarteRx, BufferedUarteTx},
    gpio::{AnyPin, Input, Level, Output, OutputDrive, Pin, Pull},
    peripherals::{PPI_CH1, PPI_CH2, PPI_GROUP0, TIMER0, UARTE0},
    uarte,
};
use embassy_time::{Duration, Timer};
use sim7000_async::{spawn_modem, BuildIo, ModemPower, PowerState, SplitIo};

use defmt_rtt as _; // linker shenanigans
use panic_rtt_target as _;

type Modem = sim7000_async::modem::Modem<'static, ModemPowerPins>;

bind_interrupts!(struct Irqs {
    UARTE0_UART0 => buffered_uarte::InterruptHandler<UARTE0>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    defmt::error!("log-level: error");
    defmt::warn!("log-level: warn");
    defmt::info!("log-level: info");
    defmt::debug!("log-level: debug");
    defmt::trace!("log-level: trace");

    defmt::info!("Started");

    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::EXCLUDED;
    config.baudrate = uarte::Baudrate::BAUD115200;

    let power_pins = ModemPowerPins {
        status: Input::new(p.P1_07.degrade(), Pull::None),
        power_key: Output::new(p.P1_03.degrade(), Level::Low, OutputDrive::Standard),
        dtr: Output::new(p.P1_05.degrade(), Level::Low, OutputDrive::Standard),
        reset: Output::new(p.P1_04.degrade(), Level::Low, OutputDrive::Standard),
        ri: Input::new(p.P1_15.degrade(), Pull::Up),
    };

    let mut modem = spawn_modem!(
        &spawner,
        UarteComponents as UarteComponents {
            uarte: p.UARTE0,
            timer: p.TIMER0,
            ppi_ch1: p.PPI_CH1,
            ppi_ch2: p.PPI_CH2,
            ppi_gr: p.PPI_GROUP0,
            rxd: p.P0_20.degrade(),
            txd: p.P0_24.degrade(),
            rts: p.P0_11.degrade(),
            cts: p.P0_08.degrade(),
            config,
            tx_buffer: [0; 64],
            rx_buffer: [0; 64],
        },
        power_pins,
        tcp_slots: 3,
    );

    defmt::info!("Initializing modem");
    modem.init(Default::default()).await.unwrap();

    defmt::info!("Activating modem");
    modem.activate().await.unwrap();

    defmt::info!("sleeping 1s");
    Timer::after(Duration::from_millis(1000)).await;

    match modem.claim_voltage_warner().await {
        Some(warner) => spawner.must_spawn(example::voltage_warn(warner)),
        None => defmt::error!("Failed to take VoltageWarner handle"),
    }

    match modem.claim_gnss().await {
        Ok(Some(gnss)) => spawner.must_spawn(example::gnss(gnss)),
        Ok(None) => defmt::error!("Failed to take GNSS handle"),
        Err(e) => defmt::error!("Failed to subscribe to GNSS: {:?}", e),
    }

    defmt::info!("sleeping 5s");
    Timer::after(Duration::from_millis(5000)).await;

    defmt::info!("Operator: {:?}", modem.query_operator_info().await);
    defmt::info!("ICCID: {:?}", modem.query_iccid().await);
    defmt::info!("Signal quality: {:?}", modem.query_signal().await);
    defmt::info!("System info: {:?}", modem.query_system_info().await);
    defmt::info!(
        "Firmware version: {:?}",
        modem.query_firmware_version().await,
    );

    for _ in 0..100 {
        defmt::info!("sleeping 1s");
        Timer::after(Duration::from_millis(1000)).await;

        defmt::info!("spawning tasks");
        let tcpbin_handle = example::ping_tcpbin(&spawner, &mut modem)
            .await
            .map_err(|e| defmt::error!("Failed to spawn ping_tcpbin: {:?}", e))
            .ok();

        let qotd_handle = example::get_quote_of_the_day(&spawner, &mut modem)
            .await
            .map_err(|e| defmt::error!("Failed to spawn Quote of the Day: {:?}", e))
            .ok();

        defmt::info!("await tcpbin");
        if let Some(handle) = tcpbin_handle {
            if let Err(e) = handle.await {
                defmt::error!("ping_tcpbin failed: {:?}", e);
            }
        }

        defmt::info!("await QotD");
        if let Some(handle) = qotd_handle {
            if let Err(e) = handle.await {
                defmt::error!("get QotD failed: {:?}", e);
            }
        }
    }

    defmt::info!("main() finished");
    loop {
        Timer::after(Duration::from_millis(1000)).await;
    }
}

struct UarteComponents {
    pub uarte: UARTE0,
    pub timer: TIMER0,
    pub ppi_ch1: PPI_CH1,
    pub ppi_ch2: PPI_CH2,
    pub ppi_gr: PPI_GROUP0,
    pub rxd: AnyPin,
    pub txd: AnyPin,
    pub rts: AnyPin,
    pub cts: AnyPin,
    pub config: uarte::Config,
    pub tx_buffer: [u8; 64],
    pub rx_buffer: [u8; 64],
}

impl BuildIo for UarteComponents {
    type IO<'d> = AppUarte<'d>
    where
    Self: 'd;

    fn build(&mut self) -> Self::IO<'_> {
        AppUarte(BufferedUarte::new_with_rtscts(
            &mut self.uarte,
            &mut self.timer,
            &mut self.ppi_ch1,
            &mut self.ppi_ch2,
            &mut self.ppi_gr,
            Irqs,
            &mut self.rxd,
            &mut self.txd,
            &mut self.cts,
            &mut self.rts,
            self.config.clone(),
            &mut self.rx_buffer,
            &mut self.tx_buffer,
        ))
    }
}

struct AppUarte<'d>(
    embassy_nrf::buffered_uarte::BufferedUarte<
        'd,
        embassy_nrf::peripherals::UARTE0,
        embassy_nrf::peripherals::TIMER0,
    >,
);

impl<'d> SplitIo for AppUarte<'d> {
    type Reader<'u> = BufferedUarteRx<'u, 'd, UARTE0, TIMER0>
    where
    Self: 'u;

    type Writer<'u> = BufferedUarteTx<'u, 'd, UARTE0, TIMER0>
    where
    Self: 'u;

    fn split(this: &mut Option<Self>) -> (Self::Reader<'_>, Self::Writer<'_>) {
        this.as_mut().unwrap().0.split()
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
        defmt::info!("power key pressed for {}ms", millis);
    }

    fn is_enabled(&self) -> bool {
        let status = self.status.is_high();
        defmt::info!(
            "modem is currently {}",
            if status { "enabled" } else { "disabled" }
        );
        status
    }
}

impl ModemPower for ModemPowerPins {
    async fn enable(&mut self) {
        defmt::info!("enabling modem");
        //poor datasheet gives only min, not max timeout
        if self.is_enabled() {
            defmt::info!("modem was enabled already");
            return;
        }
        self.press_power_key(1100).await;
        while self.status.is_low() {
            Timer::after(Duration::from_millis(100)).await;
        }
        defmt::info!("modem enabled");
    }

    async fn disable(&mut self) {
        defmt::info!("disabling modem");
        //poor datasheet gives only min, not max timeout
        if !self.is_enabled() {
            defmt::info!("modem was disabled already");
            return;
        }
        self.press_power_key(1300).await;
        while self.status.is_high() {
            Timer::after(Duration::from_millis(100)).await;
        }
        defmt::info!("modem disabled");
    }

    async fn sleep(&mut self) {
        self.dtr.set_high();
    }

    async fn wake(&mut self) {
        self.dtr.set_low();
    }

    async fn reset(&mut self) {
        self.reset.set_high();
        // Reset pin needs to be held low for 252ms. Wait for 300ms to ensure it works.
        Timer::after(Duration::from_millis(300)).await;
        self.reset.set_low();
    }

    fn state(&mut self) -> sim7000_async::PowerState {
        match self.status.is_high() {
            true => PowerState::On,
            false => PowerState::Off,
        }
    }
}
