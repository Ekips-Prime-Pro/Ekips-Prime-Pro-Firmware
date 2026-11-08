//! LSM6DS3TR-C 6-axis IMU: register setup, data conversion and heading.
//!
//! Configuration follows Pybricks `lib/pbio/drv/imu/imu_lsm6ds3tr_c_stm32.c`
//! (commit `c860b6a`, MIT): 833 Hz, ±8 g, ±2000 dps, block data update,
//! gyro data-ready on INT1 (pulsed), rounding over gyro + accel. Register
//! values from the ST datasheet / ST's `lsm6ds3tr_c_reg.h`.

/// 7-bit I2C address (SA0 = 0). Pybricks uses the 8-bit form `0xD5`.
pub const I2C_ADDR: u8 = 0x6A;
/// Expected value of `WHO_AM_I`.
pub const WHO_AM_I_VALUE: u8 = 0x6A;

/// Register addresses.
pub mod reg {
    /// Data-ready pulse configuration.
    pub const DRDY_PULSE_CFG_G: u8 = 0x0B;
    /// INT1 routing.
    pub const INT1_CTRL: u8 = 0x0D;
    /// Device ID.
    pub const WHO_AM_I: u8 = 0x0F;
    /// Accelerometer control.
    pub const CTRL1_XL: u8 = 0x10;
    /// Gyroscope control.
    pub const CTRL2_G: u8 = 0x11;
    /// Control 3 (reset, BDU, auto-increment).
    pub const CTRL3_C: u8 = 0x12;
    /// Control 5 (rounding).
    pub const CTRL5_C: u8 = 0x14;
    /// First gyro output register; 12 bytes gyro XYZ + accel XYZ follow.
    pub const OUTX_L_G: u8 = 0x22;
}

/// `CTRL3_C`: software reset.
pub const CTRL3_SW_RESET: u8 = 0x01;
/// `CTRL3_C`: block data update + register auto-increment.
pub const CTRL3_BDU_IF_INC: u8 = 0x44;

/// Initialization writes after the reset, in order.
pub const INIT_SEQUENCE: [(u8, u8); 6] = [
    (reg::CTRL3_C, CTRL3_BDU_IF_INC),
    // ODR 833 Hz (0111), FS ±8 g (11)
    (reg::CTRL1_XL, 0x7C),
    // ODR 833 Hz (0111), FS ±2000 dps (11)
    (reg::CTRL2_G, 0x7C),
    // INT1_DRDY_G
    (reg::INT1_CTRL, 0x02),
    // DRDY_PULSED
    (reg::DRDY_PULSE_CFG_G, 0x80),
    // ROUNDING = gyro + accel (011)
    (reg::CTRL5_C, 0x60),
];

/// Output data rate in Hz.
pub const ODR_HZ: f32 = 833.0;
/// Gyro sensitivity at ±2000 dps in degrees/s per LSB.
pub const GYRO_DPS_PER_LSB: f32 = 0.070;
/// Accelerometer sensitivity at ±8 g in m/s² per LSB.
pub const ACCEL_MS2_PER_LSB: f32 = 0.000_244 * 9.81;

/// Axis signs mapping chip axes to hub axes. Pybricks `pbdrvconfig.h`
/// L82-84 (`SIGN_X = -1`, `SIGN_Y = 1`, `SIGN_Z = -1`).
pub const AXIS_SIGN: [f32; 3] = [-1.0, 1.0, -1.0];

/// One converted sample in hub coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Sample {
    /// Angular rate in degrees/s (x, y, z).
    pub gyro_dps: [f32; 3],
    /// Acceleration in m/s² (x, y, z).
    pub accel_ms2: [f32; 3],
}

/// Converts the 12 raw bytes read from `OUTX_L_G`.
pub fn convert(raw: &[u8; 12]) -> Sample {
    let w = |i: usize| f32::from(i16::from_le_bytes([raw[i * 2], raw[i * 2 + 1]]));
    let mut s = Sample::default();
    for (axis, sign) in AXIS_SIGN.iter().enumerate() {
        s.gyro_dps[axis] = w(axis) * GYRO_DPS_PER_LSB * sign;
        s.accel_ms2[axis] = w(axis + 3) * ACCEL_MS2_PER_LSB * sign;
    }
    s
}

