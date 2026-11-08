//! TLC5955 48-channel LED driver: shift register frames.
//!
//! The TLC5955 has a 769-bit shift register, sent MSB first as 97 bytes.
//! Bit 768 selects the latch: 1 = control data, 0 = grayscale data.
//! Ported from Pybricks `lib/pbio/drv/pwm/pwm_tlc5955_stm32.c`
//! (commit `c860b6a`, MIT).

/// Size of one frame in bytes ((769 + 7) / 8).
pub const FRAME_LEN: usize = 97;
/// Number of PWM channels.
pub const NUM_CHANNELS: usize = 48;

/// Maximum constant current per channel (MC field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MaxCurrent {
    /// 3.2 mA (used by Pybricks for SPIKE Prime).
    Ma3_2 = 0b000,
    /// 8.0 mA
    Ma8_0 = 0b001,
    /// 11.2 mA
    Ma11_2 = 0b010,
    /// 15.9 mA
    Ma15_9 = 0b011,
}

/// Control data settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlData {
    /// Dot correction, 7 bit, same for all channels.
    pub dc: u8,
    /// Maximum current.
    pub mc: MaxCurrent,
    /// Global brightness control, 7 bit.
    pub bc: u8,
    /// Auto display repeat.
    pub dsprpt: bool,
    /// Display timing reset.
    pub tmgrst: bool,
    /// Auto data refresh.
    pub rfresh: bool,
    /// ES-PWM mode.
    pub espwm: bool,
    /// LSD detection voltage selection.
    pub lsdvlt: bool,
}

impl ControlData {
    /// The settings Pybricks uses on SPIKE Prime: DC 100 %, 3.2 mA, BC
    /// 100 %, auto repeat on, timing reset off, auto refresh off, ES-PWM
    /// on, LSD voltage 90 %.
    pub const PYBRICKS_SPIKE_PRIME: Self = Self {
        dc: 127,
        mc: MaxCurrent::Ma3_2,
        bc: 127,
        dsprpt: true,
        tmgrst: false,
        rfresh: false,
        espwm: true,
        lsdvlt: true,
    };

    /// Builds the control frame. Byte-for-byte port of the Pybricks
    /// `TLC5955_CONTROL_DATA` macro.
    pub fn frame(&self) -> [u8; FRAME_LEN] {
        let dc = u32::from(self.dc & 0x7F);
        let mc = u32::from(self.mc as u8);
        let bc = u32::from(self.bc & 0x7F);
        let b = |v: bool| u32::from(v);
        let mut f = [0u8; FRAME_LEN];
        f[0] = 1; // bit 768: control latch
        f[1] = 0x96; // bits 767-760: magic value selecting control data
        f[50] = (((b(self.lsdvlt) << 2) | (b(self.espwm) << 1) | b(self.rfresh)) & 0xff) as u8;
        f[51] = (((b(self.tmgrst) << 7) | (b(self.dsprpt) << 6) | (bc >> 1)) & 0xff) as u8;
        f[52] = (((bc << 7) | bc) & 0xff) as u8;
        f[53] = (((bc << 1) | (mc >> 2)) & 0xff) as u8;
        f[54] = (((mc << 6) | (mc << 3) | mc) & 0xff) as u8;
        // Bytes 55..=96 repeat the 7-bit DC value of all 48 channels; the
        // shift pattern has a period of 7 bytes.
        for (i, byte) in f[55..=96].iter_mut().enumerate() {
            let s = (i % 7) as u32 + 1;
            *byte = (((dc << s) | (dc >> (7 - s))) & 0xff) as u8;
        }
        f
    }
}

/// Grayscale data of all 48 channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grayscale {
    frame: [u8; FRAME_LEN],
}

impl Grayscale {
    /// All channels off.
    pub const fn new() -> Self {
        Self {
            frame: [0; FRAME_LEN],
        }
    }

    /// Sets channel `ch` (0..48) to `duty` (0 = off, 0xFFFF = full).
    /// Out-of-range channels are ignored.
    ///
    /// Channels are mapped to the GS registers in reverse order: CH 0 is
    /// GSB15 … CH 47 is GSR0, exactly as in Pybricks.
    pub fn set(&mut self, ch: u8, duty: u16) {
        let ch = ch as usize;
        if ch < NUM_CHANNELS {
            self.frame[ch * 2 + 1] = (duty >> 8) as u8;
            self.frame[ch * 2 + 2] = duty as u8;
        }
    }

    /// Reads back channel `ch`.
    pub fn get(&self, ch: u8) -> u16 {
        let ch = ch as usize;
        if ch < NUM_CHANNELS {
            u16::from_be_bytes([self.frame[ch * 2 + 1], self.frame[ch * 2 + 2]])
        } else {
            0
        }
    }

    /// The frame to shift out (bit 768 = 0: grayscale latch).
    pub fn frame(&self) -> &[u8; FRAME_LEN] {
        &self.frame
    }
}

impl Default for Grayscale {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values computed from the Pybricks C macro
    /// `TLC5955_CONTROL_DATA(x, 127, TLC5955_MC_3_2, 127, 1, 0, 0, 1, 1)`.
    #[test]
    fn control_frame_matches_pybricks_macro() {
        let f = ControlData::PYBRICKS_SPIKE_PRIME.frame();
        assert_eq!(f[0], 1);
        assert_eq!(f[1], 0x96);
        assert!(f[2..50].iter().all(|&b| b == 0));
        assert_eq!(f[50], 0b110); // lsdvlt, espwm
        assert_eq!(f[51], 0x40 | 0x3F); // dsprpt, bc >> 1
        assert_eq!(f[52], 0xFF);
        assert_eq!(f[53], 0xFE);
        assert_eq!(f[54], 0x00);
        // dc = 127 = all ones -> every DC byte is 0xFF.
        assert!(f[55..].iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn control_frame_dc_pattern() {
        let mut c = ControlData::PYBRICKS_SPIKE_PRIME;
        c.dc = 0b100_0000;
        let f = c.frame();
        // Macro: [55] = (dc << 1) | (dc >> 6), [61] = (dc << 7) | dc, [62] = [55]
        assert_eq!(f[55], 0x81);
        assert_eq!(f[61], 0x40);
        assert_eq!(f[62], f[55]);
        assert_eq!(f[96], ((0x40u32 << 7) as u8) | 0x40);
    }

    #[test]
    fn grayscale_layout() {
        let mut g = Grayscale::new();
        g.set(0, 0x1234);
        g.set(47, 0xABCD);
        g.set(48, 0xFFFF); // ignored
        let f = g.frame();
        assert_eq!(f[0], 0);
        assert_eq!(&f[1..3], &[0x12, 0x34]);
        assert_eq!(&f[95..97], &[0xAB, 0xCD]);
        assert_eq!(g.get(47), 0xABCD);
    }
}
