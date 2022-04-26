#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_nrf::Peripherals;
use rtt_target::{rtt_init_print, rprintln};

extern crate panic_reset;

#[embassy::main]
async fn main(_spawner: Spawner, _p: Peripherals) {
    rtt_init_print!();

    loop {
        Timer::after(Duration::from_millis(300)).await;
        rprintln!("PING");
    }
}
