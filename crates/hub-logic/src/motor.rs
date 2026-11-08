//! H-bridge control for the six motor ports.
//!
//! Port of Pybricks `lib/pbio/drv/motor_driver/motor_driver_hbridge_pwm.c`
//! (commit `c860b6a`, MIT). Each port has two H-bridge inputs. The timer
//! channels are configured with inverted polarity, so a channel drives its
//! pin LOW for `duty` of each period:
//!
//! | Mode | Pin 1 | Pin 2 |
//! |------|-------|-------|
//! | coast | LOW | LOW |
//! | brake | HIGH | HIGH |
//! | forward `d` | PWM `d` | HIGH |
//! | reverse `d` | HIGH | PWM `d` |

/// Full scale of a duty cycle (Pybricks `PBDRV_MOTOR_DRIVER_MAX_DUTY`).
pub const MAX_DUTY: i16 = 1000;

/// Requested output of one H-bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Drive {
    /// Both inputs LOW, the motor spins freely.
    Coast,
    /// Both inputs HIGH, the motor is short-circuited (passive brake).
    Brake,
    /// Signed duty cycle −[`MAX_DUTY`]..=[`MAX_DUTY`]; positive = forward.
    /// A duty of 0 brakes, like Pybricks.
    Duty(i16),
}

/// What one H-bridge input pin must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PinState {
    /// GPIO output LOW.
    Low,
    /// GPIO output HIGH.
    High,
    /// Timer alternate function with this duty (0..=MAX_DUTY).
    Pwm(u16),
}

/// Pin states (pin 1, pin 2) for a drive request.
pub fn pin_states(drive: Drive) -> (PinState, PinState) {
    match drive {
        Drive::Coast => (PinState::Low, PinState::Low),
        Drive::Brake | Drive::Duty(0) => (PinState::High, PinState::High),
        Drive::Duty(d) if d > 0 => (PinState::Pwm(d.min(MAX_DUTY) as u16), PinState::High),
        Drive::Duty(d) => (
            PinState::High,
            PinState::Pwm(d.max(-MAX_DUTY).unsigned_abs()),
        ),
    }
}

/// Converts a duty (0..=MAX_DUTY) to a timer compare value for a timer
/// whose period is `max_compare` counts.
pub fn compare_value(duty: u16, max_compare: u32) -> u32 {
    u32::from(duty.min(MAX_DUTY as u16)) * max_compare / MAX_DUTY as u32
}

/// Converts a speed in percent (−100..=100, `ekips-hub-api`) to a duty.
pub fn percent_to_duty(percent: f32) -> i16 {
    let p = if percent.is_nan() {
        0.0
    } else {
        percent.clamp(-100.0, 100.0)
    };
    (p * f32::from(MAX_DUTY) / 100.0) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_match_pybricks() {
        assert_eq!(pin_states(Drive::Coast), (PinState::Low, PinState::Low));
        assert_eq!(pin_states(Drive::Brake), (PinState::High, PinState::High));
        assert_eq!(pin_states(Drive::Duty(0)), (PinState::High, PinState::High));
        assert_eq!(
            pin_states(Drive::Duty(300)),
            (PinState::Pwm(300), PinState::High)
        );
        assert_eq!(
            pin_states(Drive::Duty(-300)),
            (PinState::High, PinState::Pwm(300))
        );
        assert_eq!(
            pin_states(Drive::Duty(i16::MIN)),
            (PinState::High, PinState::Pwm(1000))
        );
        assert_eq!(
            pin_states(Drive::Duty(5000)),
            (PinState::Pwm(1000), PinState::High)
        );
    }

    #[test]
    fn scaling() {
        assert_eq!(compare_value(500, 8000), 4000);
        assert_eq!(compare_value(2000, 8000), 8000);
        assert_eq!(percent_to_duty(50.0), 500);
        assert_eq!(percent_to_duty(-150.0), -1000);
        assert_eq!(percent_to_duty(f32::NAN), 0);
    }
}
