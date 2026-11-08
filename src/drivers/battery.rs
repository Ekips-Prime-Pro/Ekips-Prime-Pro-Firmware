//! Battery monitoring and MP2639A charger control.
//!
//! - Readings from [`super::analog`] are converted with
//!   `ekips_hub_logic::battery` (Pybricks scaling).
//! - Charger: MODE pin via TLC5955 channel 14 (0 = enabled, 0xFFFF =
//!   disabled), current limit ISET via TIM5_CH1 on PA0 at 96 kHz,
//!   `/CHG` read through resistor ladder 0 (\[PB\] `platform.c` L147-157,
//!   L552-561, L623-631).
//! - Without USB charger type detection the limit is 500 mA (Pybricks'
//!   value for a standard USB port) – decision D-009.
//! - Battery LED: orange = charging, green = full, red = fault, off when
//!   not on USB.
//! - Below [`ekips_hub_logic::battery::LIION_CRITICAL_MV`] for 5 s on
//!   battery the hub powers off to protect the pack.
//!
//! **Untested on hardware.**

use core::cell::Cell;

use ekips_hub_logic::battery::{
    self, ChargeLimit, ChargerMonitor, ChargerStatus, LIION_CRITICAL_MV,
};
use ekips_hub_logic::buttons::charger_chg_active;
use ekips_hub_logic::led::Rgb;
use embassy_stm32::Peri;
use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA0, TIM5};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{Ch2, Ch3, Ch4};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Ticker};

use super::{analog, power, tlc5955};
use crate::bsp::pins;

/// ISET PWM frequency. \[PB\] `platform.c` L623-631.
const ISET_HZ: u32 = 96_000;
/// Limit used while USB is plugged in (decision D-009).
const USB_LIMIT: ChargeLimit = ChargeLimit::StdMax;
/// Time below the critical voltage before shutting down.
const CRITICAL_SHUTDOWN_MS: u32 = 5_000;

/// Snapshot of the battery state.
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct BatteryState {
    /// Pack voltage in mV (filtered).
    pub voltage_mv: u16,
    /// Battery current in mA.
    pub current_ma: u16,
    /// Estimated charge in percent.
    pub percent: u8,
    /// Temperature in °C (`None` if the NTC reading is invalid).
    pub temperature_c: Option<f32>,
    /// Charger status.
    pub charger: ChargerStatus,
    /// USB VBUS present.
    pub usb: bool,
}

static STATE: Mutex<CriticalSectionRawMutex, Cell<Option<BatteryState>>> =
    Mutex::new(Cell::new(None));

/// Latest battery state (`None` until the first measurement).
#[allow(dead_code, reason = "used by the Hardware implementation later")]
pub fn state() -> Option<BatteryState> {
    STATE.lock(|c| c.get())
}

/// Pins and timer owned by the battery task.
pub struct BatteryResources {
    /// TIM5 for the ISET PWM.
    pub iset_timer: Peri<'static, TIM5>,
    /// PA0, ISET.
    pub iset: Peri<'static, PA0>,
}

/// Monitors the battery and drives the charger.
#[embassy_executor::task]
pub async fn task(r: BatteryResources) {
    let mut pwm = SimplePwm::new(
        r.iset_timer,
        Some(PwmPin::new(r.iset, OutputType::PushPull)),
        None::<PwmPin<'static, TIM5, Ch2>>,
        None::<PwmPin<'static, TIM5, Ch3>>,
        None::<PwmPin<'static, TIM5, Ch4>>,
        Hertz(ISET_HZ),
        CountingMode::EdgeAlignedUp,
    );
    let mut iset = pwm.ch1();
    iset.set_duty_cycle_fully_off();
    iset.enable();
    // Charger disabled until the first decision.
    tlc5955::set(pins::TLC_CH_CHARGER_MODE, u16::MAX);

    let mut monitor = ChargerMonitor::new();
    let mut filtered_mv: Option<u32> = None;
    let mut critical_ms: u32 = 0;
    let mut last_led = None;
    let mut ticker = Ticker::every(Duration::from_millis(u64::from(ChargerMonitor::SAMPLE_MS)));

    loop {
        ticker.next().await;
        let s = analog::latest();
        if s.count == 0 {
            continue;
        }
        let usb = power::usb_powered();
        let current = battery::current_ma(s.battery_current);
        let mv = u32::from(battery::voltage_mv(s.battery_voltage, current));
        // First-order low-pass, like Pybricks' 127/128 average but faster
        // because we sample at 4 Hz.
        let avg = match filtered_mv {
            Some(f) => (f * 7 + mv) / 8,
            None => mv,
        };
        filtered_mv = Some(avg);
        let voltage_mv = avg as u16;

        let chg = charger_chg_active(s.ladder_center).unwrap_or(false);
        let out = monitor.update(usb, USB_LIMIT, chg);
        iset.set_duty_cycle_percent(out.limit.iset_duty_percent());
        tlc5955::set(
            pins::TLC_CH_CHARGER_MODE,
            if out.enable { 0 } else { u16::MAX },
        );

        let led = match (usb, out.status) {
            (false, _) => Rgb::OFF,
            (true, ChargerStatus::Charge) => Rgb::ORANGE,
            (true, ChargerStatus::Complete) => Rgb::GREEN,
            (true, ChargerStatus::Fault) => Rgb::RED,
            (true, ChargerStatus::Discharge) => Rgb::OFF,
        };
        if last_led != Some(led) {
            tlc5955::set_rgb(pins::TLC_BATTERY, led);
            last_led = Some(led);
        }

        let st = BatteryState {
            voltage_mv,
            current_ma: current,
            percent: battery::percent(voltage_mv),
            temperature_c: battery::temperature_c(s.battery_ntc),
            charger: out.status,
            usb,
        };
        STATE.lock(|c| c.set(Some(st)));

        if !usb && voltage_mv < LIION_CRITICAL_MV {
            critical_ms += ChargerMonitor::SAMPLE_MS;
            if critical_ms >= CRITICAL_SHUTDOWN_MS {
                defmt::warn!("battery critical ({} mV), powering off", voltage_mv);
                power::power_off();
            }
        } else {
            critical_ms = 0;
        }
    }
}