/// Integrates the vertical gyro axis to a heading and learns the gyro
/// bias while the hub is still.
///
/// Sign convention of `ekips-hub-api`: positive = clockwise seen from
/// above. With hub z pointing up, a clockwise turn is a negative z rate,
/// hence the minus sign. **To be verified on hardware.**
#[derive(Debug, Clone)]
pub struct HeadingTracker {
    heading_deg: f32,
    bias_dps: f32,
    still_samples: u32,
    still_sum: f32,
    calibrated: bool,
}

impl HeadingTracker {
    /// Rate below which the hub counts as still (deg/s).
    pub const STILL_THRESHOLD_DPS: f32 = 1.5;
    /// Number of still samples averaged for the bias (~1 s at 833 Hz).
    pub const BIAS_SAMPLES: u32 = 833;

    /// New tracker, heading 0, no bias yet.
    pub const fn new() -> Self {
        Self {
            heading_deg: 0.0,
            bias_dps: 0.0,
            still_samples: 0,
            still_sum: 0.0,
            calibrated: false,
        }
    }

    /// Current heading in degrees (clockwise positive, not wrapped).
    pub fn heading(&self) -> f32 {
        self.heading_deg
    }

    /// Whether a gyro bias has been measured.
    pub fn calibrated(&self) -> bool {
        self.calibrated
    }

    /// Sets the heading to zero.
    pub fn reset(&mut self) {
        self.heading_deg = 0.0;
    }

    /// Feeds one sample taken `dt_s` after the previous one.
    pub fn update(&mut self, s: &Sample, dt_s: f32) {
        let z = s.gyro_dps[2];
        let [x, y, _] = s.gyro_dps;
        let still = x.abs() < Self::STILL_THRESHOLD_DPS
            && y.abs() < Self::STILL_THRESHOLD_DPS
            && (z - self.bias_dps).abs() < Self::STILL_THRESHOLD_DPS;
        if still {
            self.still_sum += z;
            self.still_samples += 1;
            if self.still_samples >= Self::BIAS_SAMPLES {
                self.bias_dps = self.still_sum / self.still_samples as f32;
                self.calibrated = true;
                self.still_samples = 0;
                self.still_sum = 0.0;
            }
        } else {
            self.still_samples = 0;
            self.still_sum = 0.0;
        }
        let rate = z - self.bias_dps;
        // Ignore the residual noise while still so the heading does not drift.
        if !still || !self.calibrated {
            self.heading_deg -= rate * dt_s;
        }
    }
}

impl Default for HeadingTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_with_gyro_z(z: i16) -> [u8; 12] {
        let mut r = [0u8; 12];
        r[4..6].copy_from_slice(&z.to_le_bytes());
        r
    }

    #[test]
    fn conversion_scales_and_signs() {
        let s = convert(&raw_with_gyro_z(1000));
        assert!((s.gyro_dps[2] + 70.0).abs() < 1e-3); // SIGN_Z = -1
        let mut r = [0u8; 12];
        r[6..8].copy_from_slice(&4096i16.to_le_bytes()); // accel x
        let s = convert(&r);
        assert!((s.accel_ms2[0] + 4096.0 * ACCEL_MS2_PER_LSB).abs() < 1e-3);
    }

    #[test]
    fn integrates_clockwise_turn() {
        let mut h = HeadingTracker::new();
        let s = Sample {
            gyro_dps: [0.0, 0.0, -90.0], // clockwise from above
            accel_ms2: [0.0, 0.0, 9.81],
        };
        for _ in 0..833 {
            h.update(&s, 1.0 / ODR_HZ);
        }
        assert!((h.heading() - 90.0).abs() < 0.5, "{}", h.heading());
        h.reset();
        assert_eq!(h.heading(), 0.0);
    }

    #[test]
    fn learns_bias_and_stops_drifting() {
        let mut h = HeadingTracker::new();
        let s = Sample {
            gyro_dps: [0.0, 0.0, 0.5],
            accel_ms2: [0.0, 0.0, 9.81],
        };
        for _ in 0..HeadingTracker::BIAS_SAMPLES {
            h.update(&s, 1.0 / ODR_HZ);
        }
        assert!(h.calibrated());
        h.reset();
        for _ in 0..10_000 {
            h.update(&s, 1.0 / ODR_HZ);
        }
        assert_eq!(h.heading(), 0.0);
    }
}
