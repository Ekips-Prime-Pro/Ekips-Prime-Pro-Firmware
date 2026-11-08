//! Board support for the LEGO SPIKE Prime hub.
//!
//! - [`pins`]: the complete, sourced pin map.
//! - [`raw`]: raw GPIO register helpers.
//! - [`clock_config`]: the clock tree.
//! - [`safe_state`]: puts all actuators into a harmless state.
//!
//! **Untested on hardware.**

pub mod pins;
#[allow(dead_code)] // helpers are used by drivers added step by step
pub mod raw;

use embassy_stm32::Config;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllRDiv,
    PllSource, Sysclk, mux,
};
use embassy_stm32::time::Hertz;

/// Frequency of the external crystal. \[PB\] `platform.c` L1049-1062.
pub const HSE_HZ: u32 = 16_000_000;
/// System clock, also used for busy-wait delays. Pybricks uses 96 MHz (not the maximum of 100 MHz) so the
/// PLL can also produce exactly 48 MHz for USB. \[PB\] L1058-1062.
pub const SYSCLK_HZ: u32 = 96_000_000;

/// Clock tree identical to Pybricks (`SystemInit`, \[PB\] L1049-1073):
///
/// - HSE 16 MHz → /8 → 2 MHz → ×96 → VCO 192 MHz
/// - PLL P /2 → 96 MHz system clock
/// - PLL Q /4 → 48 MHz USB clock
/// - AHB 96 MHz, APB1 48 MHz (timers 96 MHz), APB2 96 MHz
///
/// **Untested on hardware.**
pub fn clock_config() -> Config {
    let mut config = Config::default();
    let rcc = &mut config.rcc;
    rcc.hse = Some(Hse {
        freq: Hertz(HSE_HZ),
        mode: HseMode::Oscillator,
    });
    rcc.hsi = false;
    rcc.pll_src = PllSource::HSE;
    rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV8,
        mul: PllMul::MUL96,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV4),
        divr: Some(PllRDiv::DIV2),
    });
    rcc.sys = Sysclk::PLL1_P;
    rcc.ahb_pre = AHBPrescaler::DIV1;
    rcc.apb1_pre = APBPrescaler::DIV2;
    rcc.apb2_pre = APBPrescaler::DIV1;
    rcc.mux.clk48sel = mux::Clk48sel::PLL1_Q;
    config
}

/// Drives every actuator into a harmless state without owning any
/// peripheral. Used by the panic handler and before power-off.
///
/// - all motor H-bridge inputs LOW (coast)
/// - speaker amplifier off
/// - Bluetooth chip held in reset
///
/// The TLC5955 keeps its last state (it needs SPI to change).
///
/// **Untested on hardware.**
pub fn safe_state() {
    for port in [
        pins::GpioPort::A,
        pins::GpioPort::B,
        pins::GpioPort::C,
        pins::GpioPort::D,
        pins::GpioPort::E,
    ] {
        raw::enable_port_clock(port);
    }
    for m in pins::MOTOR {
        raw::output(m.pin1, false);
        raw::output(m.pin2, false);
    }
    raw::output(pins::SPEAKER_AMP_ENABLE, false);
    raw::output(pins::BT_ENABLE, false);
}
