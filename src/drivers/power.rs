//! Power latch, port VCC and power-off.
//!
//! The hub has a soft-latch power circuit: pressing the center button
//! powers the MCU, and the firmware must then drive PA13 HIGH to keep the
//! main 3.3 V rail on. Releasing PA13 turns the hub off again unless USB
//! is plugged in (VBUS keeps the rail alive). \[PB\] `platform.c`
//! L651-656, L1086-1092; spike-rtic `src/power.rs` L1-23.
//!
//! The latch is set in `__pre_init` (assembly, before `.data`/`.bss`
//! initialization and before Embassy starts), so the hub stays on even if
//! the rest of the startup takes a while when running from battery.
//!
//! With the `swd-debug` feature, PA13/PA14 are left in their SWD reset
//! function and the hub must be powered over USB (decision D-003).
//!
//! **Untested on hardware.**

use crate::bsp::{pins, raw};

// Early power latch, executed by cortex-m-rt right after reset.
//
// RCC_AHB1ENR = 0x4002_3830, bit 0 = GPIOAEN
// GPIOA_BSRR  = 0x4002_0018, bit 13 = BS13 (set PA13 before switching mode)
// GPIOA_MODER = 0x4002_0000, bits 27:26 = 0b01 (general purpose output)
//
// Written in assembly because no Rust code may run before RAM is
// initialized (see the cortex-m-rt documentation of `__pre_init`).
#[cfg(not(feature = "swd-debug"))]
core::arch::global_asm!(
    ".section .text.__pre_init, \"ax\"",
    ".global __pre_init",
    ".type __pre_init, %function",
    ".thumb_func",
    "__pre_init:",
    "    ldr r0, =0x40023830",
    "    ldr r1, [r0]",
    "    orr r1, r1, #1",
    "    str r1, [r0]",
    "    ldr r1, [r0]", // read back: wait for the clock to be enabled
    "    ldr r0, =0x40020018",
    "    mov r1, #0x2000",
    "    str r1, [r0]",
    "    ldr r0, =0x40020000",
    "    ldr r1, [r0]",
    "    bic r1, r1, #0x0C000000",
    "    orr r1, r1, #0x04000000",
    "    str r1, [r0]",
    "    bx lr",
    ".size __pre_init, . - __pre_init",
);

/// Whether this build drives the power latch (false with `swd-debug`).
pub const LATCH_ENABLED: bool = cfg!(not(feature = "swd-debug"));

/// Makes sure the power latch is held (idempotent; normally already done
/// in `__pre_init`). Does nothing with `swd-debug`.
pub fn hold() {
    if LATCH_ENABLED {
        raw::enable_port_clock(pins::POWER_LATCH.port);
        raw::output(pins::POWER_LATCH, true);
    }
}

/// Switches 3.3 V on the six I/O ports on or off. Required for sensors and
/// motor encoders. Does nothing with `swd-debug` because PA14 is SWCLK.
/// \[PB\] `ioport.c` L89-97.
pub fn set_port_vcc(on: bool) {
    if LATCH_ENABLED {
        raw::output(pins::PORT_VCC_ENABLE, on);
    }
}

/// Whether USB VBUS is present (hub is powered by USB).
pub fn usb_powered() -> bool {
    raw::read(pins::USB_VBUS)
}

/// Turns the hub off: actuators into safe state, port VCC off, latch
/// released. If USB is plugged in the hub stays powered; the function then
/// loops forever with everything off (a reset or unplugging ends it).
pub fn power_off() -> ! {
    crate::bsp::safe_state();
    set_port_vcc(false);
    if LATCH_ENABLED {
        raw::write(pins::POWER_LATCH, false);
    }
    loop {
        cortex_m::asm::wfi();
    }
}
