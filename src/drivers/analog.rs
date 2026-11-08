//! ADC1 sampling of all analog inputs of the hub.
//!
//! One task owns ADC1 and samples the six channels used by Pybricks
//! (\[PB\] `platform.c` L773-860) every [`SAMPLE_PERIOD_MS`]. Other drivers
//! read the latest values with [`latest`]; the conversion to physical
//! units lives in `ekips_hub_logic`.
//!
//! Pybricks triggers the ADC from TIM2 with DMA. A software-triggered
//! blocking read of six channels takes a few microseconds, which is fine
//! at this rate and keeps TIM2 free.
//!
//! **Untested on hardware.**

use core::cell::Cell;

use embassy_stm32::Peri;
use embassy_stm32::adc::{Adc, AdcChannel, AnyAdcChannel, SampleTime};
use embassy_stm32::peripherals::{ADC1, PA1, PA3, PB0, PC0, PC1, PC4};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Ticker};

/// Sampling period of all analog channels.
pub const SAMPLE_PERIOD_MS: u64 = 10;

/// Latest raw 12-bit readings.
#[derive(Debug, Clone, Copy, Default, defmt::Format)]
pub struct AnalogSample {
    /// Battery current, PC0 / ch10.
    pub battery_current: u16,
    /// Battery voltage, PC1 / ch11.
    pub battery_voltage: u16,
    /// Battery NTC, PB0 / ch8.
    pub battery_ntc: u16,
    /// USB charger current, PA3 / ch3.
    pub usb_charge_current: u16,
    /// Resistor ladder 0 (center button, charger /CHG), PC4 / ch14.
    pub ladder_center: u16,
    /// Resistor ladder 1 (left, right, Bluetooth buttons), PA1 / ch1.
    pub ladder_buttons: u16,
    /// Number of completed sampling rounds (0 = no data yet).
    pub count: u32,
}

static LATEST: Mutex<CriticalSectionRawMutex, Cell<AnalogSample>> =
    Mutex::new(Cell::new(AnalogSample {
        battery_current: 0,
        battery_voltage: 0,
        battery_ntc: 0,
        usb_charge_current: 0,
        ladder_center: 0,
        ladder_buttons: 0,
        count: 0,
    }));

/// Returns the most recent sample.
pub fn latest() -> AnalogSample {
    LATEST.lock(|c| c.get())
}

/// Pins and peripheral owned by the analog task.
pub struct AnalogResources {
    /// ADC1.
    pub adc: Peri<'static, ADC1>,
    /// PC0, battery current.
    pub battery_current: Peri<'static, PC0>,
    /// PC1, battery voltage.
    pub battery_voltage: Peri<'static, PC1>,
    /// PB0, battery NTC.
    pub battery_ntc: Peri<'static, PB0>,
    /// PA3, USB charge current.
    pub usb_charge_current: Peri<'static, PA3>,
    /// PC4, resistor ladder 0.
    pub ladder_center: Peri<'static, PC4>,
    /// PA1, resistor ladder 1.
    pub ladder_buttons: Peri<'static, PA1>,
}

/// Samples all channels forever.
#[embassy_executor::task]
pub async fn task(r: AnalogResources) {
    let mut adc = Adc::new(r.adc);
    let mut chs: [AnyAdcChannel<'static, ADC1>; 6] = [
        r.battery_current.degrade_adc(),
        r.battery_voltage.degrade_adc(),
        r.battery_ntc.degrade_adc(),
        r.usb_charge_current.degrade_adc(),
        r.ladder_center.degrade_adc(),
        r.ladder_buttons.degrade_adc(),
    ];
    // Pybricks uses 15 cycles; the ladders have high impedance, so a
    // longer sample time does no harm at this rate.
    let st = SampleTime::CYCLES84;
    let mut ticker = Ticker::every(Duration::from_millis(SAMPLE_PERIOD_MS));
    let mut count: u32 = 0;
    loop {
        let mut v = [0u16; 6];
        for (value, ch) in v.iter_mut().zip(chs.iter_mut()) {
            *value = adc.blocking_read(ch, st);
        }
        count = count.wrapping_add(1).max(1);
        LATEST.lock(|c| {
            c.set(AnalogSample {
                battery_current: v[0],
                battery_voltage: v[1],
                battery_ntc: v[2],
                usb_charge_current: v[3],
                ladder_center: v[4],
                ladder_buttons: v[5],
                count,
            })
        });
        ticker.next().await;
    }
}
