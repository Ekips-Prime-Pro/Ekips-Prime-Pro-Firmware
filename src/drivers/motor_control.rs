//! Closed-loop motor control at 100 Hz using the LUMP encoder angle.
//!
//! Commands per port: open-loop duty, move to a target angle with a duty
//! limit, or hold the current angle. The control law is the simple
//! proportional controller from `ekips_hub_logic::control`.
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_api::Port;
use ekips_hub_logic::control::{self, SpeedEstimator};
use ekips_hub_logic::lump::{self, DeviceClass};
use ekips_hub_logic::motor::Drive;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Instant, Ticker};

use super::{lump as lump_drv, motors};

/// Control period.
pub const PERIOD_MS: u64 = 10;
/// Duty limit while holding a position.
const HOLD_LIMIT_DUTY: i16 = 300;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Command {
    Coast,
    Duty(i16),
    MoveTo { target: f32, limit: i16 },
    Hold { target: f32 },
}

#[derive(Debug, Clone)]
struct MotorState {
    command: Command,
    offset_deg: f32,
    angle_deg: Option<f32>,
    estimator: SpeedEstimator,
}

impl MotorState {
    const fn new() -> Self {
        Self {
            command: Command::Coast,
            offset_deg: 0.0,
            angle_deg: None,
            estimator: SpeedEstimator::new(),
        }
    }
}

static STATE: Mutex<CriticalSectionRawMutex, RefCell<[MotorState; 6]>> =
    Mutex::new(RefCell::new([const { MotorState::new() }; 6]));

fn with<R>(port: Port, f: impl FnOnce(&mut MotorState) -> R) -> R {
    STATE.lock(|s| f(&mut s.borrow_mut()[port.index()]))
}

/// Angle in degrees since the last reset (`None` without encoder data).
pub fn angle(port: Port) -> Option<f32> {
    with(port, |m| m.angle_deg)
}

/// Speed in degrees per second.
pub fn speed_dps(port: Port) -> f32 {
    with(port, |m| m.estimator.speed())
}

/// Sets the current angle to zero (keeps a running move consistent).
pub fn reset_angle(port: Port) {
    with(port, |m| {
        if let Some(a) = m.angle_deg {
            m.offset_deg += a;
            m.command = match m.command {
                Command::MoveTo { target, limit } => Command::MoveTo {
                    target: target - a,
                    limit,
                },
                Command::Hold { target } => Command::Hold { target: target - a },
                c => c,
            };
            m.angle_deg = Some(0.0);
        }
    });
}

/// Runs open loop with a signed duty (−1000..=1000).
pub fn run_duty(port: Port, duty: i16) {
    with(port, |m| m.command = Command::Duty(duty));
}

/// Moves by `delta_deg` relative to the current angle with a duty limit.
pub fn run_by(port: Port, delta_deg: f32, limit: i16) {
    with(port, |m| {
        let base = match m.command {
            Command::MoveTo { target, .. } | Command::Hold { target } => target,
            _ => m.angle_deg.unwrap_or(0.0),
        };
        m.command = Command::MoveTo {
            target: base + delta_deg,
            limit,
        };
    });
}

/// Stops and holds the current angle (brakes if there is no encoder).
pub fn stop(port: Port) {
    with(port, |m| {
        m.command = match m.angle_deg {
            Some(a) => Command::Hold { target: a },
            None => Command::Duty(0),
        };
    });
}

/// Coasts the motor.
pub fn coast(port: Port) {
    with(port, |m| m.command = Command::Coast);
}

/// Whether the last `run_by` has reached its target.
pub fn done(port: Port) -> bool {
    with(port, |m| !matches!(m.command, Command::MoveTo { .. }))
}

/// Runs the controller forever.
#[embassy_executor::task]
pub async fn task() {
    let mut ticker = Ticker::every(Duration::from_millis(PERIOD_MS));
    loop {
        ticker.next().await;
        let now_ms = Instant::now().as_millis();
        for port in Port::ALL {
            let s = lump_drv::state(port);
            let encoder = (s.class == Some(DeviceClass::AngularMotor)
                && s.mode == lump::mode::MOTOR_POS)
                .then(|| lump::i32_at(s.data.as_slice(), 0))
                .flatten();
            let drive = with(port, |m| {
                m.angle_deg = encoder.map(|raw| raw as f32 - m.offset_deg);
                if let Some(a) = m.angle_deg {
                    m.estimator.update(a, now_ms);
                }
                match (m.command, m.angle_deg) {
                    (Command::Coast, _) => Drive::Coast,
                    (Command::Duty(d), _) => Drive::Duty(d),
                    (Command::MoveTo { target, limit }, Some(a)) => {
                        let (duty, done) = control::position_step(a, target, limit);
                        if done {
                            m.command = Command::Hold { target };
                        }
                        Drive::Duty(duty)
                    }
                    (Command::Hold { target }, Some(a)) => {
                        Drive::Duty(control::position_step(a, target, HOLD_LIMIT_DUTY).0)
                    }
                    // No encoder data: never drive blindly towards a target.
                    (Command::MoveTo { .. } | Command::Hold { .. }, None) => Drive::Brake,
                }
            });
            motors::set(port, drive);
        }
    }
}
