//! Battery and charger conversions and the MP2639A charger state machine.
//!
//! Constants from Pybricks (commit `c860b6a`, MIT):
//! `platform/prime_hub_f4/pbdrvconfig.h` L14-26,
//! `drv/battery/battery_adc.c`, `drv/charger/charger_mp2639a.c`,
//! `sys/battery.c` L29-33.

/// Full-scale raw ADC value used by Pybricks' scaling (12 bit).
const RAW_MAX: u32 = 4096;
/// Battery voltage at raw full scale, mV (`PBDRV_CONFIG_BATTERY_ADC_VOLTAGE_SCALED_MAX`).
const VOLTAGE_SCALED_MAX: u32 = 9900;
/// Battery current at raw full scale, mA (`..._CURRENT_SCALED_MAX`).
const CURRENT_SCALED_MAX: u32 = 7300;
/// Voltage drop correction per mA, in 1/16 mV (`..._CURRENT_CORRECTION`).
const CURRENT_CORRECTION: u32 = 3;

/// Li-ion pack (2 cells) is full at this voltage (`sys/battery.c`).
pub const LIION_FULL_MV: u16 = 8190;
/// Li-ion "ok" level.
pub const LIION_OK_MV: u16 = 7200;
/// Li-ion low level: warn the user.
pub const LIION_LOW_MV: u16 = 6800;
/// Li-ion critical level: shut down.
pub const LIION_CRITICAL_MV: u16 = 6000;

/// Battery current in mA from the raw IBAT reading.
pub fn current_ma(raw: u16) -> u16 {
    (u32::from(raw) * CURRENT_SCALED_MAX / RAW_MAX) as u16
}

/// Battery voltage in mV, corrected for the drop caused by the current
/// (exactly as Pybricks `pbdrv_battery_get_voltage_now`).
pub fn voltage_mv(raw_voltage: u16, current_ma: u16) -> u16 {
    let v = u32::from(raw_voltage) * VOLTAGE_SCALED_MAX / RAW_MAX
        + u32::from(current_ma) * CURRENT_CORRECTION / 16;
    v.min(u32::from(u16::MAX)) as u16
}

/// Rough state of charge in percent from the pack voltage.
///
/// **Approximation** (not from Pybricks, not calibrated): piecewise linear
/// over typical Li-ion points between [`LIION_CRITICAL_MV`] (0 %) and
/// [`LIION_FULL_MV`] (100 %). Under load the voltage drops, so the value
/// is only an estimate.
pub fn percent(voltage_mv: u16) -> u8 {
    const POINTS: [(u16, u8); 6] = [
        (LIION_CRITICAL_MV, 0),
        (LIION_LOW_MV, 10),
        (LIION_OK_MV, 40),
        (7600, 70),
        (7900, 90),
        (LIION_FULL_MV, 100),
    ];
    if voltage_mv <= POINTS[0].0 {
        return 0;
    }
    for w in POINTS.windows(2) {
        let ((v0, p0), (v1, p1)) = (w[0], w[1]);
        if voltage_mv <= v1 {
            let dv = u32::from(voltage_mv - v0);
            let span = u32::from(v1 - v0);
            return p0 + (dv * u32::from(p1 - p0) / span) as u8;
        }
    }
    100
}

/// Battery temperature in °C from the NTC reading (103AT thermistor with
/// 2.4 kΩ / 7.5 kΩ network, B = 3435). Port of Pybricks
/// `pbdrv_battery_get_temperature`. `None` for out-of-range readings.
pub fn temperature_c(raw: u16) -> Option<f32> {
    if raw == 0 || raw >= 4095 {
        return None;
    }
    let raw = f32::from(raw);
    let r_ntc = 1.0 / (((4095.0 / raw - 1.0) / 2400.0) - 1.0 / 7500.0);
    if !(r_ntc.is_finite() && r_ntc > 0.0) {
        return None;
    }
    let t = 3435.0 / (libm::logf(r_ntc / 10_000.0) + 3435.0 / 298.15);
    Some(t - 273.15)
}

