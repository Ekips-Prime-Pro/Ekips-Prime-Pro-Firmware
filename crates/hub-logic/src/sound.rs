//! Beep generation for the DAC speaker.
//!
//! Like Pybricks (`lib/pbio/drv/sound/beep_sampled.c`, commit `c860b6a`)
//! a beep is one period of a square wave that the DAC plays in a loop via
//! circular DMA; a timer sets the sample rate.

/// Samples per waveform period.
pub const SAMPLES_PER_PERIOD: usize = 32;
/// Lowest supported beep frequency in Hz.
pub const MIN_FREQ_HZ: u16 = 20;
/// Highest supported beep frequency in Hz.
pub const MAX_FREQ_HZ: u16 = 10_000;
/// DAC mid level (silence) in 16-bit left-aligned format.
pub const MID_LEVEL: u16 = 0x8000;

/// Peak amplitude for a volume in percent (0..=100), squared so the
/// perceived loudness grows more evenly.
pub fn amplitude(volume_percent: u8) -> u16 {
    let v = u32::from(volume_percent.min(100));
    (v * v * 0x7FFF / 10_000) as u16
}

/// Fills one period of a square wave around [`MID_LEVEL`].
pub fn square_wave(amplitude: u16, out: &mut [u16; SAMPLES_PER_PERIOD]) {
    let amplitude = amplitude.min(0x7FFF);
    let (lo, hi) = (MID_LEVEL - amplitude, MID_LEVEL + amplitude);
    for (i, s) in out.iter_mut().enumerate() {
        *s = if i < SAMPLES_PER_PERIOD / 2 { lo } else { hi };
    }
}

/// Clamps a frequency to the supported range.
pub fn clamp_freq(freq_hz: u16) -> u16 {
    freq_hz.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ)
}

/// Timer trigger rate (samples per second) for a beep frequency.
pub fn sample_rate_hz(freq_hz: u16) -> u32 {
    u32::from(clamp_freq(freq_hz)) * SAMPLES_PER_PERIOD as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_is_symmetric() {
        let mut w = [0; SAMPLES_PER_PERIOD];
        square_wave(1000, &mut w);
        assert_eq!(w[0], MID_LEVEL - 1000);
        assert_eq!(w[SAMPLES_PER_PERIOD - 1], MID_LEVEL + 1000);
        square_wave(u16::MAX, &mut w);
        assert_eq!(w[SAMPLES_PER_PERIOD - 1], 0xFFFF);
        assert_eq!(w[0], 1);
    }

    #[test]
    fn volume_and_rate() {
        assert_eq!(amplitude(0), 0);
        assert_eq!(amplitude(100), 0x7FFF);
        assert_eq!(sample_rate_hz(1000), 32_000);
        assert_eq!(sample_rate_hz(1), u32::from(MIN_FREQ_HZ) * 32);
        assert_eq!(sample_rate_hz(60_000), 320_000);
    }
}
