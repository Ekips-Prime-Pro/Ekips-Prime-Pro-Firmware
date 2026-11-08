//! Ekips Prime Pro firmware for the LEGO SPIKE Prime hub.
//!
//! Start-up order:
//!
//! 1. `__pre_init` (assembly, [`drivers::power`]): power latch PA13 HIGH.
//! 2. Embassy init with the Pybricks clock tree ([`bsp::clock_config`]).
//! 3. Drivers are started as Embassy tasks.
//!
//! **Untested on hardware.**

#![no_std]
#![no_main]

mod bsp;
mod drivers;

use defmt_rtt as _;
use embassy_executor::Spawner;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    drivers::power::hold();
    let p = embassy_stm32::init(bsp::clock_config());
    defmt::info!(
        "Ekips Prime Pro firmware started (USB powered: {})",
        drivers::power::usb_powered()
    );

    spawner.spawn(
        drivers::analog::task(drivers::analog::AnalogResources {
            adc: p.ADC1,
            battery_current: p.PC0,
            battery_voltage: p.PC1,
            battery_ntc: p.PB0,
            usb_charge_current: p.PA3,
            ladder_center: p.PC4,
            ladder_buttons: p.PA1,
        })
        .unwrap(),
    );
    spawner.spawn(drivers::buttons::task().unwrap());

    drivers::buttons::POWER_OFF_REQUEST.wait().await;
    drivers::power::power_off();
}

/// Panic: put actuators into a safe state, then power off after a while so
/// a crashed hub does not drain the battery. On USB power it stays on.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    bsp::safe_state();
    defmt::error!("panic: {}", defmt::Display2Format(info));
    // ~10 s (the delay is in CPU cycles).
    cortex_m::asm::delay(10 * bsp::SYSCLK_HZ);
    drivers::power::power_off()
}
