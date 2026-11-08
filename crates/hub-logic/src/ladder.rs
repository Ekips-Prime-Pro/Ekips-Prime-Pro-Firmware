//! Resistor ladder decoding.
//!
//! The SPIKE Prime hub reads several switches through one ADC pin each:
//! every switch adds a resistor to a ladder, so each combination of closed
//! switches gives a distinct voltage. Ported from Pybricks
//! `lib/pbio/drv/resistor_ladder/resistor_ladder.c` (commit `c860b6a`,
//! MIT); thresholds from `platform.c` L660-671.

/// Bit flags of the three ladder inputs. A flag is set when the
/// corresponding switch is closed (resistor connected).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LadderFlags(pub u8);

impl LadderFlags {
    /// Input 0.
    pub const CH0: u8 = 1 << 0;
    /// Input 1.
    pub const CH1: u8 = 1 << 1;
    /// Input 2.
    pub const CH2: u8 = 1 << 2;

    /// Whether input `mask` is set.
    pub const fn has(self, mask: u8) -> bool {
        self.0 & mask != 0
    }
}

/// Thresholds of one ladder (12-bit ADC counts, descending).
pub type LadderLevels = [u16; 8];

/// Ladder 0 on PC4 / ADC ch14: CH1 = center button, CH2 = charger `/CHG`.
/// Pybricks `platform.c` L663-666.
pub const LADDER0_LEVELS: LadderLevels = [3642, 3142, 2879, 2634, 2449, 2209, 2072, 1800];

/// Ladder 1 on PA1 / ADC ch1: CH0 = left, CH1 = right, CH2 = Bluetooth button.
/// Pybricks `platform.c` L667-670.
pub const LADDER1_LEVELS: LadderLevels = [3872, 3394, 3009, 2755, 2538, 2327, 2141, 1969];

/// Decodes a raw 12-bit ADC value. Returns `None` for values below the
/// lowest level, which indicates a hardware fault (or that the ADC has not
/// settled yet right after boot).
pub fn decode(raw: u16, levels: &LadderLevels) -> Option<LadderFlags> {
    use LadderFlags as F;
    const TABLE: [u8; 8] = [
        0,
        F::CH2,
        F::CH1,
        F::CH1 | F::CH2,
        F::CH0,
        F::CH0 | F::CH2,
        F::CH0 | F::CH1,
        F::CH0 | F::CH1 | F::CH2,
    ];
    levels
        .iter()
        .position(|&level| raw > level)
        .map(|i| LadderFlags(TABLE[i]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_every_band() {
        let l = LADDER1_LEVELS;
        assert_eq!(decode(4095, &l), Some(LadderFlags(0)));
        assert_eq!(decode(3500, &l), Some(LadderFlags(LadderFlags::CH2)));
        assert_eq!(decode(3100, &l), Some(LadderFlags(LadderFlags::CH1)));
        assert_eq!(decode(2000, &l).map(|f| f.0), Some(0b111));
        assert_eq!(decode(1969, &l), None);
        assert_eq!(decode(0, &l), None);
    }

    #[test]
    fn boundaries_are_exclusive() {
        let l = LADDER0_LEVELS;
        assert_eq!(decode(3642, &l), Some(LadderFlags(LadderFlags::CH2)));
        assert_eq!(decode(3643, &l), Some(LadderFlags(0)));
    }
}