/// USB charger input current in mA (empirical Pybricks scaling).
pub fn usb_charge_current_ma(raw: u16) -> i32 {
    ((i32::from(raw) * 35116) >> 16) - 123
}

/// Charge current limit, mapped to the ISET PWM duty (percent of 96 kHz).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChargeLimit {
    /// Charger off.
    None,
    /// 100 mA.
    StdMin,
    /// 500 mA, standard USB port.
    StdMax,
    /// Maximum, dedicated charger.
    Charging,
}

impl ChargeLimit {
    /// ISET duty cycle in percent (Pybricks `pbdrv_charger_enable`).
    pub fn iset_duty_percent(self) -> u8 {
        match self {
            ChargeLimit::None => 0,
            ChargeLimit::StdMin => 2,
            ChargeLimit::StdMax => 15,
            ChargeLimit::Charging => 100,
        }
    }
}

/// Charger status as reported to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ChargerStatus {
    /// Not charging (no USB, or paused).
    Discharge,
    /// Charging.
    Charge,
    /// Battery full.
    Complete,
    /// Charger fault (`/CHG` blinking).
    Fault,
}

/// What the charger driver must do after a [`ChargerMonitor::update`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChargerOutput {
    /// Whether the MP2639A is enabled (MODE pin LOW).
    pub enable: bool,
    /// Current limit to apply.
    pub limit: ChargeLimit,
    /// Status for the UI.
    pub status: ChargerStatus,
}

/// Port of the Pybricks MP2639A process (`charger_mp2639a.c`).
///
/// Call [`update`](Self::update) every [`SAMPLE_MS`](Self::SAMPLE_MS).
/// `/CHG` blinks at 1 Hz on a fault, which is detected by counting
/// transitions in a window of 7 samples. After one hour of continuous
/// charging, charging pauses for 30 s so the chip's safety timer cannot
/// expire while the hub is in use.
#[derive(Debug, Clone)]
pub struct ChargerMonitor {
    samples: [bool; 7],
    index: usize,
    charge_count: u32,
    pause_left: u32,
}

impl ChargerMonitor {
    /// Sample period.
    pub const SAMPLE_MS: u32 = 250;
    /// Continuous charge time before a pause.
    pub const CHARGE_TIMEOUT_MS: u32 = 60 * 60 * 1000;
    /// Length of the pause.
    pub const CHARGE_PAUSE_MS: u32 = 30 * 1000;

    /// New monitor.
    pub const fn new() -> Self {
        Self {
            samples: [false; 7],
            index: 0,
            charge_count: 0,
            pause_left: 0,
        }
    }

    /// Feeds one sample. `usb_present`: VBUS is there. `limit_when_usb`:
    /// the limit to use with USB (without USB charger detection, Pybricks'
    /// "standard downstream port" value `StdMax` is the safe choice).
    /// `chg_active`: `/CHG` is LOW (from the resistor ladder).
    pub fn update(
        &mut self,
        usb_present: bool,
        limit_when_usb: ChargeLimit,
        chg_active: bool,
    ) -> ChargerOutput {
        let off = ChargerOutput {
            enable: false,
            limit: ChargeLimit::None,
            status: ChargerStatus::Discharge,
        };
        if self.pause_left > 0 {
            self.pause_left -= 1;
            return off;
        }
        if !usb_present || limit_when_usb == ChargeLimit::None {
            self.charge_count = 0;
            self.samples[self.index] = chg_active;
            self.index = (self.index + 1) % self.samples.len();
            return off;
        }

        self.samples[self.index] = chg_active;
        self.charge_count += 1;
        let n = self.samples.len();
        let mut transitions = usize::from(self.samples[0] != self.samples[n - 1]);
        for i in 1..n {
            transitions += usize::from(self.samples[i] != self.samples[i - 1]);
        }
        let status = if transitions > 2 {
            ChargerStatus::Fault
        } else if self.samples[self.index] {
            ChargerStatus::Charge
        } else if self.charge_count > 2 {
            ChargerStatus::Complete
        } else {
            ChargerStatus::Discharge
        };
        self.index = (self.index + 1) % n;

        if self.charge_count > Self::CHARGE_TIMEOUT_MS / Self::SAMPLE_MS {
            self.charge_count = 0;
            self.pause_left = Self::CHARGE_PAUSE_MS / Self::SAMPLE_MS;
            return off;
        }
        ChargerOutput {
            enable: true,
            limit: limit_when_usb,
            status,
        }
    }
}

