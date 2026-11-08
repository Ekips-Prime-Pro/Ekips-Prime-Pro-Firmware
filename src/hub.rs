//! Implementation of `ekips_hub_api::Hardware` for the SPIKE Prime hub.
//!
//! **Skeleton, untested on hardware.** It maps the VM's calls onto the
//! drivers that exist so far. Known limitations:
//!
//! - Motor control is a simple proportional controller
//!   (`drivers::motor_control`), not Pybricks-grade PID.
//! - Drive base: the left motor is assumed to be mounted mirrored, so
//!   "forward" is negative rotation for the left and positive for the right
//!   wheel ([`LEFT_WHEEL_SIGN`]). To be verified on a real robot.
//! - Sensors report the value of their *current* mode. Reading a value of
//!   another mode (e.g. `reflection` while the color sensor is in color
//!   mode) requests a mode switch and returns a neutral value until data of
//!   the new mode arrives.

use ekips_hub_api::{Color, DeviceKind, DriveBaseConfig, Hardware, HardwareError, Port};
use ekips_hub_logic::control;
use ekips_hub_logic::lump::{self, DeviceClass, SpikeColor};
use embassy_time::Instant;

use crate::drivers::{battery, imu, lump as lump_drv, matrix, motor_control, speaker, usb};

/// Rotation sign of the left drive motor for driving forward.
pub const LEFT_WHEEL_SIGN: f32 = -1.0;
/// Rotation sign of the right drive motor for driving forward.
pub const RIGHT_WHEEL_SIGN: f32 = 1.0;

/// The SPIKE Prime hub as seen by the VM.
pub struct SpikeHub {
    start: Instant,
    drive: Option<DriveBaseConfig>,
}

impl SpikeHub {
    /// Creates the hub; the program clock starts now.
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            drive: None,
        }
    }

    fn device(&self, port: Port, kind: DeviceKind) -> Result<lump_drv::PortState, HardwareError> {
        let s = lump_drv::state(port);
        let class = s.class.ok_or(HardwareError::NoDevice(port))?;
        let ok = matches!(
            (kind, class),
            (DeviceKind::Motor, DeviceClass::AngularMotor)
                | (DeviceKind::ColorSensor, DeviceClass::ColorSensor)
                | (DeviceKind::DistanceSensor, DeviceClass::DistanceSensor)
                | (DeviceKind::ForceSensor, DeviceClass::ForceSensor)
        );
        if ok {
            Ok(s)
        } else {
            Err(HardwareError::WrongDevice {
                port,
                expected: kind,
            })
        }
    }

    /// Returns the payload if the device is in `mode`, otherwise asks it
    /// to switch and returns `None`.
    fn mode_data(
        &self,
        port: Port,
        kind: DeviceKind,
        mode: u8,
    ) -> Result<Option<lump::Payload>, HardwareError> {
        let s = self.device(port, kind)?;
        if s.mode == mode {
            Ok(Some(s.data))
        } else {
            lump_drv::select_mode(port, mode);
            Ok(None)
        }
    }

    fn drive_base(&self) -> Result<DriveBaseConfig, HardwareError> {
        self.drive.ok_or(HardwareError::NoDriveBase)
    }
}

impl Default for SpikeHub {
    fn default() -> Self {
        Self::new()
    }
}

fn color_from(c: Option<SpikeColor>) -> Color {
    match c {
        None => Color::None,
        Some(SpikeColor::Black) => Color::Black,
        Some(SpikeColor::Violet) => Color::Violet,
        Some(SpikeColor::Blue) => Color::Blue,
        Some(SpikeColor::Azure) => Color::Azure,
        Some(SpikeColor::Green) => Color::Green,
        Some(SpikeColor::Yellow) => Color::Yellow,
        Some(SpikeColor::Red) => Color::Red,
        Some(SpikeColor::White) => Color::White,
    }
}

impl Hardware for SpikeHub {
    fn now_ms(&self) -> u64 {
        (Instant::now() - self.start).as_millis()
    }

    fn check_device(&self, port: Port, kind: DeviceKind) -> Result<(), HardwareError> {
        self.device(port, kind).map(|_| ())
    }

