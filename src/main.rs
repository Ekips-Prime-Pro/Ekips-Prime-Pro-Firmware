//! Ekips Prime Pro firmware for the LEGO SPIKE Prime hub.
//!
//! **Untested on hardware.**

#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_stm32::init(Default::default());
    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        cortex_m::asm::nop();
    }
}
