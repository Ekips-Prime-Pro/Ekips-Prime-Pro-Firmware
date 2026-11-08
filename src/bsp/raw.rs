//! Raw GPIO register access for the few places where Embassy's typed pin
//! API does not fit:
//!
//! - the panic handler, which must reach a safe state without owning any
//!   peripheral,
//! - the motor H-bridges, which switch pins between timer alternate
//!   function and plain GPIO output at runtime (like Pybricks does).
//!
//! Callers must own the corresponding Embassy pin (or be the panic handler)
//! so no other driver touches the same pin concurrently.
//!
//! **Untested on hardware.**

use embassy_stm32::pac;
use embassy_stm32::pac::gpio::vals;

use super::pins::{GpioPort, PinId};

fn regs(port: GpioPort) -> pac::gpio::Gpio {
    match port {
        GpioPort::A => pac::GPIOA,
        GpioPort::B => pac::GPIOB,
        GpioPort::C => pac::GPIOC,
        GpioPort::D => pac::GPIOD,
        GpioPort::E => pac::GPIOE,
    }
}

/// Enables the AHB1 clock of a GPIO port. Embassy does this in `init()`
/// already; this is for code that may run before or without it.
pub fn enable_port_clock(port: GpioPort) {
    pac::RCC.ahb1enr().modify(|w| match port {
        GpioPort::A => w.set_gpioaen(true),
        GpioPort::B => w.set_gpioben(true),
        GpioPort::C => w.set_gpiocen(true),
        GpioPort::D => w.set_gpioden(true),
        GpioPort::E => w.set_gpioeen(true),
    });
}

/// Drives the pin HIGH (`true`) or LOW (`false`). Does not change the mode.
pub fn write(pin: PinId, high: bool) {
    let r = regs(pin.port);
    let n = pin.pin as usize;
    r.bsrr().write(|w| {
        if high {
            w.set_bs(n, true)
        } else {
            w.set_br(n, true)
        }
    });
}

/// Configures the pin as push-pull output and drives it to `high`.
/// The output level is set before the mode to avoid a glitch.
pub fn output(pin: PinId, high: bool) {
    write(pin, high);
    let r = regs(pin.port);
    let n = pin.pin as usize;
    r.otyper().modify(|w| w.set_ot(n, vals::Ot::PUSH_PULL));
    r.moder().modify(|w| w.set_moder(n, vals::Moder::OUTPUT));
}

/// Configures the pin as push-pull alternate function `af`.
pub fn alternate(pin: PinId, af: u8) {
    let r = regs(pin.port);
    let n = pin.pin as usize;
    r.afr(n / 8).modify(|w| w.set_afr(n % 8, af));
    r.otyper().modify(|w| w.set_ot(n, vals::Ot::PUSH_PULL));
    r.moder().modify(|w| w.set_moder(n, vals::Moder::ALTERNATE));
}

/// Configures the pin as floating input.
pub fn input(pin: PinId) {
    let r = regs(pin.port);
    let n = pin.pin as usize;
    r.pupdr().modify(|w| w.set_pupdr(n, vals::Pupdr::FLOATING));
    r.moder().modify(|w| w.set_moder(n, vals::Moder::INPUT));
}

/// Reads the input level of the pin.
pub fn read(pin: PinId) -> bool {
    regs(pin.port).idr().read().idr(pin.pin as usize) == vals::Idr::HIGH
}
