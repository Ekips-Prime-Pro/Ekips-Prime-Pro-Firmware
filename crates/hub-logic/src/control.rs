//! Simple motor control on top of the encoder angle from LUMP.
//!
//! This is deliberately basic (a proportional position controller with a
//! duty limit and a speed estimator). Pybricks uses a full PID with
//! trajectory generation; replacing this with something comparable is
//! future work.

use crate::motor::MAX_DUTY;

/// Motor speed that counts as 100 % (degrees per second). SPIKE motors
/// reach roughly 1000 °/s unloaded. **Approximation.**
pub const FULL_SPEED_DPS: f32 = 1000.0;
/// Position error below which a move counts as done (degrees).
pub const DONE_TOLERANCE_DEG: f32 = 3.0;
/// Proportional gain: duty per degree of error.
pub const KP_DUTY_PER_DEG: f32 = 12.0;
/// Minimum duty that overcomes friction while still moving.
pub const MIN_MOVING_DUTY: i16 = 120;

/// Converts degrees per second to percent of [`FULL_SPEED_DPS`].
pub fn dps_to_percent(dps: f32) -> f32 {
    dps * 100.0 / FULL_SPEED_DPS
}

/// Duty for a speed in percent, used as the limit of a move.
pub fn speed_limit_duty(speed_pct: f32) -> i16 {
    let p = if speed_pct.is_nan() {
        0.0
    } else {
        speed_pct.abs().min(100.0)
    };
    (p * f32::from(MAX_DUTY) / 100.0) as i16
}

/// One step of the position controller. Returns `(duty, done)`.
pub fn position_step(angle_deg: f32, target_deg: f32, limit_duty: i16) -> (i16, bool) {
    let error = target_deg - angle_deg;
    if error.abs() <= DONE_TOLERANCE_DEG {
        return (0, true);
    }
    let limit = limit_duty.clamp(0, MAX_DUTY);
    let raw = (error * KP_DUTY_PER_DEG) as i32;
    let mut duty = raw.clamp(-i32::from(limit), i32::from(limit)) as i16;
    if duty.abs() < MIN_MOVING_DUTY.min(limit) {
        let sign = if error > 0.0 { 1 } else { -1 };
        duty = MIN_MOVING_DUTY.min(limit) * sign;
    }
    (duty, false)
}

/// Low-pass filtered speed from successive angle readings.
#[derive(Debug, Clone, Default)]
pub struct SpeedEstimator {
    last: Option<(f32, u64)>,
    speed_dps: f32,
}

impl SpeedEstimator {
    /// New estimator.
    pub const fn new() -> Self {
        Self {
            last: None,
            speed_dps: 0.0,
        }
    }

    /// Feeds an angle measured at `t_ms`. Returns the speed in °/s.
    pub fn update(&mut self, angle_deg: f32, t_ms: u64) -> f32 {
        if let Some((a, t)) = self.last
            && t_ms > t
        {
            let raw = (angle_deg - a) * 1000.0 / (t_ms - t) as f32;
            self.speed_dps += (raw - self.speed_dps) * 0.3;
        }
        self.last = Some((angle_deg, t_ms));
        self.speed_dps
    }

    /// Last estimate in °/s.
    pub fn speed(&self) -> f32 {
        self.speed_dps
    }
}

/// Wheel rotation in degrees for driving `distance_mm`.
pub fn wheel_degrees_for_distance(distance_mm: f32, wheel_diameter_mm: f32) -> f32 {
    if wheel_diameter_mm <= 0.0 {
        return 0.0;
    }
    distance_mm * 360.0 / (core::f32::consts::PI * wheel_diameter_mm)
}

/// Wheel rotation in degrees (each wheel, opposite directions) for turning
/// the robot on the spot by `robot_deg`.
pub fn wheel_degrees_for_turn(robot_deg: f32, axle_track_mm: f32, wheel_diameter_mm: f32) -> f32 {
    let arc_mm = core::f32::consts::PI * axle_track_mm * robot_deg / 360.0;
    wheel_degrees_for_distance(arc_mm, wheel_diameter_mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controller_moves_towards_target_and_stops() {
        assert_eq!(position_step(0.0, 1.0, 500), (0, true));
        let (d, done) = position_step(0.0, 90.0, 500);
        assert!(!done && d == 500);
        let (d, _) = position_step(100.0, 90.0, 500);
        assert!(d < 0);
        // Small error still gets enough duty to move.
        let (d, _) = position_step(0.0, 5.0, 1000);
        assert!(d >= MIN_MOVING_DUTY);
        // A limit below the minimum duty is respected.
        let (d, _) = position_step(0.0, 5.0, 50);
        assert_eq!(d, 50);
        let (d, _) = position_step(0.0, -4.0, 1000);
        assert_eq!(d, -MIN_MOVING_DUTY);
    }

    #[test]
    fn speed_estimate_converges() {
        let mut s = SpeedEstimator::new();
        for i in 0..50u64 {
            s.update(i as f32 * 5.0, i * 10); // 500 deg/s
        }
        assert!((s.speed() - 500.0).abs() < 5.0);
        assert!((dps_to_percent(500.0) - 50.0).abs() < 1e-3);
    }

    #[test]
    fn drive_geometry() {
        // 56 mm wheel: one revolution = 175.9 mm
        let deg = wheel_degrees_for_distance(175.929, 56.0);
        assert!((deg - 360.0).abs() < 0.1);
        // Turning 360° with 112 mm track = one circle of 351.9 mm per wheel.
        let deg = wheel_degrees_for_turn(360.0, 112.0, 56.0);
        assert!((deg - 720.0).abs() < 0.5);
        assert_eq!(speed_limit_duty(-50.0), 500);
    }
}
