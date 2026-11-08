//! H-bridge PWM for the six motor ports.
//!
//! - TIM1 (ports A, B), TIM4 (C, D), TIM3 (E, F) at 12 kHz, all channels
//!   PWM mode 1 with **inverted** polarity, like Pybricks (\[PB\]
//!   `platform.c` L563-604). Pybricks notes that the LEGO firmware uses
//!   1.2 kHz; 12 kHz is quieter.
//! - Pins are switched at runtime between timer alternate function and
//!   GPIO LOW/HIGH (`ekips_hub_logic::motor::pin_states`). The pins are
//!   owned through [`MotorResources`] and then driven with `bsp::raw`.
//! - All ports start in coast. The panic handler and power-off set every
//!   input LOW (coast) through `bsp::safe_state`.
//!
//! This is the open-loop duty-cycle layer. Position/speed control needs
//! the motor encoders, which arrive over LUMP (planned `drivers::lump`).
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_api::Port;
use ekips_hub_logic::motor::{Drive, PinState, compare_value, pin_states};
use embassy_stm32::Peri;
use embassy_stm32::gpio::AnyPin;
use embassy_stm32::peripherals::{TIM1, TIM3, TIM4};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::low_level::{OutputCompareMode, OutputPolarity, RoundTo, Timer};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;

use crate::bsp::pins::{self, MotorTimer};
use crate::bsp::raw;

/// PWM frequency. \[PB\] `platform.c` L563-573.
pub const PWM_HZ: u32 = 12_000;

/// Timers and the twelve H-bridge pins (owned so nothing else uses them).
pub struct MotorResources {
    /// TIM1 (ports A, B).
    pub tim1: Peri<'static, TIM1>,
    /// TIM3 (ports E, F).
    pub tim3: Peri<'static, TIM3>,
    /// TIM4 (ports C, D).
    pub tim4: Peri<'static, TIM4>,
    /// H-bridge pins in port order A1, A2, B1, B2, … (see `bsp::pins::MOTOR`).
    pub pins: [Peri<'static, AnyPin>; 12],
}

struct Motors {
    tim1: Timer<'static, TIM1>,
    tim3: Timer<'static, TIM3>,
    tim4: Timer<'static, TIM4>,
}

static MOTORS: Mutex<CriticalSectionRawMutex, RefCell<Option<Motors>>> =
    Mutex::new(RefCell::new(None));

fn channel(n: u8) -> Channel {
    match n {
        1 => Channel::Ch1,
        2 => Channel::Ch2,
        3 => Channel::Ch3,
        _ => Channel::Ch4,
    }
}

macro_rules! setup_timer {
    ($t:expr) => {{
        let t = $t;
        t.stop();
        t.set_frequency(Hertz(PWM_HZ), RoundTo::Faster);
        for ch in [Channel::Ch1, Channel::Ch2, Channel::Ch3, Channel::Ch4] {
            t.set_output_compare_mode(ch, OutputCompareMode::PwmMode1);
            t.set_output_compare_preload(ch, true);
            t.set_output_polarity(ch, OutputPolarity::ActiveLow);
            t.set_compare_value(ch, 0);
            t.enable_channel(ch, true);
        }
        t.enable_outputs();
        t.start();
        t
    }};
}

/// Configures the timers and puts all ports into coast.
pub fn init(r: MotorResources) {
    // The pins are only claimed; from now on bsp::raw drives them.
    let MotorResources {
        tim1,
        tim3,
        tim4,
        pins: _claimed,
    } = r;
    for m in pins::MOTOR {
        raw::output(m.pin1, false);
        raw::output(m.pin2, false);
    }
    let motors = Motors {
        tim1: setup_timer!(Timer::new(tim1)),
        tim3: setup_timer!(Timer::new(tim3)),
        tim4: setup_timer!(Timer::new(tim4)),
    };
    MOTORS.lock(|m| *m.borrow_mut() = Some(motors));
    defmt::info!("motor PWM initialized ({} Hz)", PWM_HZ);
}

fn set_compare(m: &Motors, timer: MotorTimer, ch: u8, duty: u16) {
    let ch = channel(ch);
    match timer {
        MotorTimer::Tim1 => {
            let v = compare_value(duty, m.tim1.get_max_compare_value().into());
            m.tim1.set_compare_value(ch, v as u16);
        }
        MotorTimer::Tim3 => {
            let v = compare_value(duty, m.tim3.get_max_compare_value().into());
            m.tim3.set_compare_value(ch, v as u16);
        }
        MotorTimer::Tim4 => {
            let v = compare_value(duty, m.tim4.get_max_compare_value().into());
            m.tim4.set_compare_value(ch, v as u16);
        }
    }
}

/// Sets the H-bridge of `port`. Does nothing before [`init`].
pub fn set(port: Port, drive: Drive) {
    let mp = pins::MOTOR[port.index()];
    let (s1, s2) = pin_states(drive);
    MOTORS.lock(|m| {
        let m = m.borrow();
        let Some(m) = m.as_ref() else {
            return;
        };
        for (pin, ch, state) in [(mp.pin1, mp.ch1, s1), (mp.pin2, mp.ch2, s2)] {
            match state {
                PinState::Low => raw::output(pin, false),
                PinState::High => raw::output(pin, true),
                PinState::Pwm(duty) => {
                    set_compare(m, mp.timer, ch, duty);
                    raw::alternate(pin, mp.af);
                }
            }
        }
    });
}

/// Coasts all ports.
pub fn coast_all() {
    for port in Port::ALL {
        set(port, Drive::Coast);
    }
}