impl Default for ChargerMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_match_pybricks_scaling() {
        assert_eq!(current_ma(4096), 7300);
        assert_eq!(current_ma(0), 0);
        // 3300 raw ~ 7976 mV, plus 160 mA * 3 / 16 = 30 mV
        assert_eq!(voltage_mv(3300, 160), 7976 + 30);
        assert_eq!(usb_charge_current_ma(0), -123);
    }

    #[test]
    fn percent_curve() {
        assert_eq!(percent(5000), 0);
        assert_eq!(percent(LIION_CRITICAL_MV), 0);
        assert_eq!(percent(LIION_LOW_MV), 10);
        assert_eq!(percent(LIION_FULL_MV), 100);
        assert_eq!(percent(9000), 100);
        assert!(percent(7400) > 40 && percent(7400) < 70);
    }

    #[test]
    fn temperature_around_room() {
        // At 25 °C R_NTC = 10k. Solve raw for 10k: 4095 / (1 + 2400 * (1/10000 + 1/7500)).
        let raw = (4095.0 / (1.0 + 2400.0 * (1.0 / 10_000.0 + 1.0 / 7500.0))) as u16;
        let t = temperature_c(raw).unwrap();
        assert!((t - 25.0).abs() < 1.0, "t = {t}");
        assert_eq!(temperature_c(0), None);
    }

    #[test]
    fn charger_without_usb_is_off() {
        let mut m = ChargerMonitor::new();
        let o = m.update(false, ChargeLimit::StdMax, true);
        assert!(!o.enable);
        assert_eq!(o.status, ChargerStatus::Discharge);
    }

    #[test]
    fn charger_reports_charge_complete_and_fault() {
        let mut m = ChargerMonitor::new();
        for _ in 0..10 {
            assert_eq!(
                m.update(true, ChargeLimit::StdMax, true).status,
                ChargerStatus::Charge
            );
        }
        for _ in 0..10 {
            m.update(true, ChargeLimit::StdMax, false);
        }
        assert_eq!(
            m.update(true, ChargeLimit::StdMax, false).status,
            ChargerStatus::Complete
        );
        let mut last = ChargerStatus::Discharge;
        for i in 0..14 {
            // 1 Hz blinking = 2 samples on, 2 off at 4 Hz sampling
            last = m.update(true, ChargeLimit::StdMax, (i / 2) % 2 == 0).status;
        }
        assert_eq!(last, ChargerStatus::Fault);
    }

    #[test]
    fn charger_pauses_after_timeout() {
        let mut m = ChargerMonitor::new();
        let steps = ChargerMonitor::CHARGE_TIMEOUT_MS / ChargerMonitor::SAMPLE_MS;
        for _ in 0..steps {
            assert!(m.update(true, ChargeLimit::StdMax, true).enable);
        }
        assert!(!m.update(true, ChargeLimit::StdMax, true).enable);
        for _ in 0..ChargerMonitor::CHARGE_PAUSE_MS / ChargerMonitor::SAMPLE_MS {
            assert!(!m.update(true, ChargeLimit::StdMax, true).enable);
        }
        assert!(m.update(true, ChargeLimit::StdMax, true).enable);
    }
}
