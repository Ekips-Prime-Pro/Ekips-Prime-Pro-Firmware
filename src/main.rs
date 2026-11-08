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
    spawner.spawn(
        drivers::tlc5955::task(drivers::tlc5955::Tlc5955Resources {
            spi: p.SPI1,
            sck: p.PA5,
            mosi: p.PA7,
            tx_dma: p.DMA2_CH3,
            lat: p.PA15,
            gsclk_timer: p.TIM12,
            gsclk: p.PB15,
        })
        .unwrap(),
    );
    drivers::usb::start(
        spawner,
        drivers::usb::UsbResources {
            otg: p.USB_OTG_FS,
            dp: p.PA12,
            dm: p.PA11,
        },
    );
    // Green status light: firmware is running.
    drivers::status_light::set(ekips_hub_logic::led::Rgb::GREEN);
    spawner.spawn(drivers::matrix::task().unwrap());
    spawner.spawn(
        drivers::imu::task(drivers::imu::ImuResources {
            i2c: p.I2C2,
            scl: p.PB10,
            sda: p.PB3,
            int1: p.PB4,
            exti: p.EXTI4,
            tx_dma: p.DMA1_CH7,
            rx_dma: p.DMA1_CH2,
        })
        .unwrap(),
    );
    spawner.spawn(
        drivers::speaker::task(drivers::speaker::SpeakerResources {
            dac: p.DAC1,
            out: p.PA4,
            dma: p.DMA1_CH5,
            timer: p.TIM6,
            amp_enable: p.PC10,
        })
        .unwrap(),
    );
    // Start-up beep.
    drivers::speaker::beep(1000, 100);
    spawner.spawn(
        drivers::battery::task(drivers::battery::BatteryResources {
            iset_timer: p.TIM5,
            iset: p.PA0,
        })
        .unwrap(),
    );
    drivers::matrix::show_text("EKIPS");

    drivers::buttons::POWER_OFF_REQUEST.wait().await;
    drivers::usb::write(b"powering off\r\n");
    drivers::matrix::clear();
    embassy_time::Timer::after_millis(100).await;
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
