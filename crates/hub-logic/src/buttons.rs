//! Button state, debouncing and long-press detection.

use crate::ladder::{self, LadderFlags};

/// The four hub buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Buttons {
    /// Center (power) button.
    pub center: bool,
    /// Left arrow button.
    pub left: bool,
    /// Right arrow button.
    pub right: bool,
    /// Bluetooth button (top right).
    pub bluetooth: bool,
}

/// Maps the two ladder readings to buttons. Mapping from Pybricks
/// `lib/pbio/drv/button/button_resistor_ladder.c` (commit `c860b6a`).
/// Returns `None` while a reading is invalid.
pub fn from_adc(ladder0_raw: u16, ladder1_raw: u16) -> Option<Buttons> {
    let l0 = ladder::decode(ladder0_raw, &ladder::LADDER0_LEVELS)?;
    let l1 = ladder::decode(ladder1_raw, &ladder::LADDER1_LEVELS)?;
    Some(Buttons {
        center: l0.has(LadderFlags::CH1),
        left: l1.has(LadderFlags::CH0),
        right: l1.has(LadderFlags::CH1),
        bluetooth: l1.has(LadderFlags::CH2),
    })
}

/// Whether the charger `/CHG` line is active (charging), from ladder 0.
pub fn charger_chg_active(ladder0_raw: u16) -> Option<bool> {
    ladder::decode(ladder0_raw, &ladder::LADDER0_LEVELS).map(|f| f.has(LadderFlags::CH2))
}

/// Events produced by [`ButtonTracker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ButtonEvent {
    /// The debounced state changed.
    Changed(Buttons),
    /// The center button has been held for the power-off time.
    PowerOffRequested,
}

/// Debounces button samples and detects the long center press that
/// powers the hub off.
///
/// The center press that switched the hub on is usually still held when
/// the firmware starts, so a long press only counts after the center
/// button has been released once.
#[derive(Debug, Clone)]
pub struct ButtonTracker {
    stable: Buttons,
    candidate: Buttons,
    candidate_count: u8,
    settled: bool,
    center_released_since_boot: bool,
    center_held_ms: u32,
    power_off_sent: bool,
}

impl ButtonTracker {
    /// Number of identical consecutive samples needed to accept a change.
    pub const DEBOUNCE_SAMPLES: u8 = 3;
    /// Hold time of the center button that requests power-off.
    pub const POWER_OFF_HOLD_MS: u32 = 2000;

    /// New tracker, all buttons released.
    pub const fn new() -> Self {
        Self {
            stable: Buttons {
                center: false,
                left: false,
                right: false,
                bluetooth: false,
            },
            candidate: Buttons {
                center: false,
                left: false,
                right: false,
                bluetooth: false,
            },
            candidate_count: 0,
            settled: false,
            center_released_since_boot: false,
            center_held_ms: 0,
            power_off_sent: false,
        }
    }

    /// The current debounced state.
    pub fn state(&self) -> Buttons {
        self.stable
    }

    /// Feeds one sample taken `dt_ms` after the previous one.
    pub fn update(&mut self, sample: Buttons, dt_ms: u32) -> Option<ButtonEvent> {
        let mut event = None;
        if sample == self.candidate {
            self.candidate_count = self.candidate_count.saturating_add(1);
        } else {
            self.candidate = sample;
            self.candidate_count = 1;
        }
        if self.candidate_count >= Self::DEBOUNCE_SAMPLES {
            self.settled = true;
            if self.candidate != self.stable {
                self.stable = self.candidate;
                event = Some(ButtonEvent::Changed(self.stable));
            }
        }

        if !self.settled {
            // No debounced state yet: neither a release nor a hold counts.
        } else if !self.stable.center {
            self.center_released_since_boot = true;
            self.center_held_ms = 0;
            self.power_off_sent = false;
        } else if self.center_released_since_boot {
            self.center_held_ms = self.center_held_ms.saturating_add(dt_ms);
            if self.center_held_ms >= Self::POWER_OFF_HOLD_MS && !self.power_off_sent {
                self.power_off_sent = true;
                return Some(ButtonEvent::PowerOffRequested);
            }
        }
        event
    }
}

impl Default for ButtonTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CENTER: Buttons = Buttons {
        center: true,
        left: false,
        right: false,
        bluetooth: false,
    };

    #[test]
    fn maps_ladders_to_buttons() {
        assert_eq!(from_adc(4000, 4000), Some(Buttons::default()));
        let b = from_adc(3000, 2600).unwrap(); // l0: CH1, l1: CH0
        assert!(b.center && b.left && !b.right && !b.bluetooth);
        assert_eq!(from_adc(0, 4000), None);
        assert_eq!(charger_chg_active(3500), Some(true));
    }

    #[test]
    fn debounces() {
        let mut t = ButtonTracker::new();
        assert_eq!(t.update(CENTER, 10), None);
        assert_eq!(t.update(Buttons::default(), 10), None);
        assert_eq!(t.update(CENTER, 10), None);
        assert_eq!(t.update(CENTER, 10), None);
        assert_eq!(t.update(CENTER, 10), Some(ButtonEvent::Changed(CENTER)));
    }

    #[test]
    fn boot_press_does_not_power_off() {
        let mut t = ButtonTracker::new();
        for _ in 0..500 {
            assert_ne!(t.update(CENTER, 10), Some(ButtonEvent::PowerOffRequested));
        }
    }

    #[test]
    fn long_press_powers_off_once() {
        let mut t = ButtonTracker::new();
        for _ in 0..5 {
            t.update(Buttons::default(), 10);
        }
        let mut count = 0;
        for _ in 0..500 {
            if t.update(CENTER, 10) == Some(ButtonEvent::PowerOffRequested) {
                count += 1;
            }
        }
        assert_eq!(count, 1);
    }
}
