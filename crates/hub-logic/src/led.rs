//! Color and brightness conversion for the RGB LEDs on the TLC5955.
//!
//! Port of Pybricks `lib/pbio/drv/led/led_pwm.c` (commit `c860b6a`, MIT)
//! with the SPIKE Prime constants from `platform.c` L358-411, adapted to
//! take RGB input instead of HSV.

/// An RGB color, 8 bits per component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Rgb {
    /// Red
    pub r: u8,
    /// Green
    pub g: u8,
    /// Blue
    pub b: u8,
}

impl Rgb {
    /// Creates a color.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
    /// Off.
    pub const OFF: Self = Self::new(0, 0, 0);
    /// Red.
    pub const RED: Self = Self::new(255, 0, 0);
    /// Green.
    pub const GREEN: Self = Self::new(0, 255, 0);
    /// Blue.
    pub const BLUE: Self = Self::new(0, 0, 255);
    /// Yellow.
    pub const YELLOW: Self = Self::new(255, 255, 0);
    /// Orange.
    pub const ORANGE: Self = Self::new(255, 128, 0);
    /// White.
    pub const WHITE: Self = Self::new(255, 255, 255);
}

// SPIKE Prime LED color constants, Pybricks platform.c L358-365.
const R_FACTOR: u32 = 1000;
const G_FACTOR: u32 = 170;
const B_FACTOR: u32 = 200;
const R_BRIGHTNESS: u32 = 174;
const G_BRIGHTNESS: u32 = 1590;
const B_BRIGHTNESS: u32 = 327;
// Pybricks platform.c L377 (same for all RGB LEDs).
const SCALE_FACTOR: u32 = 35;

/// Converts a color to (red, green, blue) TLC5955 duty cycles.
///
/// Like Pybricks: the hue/saturation part is corrected for the LED's
/// chromaticity, normalized to apparent brightness, then scaled by the
/// squared value (gamma) of the original color.
pub fn rgb_to_duty(c: Rgb) -> (u16, u16, u16) {
    let max = u32::from(c.r.max(c.g).max(c.b));
    if max == 0 {
        return (0, 0, 0);
    }
    // Same hue/saturation at full value (V = 100 %).
    let (r, g, b) = (
        u32::from(c.r) * 255 / max,
        u32::from(c.g) * 255 / max,
        u32::from(c.b) * 255 / max,
    );
    let r = r * R_FACTOR;
    let g = g * G_FACTOR;
    let b = b * B_FACTOR;
    let y = ((R_BRIGHTNESS * r + G_BRIGHTNESS * g + B_BRIGHTNESS * b) >> 12) + 1;
    let v = max * 100 / 255; // HSV value in percent
    let scale = v * v * SCALE_FACTOR / 128;
    let clamp = |x: u32| (x * scale / y).min(u32::from(u16::MAX)) as u16;
    (clamp(r), clamp(g), clamp(b))
}

/// Converts a brightness in percent (0..=100) to a duty cycle for a
/// single-color pixel of the 5x5 matrix. Squared for perceived linearity.
pub fn brightness_to_duty(percent: u8) -> u16 {
    let p = u32::from(percent.min(100));
    (p * p * u32::from(u16::MAX) / 10_000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_is_off() {
        assert_eq!(rgb_to_duty(Rgb::OFF), (0, 0, 0));
        assert_eq!(brightness_to_duty(0), 0);
    }

    #[test]
    fn pure_colors_only_drive_their_channel() {
        let (r, g, b) = rgb_to_duty(Rgb::RED);
        assert!(r > 0 && g == 0 && b == 0);
        let (r, g, b) = rgb_to_duty(Rgb::BLUE);
        assert!(r == 0 && g == 0 && b > 0);
    }

    #[test]
    fn dimmer_is_darker() {
        let full = rgb_to_duty(Rgb::GREEN).1;
        let half = rgb_to_duty(Rgb::new(0, 128, 0)).1;
        assert!(half < full && half > 0);
    }

    #[test]
    fn brightness_curve() {
        assert_eq!(brightness_to_duty(100), u16::MAX);
        assert_eq!(brightness_to_duty(200), u16::MAX);
        assert!(brightness_to_duty(50) < u16::MAX / 2);
    }
}
