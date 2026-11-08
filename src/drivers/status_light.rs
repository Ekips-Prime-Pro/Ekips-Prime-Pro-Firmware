//! Center button status light (two RGB LEDs, top and bottom half).
//!
//! TLC5955 channels: top (5, 4, 3), bottom (8, 7, 6) – \[PB\] `platform.c`
//! L367-389.
//!
//! **Untested on hardware.**

use ekips_hub_logic::led::Rgb;

use super::tlc5955;
use crate::bsp::pins;

/// Sets both halves of the status light to `color`.
pub fn set(color: Rgb) {
    set_halves(color, color);
}

/// Sets the top and bottom halves separately.
pub fn set_halves(top: Rgb, bottom: Rgb) {
    tlc5955::set_rgb(pins::TLC_STATUS_TOP, top);
    tlc5955::set_rgb(pins::TLC_STATUS_BOTTOM, bottom);
}