    fn motor_run_angle(
        &mut self,
        port: Port,
        degrees: f32,
        speed_pct: f32,
    ) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        let delta = if speed_pct < 0.0 { -degrees } else { degrees };
        motor_control::run_by(port, delta, control::speed_limit_duty(speed_pct));
        Ok(())
    }

    fn motor_run(&mut self, port: Port, speed_pct: f32) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        motor_control::run_duty(port, ekips_hub_logic::motor::percent_to_duty(speed_pct));
        Ok(())
    }

    fn motor_stop(&mut self, port: Port) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        motor_control::stop(port);
        Ok(())
    }

    fn motor_done(&self, port: Port) -> bool {
        motor_control::done(port)
    }

    fn motor_angle(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        Ok(motor_control::angle(port).unwrap_or(0.0))
    }

    fn motor_speed(&self, port: Port) -> Result<f32, HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        Ok(control::dps_to_percent(motor_control::speed_dps(port)))
    }

    fn motor_reset_angle(&mut self, port: Port) -> Result<(), HardwareError> {
        self.device(port, DeviceKind::Motor)?;
        motor_control::reset_angle(port);
        Ok(())
    }

    fn drive_configure(&mut self, config: DriveBaseConfig) -> Result<(), HardwareError> {
        self.device(config.left, DeviceKind::Motor)?;
        self.device(config.right, DeviceKind::Motor)?;
        self.drive = Some(config);
        Ok(())
    }

    fn drive_straight(&mut self, distance_mm: f32, speed_pct: f32) -> Result<(), HardwareError> {
        let c = self.drive_base()?;
        let deg = control::wheel_degrees_for_distance(distance_mm, c.wheel_diameter_mm);
        let limit = control::speed_limit_duty(speed_pct);
        motor_control::run_by(c.left, deg * LEFT_WHEEL_SIGN, limit);
        motor_control::run_by(c.right, deg * RIGHT_WHEEL_SIGN, limit);
        Ok(())
    }

    fn drive_turn(&mut self, degrees: f32, speed_pct: f32) -> Result<(), HardwareError> {
        let c = self.drive_base()?;
        let deg = control::wheel_degrees_for_turn(degrees, c.axle_track_mm, c.wheel_diameter_mm);
        let limit = control::speed_limit_duty(speed_pct);
        // Clockwise: left wheel forward, right wheel backward.
        motor_control::run_by(c.left, deg * LEFT_WHEEL_SIGN, limit);
        motor_control::run_by(c.right, -deg * RIGHT_WHEEL_SIGN, limit);
        Ok(())
    }

    fn drive_done(&self) -> bool {
        match self.drive {
            Some(c) => motor_control::done(c.left) && motor_control::done(c.right),
            None => true,
        }
    }

    fn drive_stop(&mut self) -> Result<(), HardwareError> {
        let c = self.drive_base()?;
        motor_control::stop(c.left);
        motor_control::stop(c.right);
        Ok(())
    }

    fn heading(&self) -> f32 {
        imu::heading()
    }

    fn reset_heading(&mut self) {
        imu::reset_heading();
    }

    fn color(&self, port: Port) -> Result<Color, HardwareError> {
        let data = self.mode_data(port, DeviceKind::ColorSensor, lump::mode::COLOR_COLOR)?;
        Ok(color_from(
            data.and_then(|d| lump::i8_at(d.as_slice(), 0))
                .and_then(lump::spike_color),
        ))
    }

    fn reflection(&self, port: Port) -> Result<f32, HardwareError> {
        let data = self.mode_data(port, DeviceKind::ColorSensor, lump::mode::COLOR_REFLT)?;
        Ok(data
            .and_then(|d| lump::i8_at(d.as_slice(), 0))
            .map_or(0.0, f32::from))
    }

    fn distance_mm(&self, port: Port) -> Result<Option<f32>, HardwareError> {
        let data = self.mode_data(port, DeviceKind::DistanceSensor, lump::mode::DIST_DISTL)?;
        Ok(data
            .and_then(|d| lump::i16_at(d.as_slice(), 0))
            .filter(|&mm| mm >= 0)
            .map(f32::from))
    }

    fn pressed(&self, port: Port) -> Result<bool, HardwareError> {
        Ok(self.force_n(port)? > 0.5)
    }

    fn force_n(&self, port: Port) -> Result<f32, HardwareError> {
        // LEGO mode 0 reports the force in percent of 10 N (unverified).
        let data = self.mode_data(port, DeviceKind::ForceSensor, lump::mode::FORCE_FORCE)?;
        Ok(data
            .and_then(|d| lump::i8_at(d.as_slice(), 0))
            .map_or(0.0, |v| f32::from(v.max(0)) / 10.0))
    }

    fn show(&mut self, text: &str) {
        matrix::show_text(text);
    }

    fn beep(&mut self, frequency_hz: u16, duration_ms: u16) {
        speaker::beep(frequency_hz, duration_ms);
    }

    fn battery_pct(&self) -> f32 {
        battery::state().map_or(0.0, |s| f32::from(s.percent))
    }

    fn print(&mut self, text: &str) {
        usb::write(text.as_bytes());
        usb::write(b"\r\n");
    }
}
