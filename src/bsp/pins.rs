//! Complete pin map of the LEGO SPIKE Prime hub (STM32F413VGT6).
//!
//! **Untested on hardware.** All values are taken from the sources below and
//! have not been verified on a real hub by this project.
//!
//! # Sources
//!
//! Every constant cites at least one source with file and line number:
//!
//! - **\[PB\]** Pybricks, `lib/pbio/platform/prime_hub_f4/platform.c` at commit
//!   [`c860b6a`](https://github.com/pybricks/pybricks-micropython/blob/c860b6a00301fb7c6ecd429da11dcb5a6c557c7a/lib/pbio/platform/prime_hub_f4/platform.c)
//!   (MIT, © The Pybricks Authors). Older Pybricks versions call the
//!   directory `prime_hub`.
//! - **\[PB-cfg\]** Pybricks, `lib/pbio/platform/prime_hub_f4/pbdrvconfig.h`,
//!   same commit.
//! - **\[SR\]** spike-rtic, `src/pins.rs` at commit
//!   [`e6060b0`](https://github.com/springyworks/spike-rtic/blob/e6060b0beef16d3635b9d63f2ac829f5519be7d8/src/pins.rs)
//!   (MIT, © spike-rtic contributors).
//!
//! The same table is rendered in `docs/en/pinout.md` / `docs/de/pinout.md`.
//! Keep them in sync.

// The pin map is complete on purpose (it is documentation as well), so not
// every constant is used by a driver yet.
#![allow(dead_code)]

/// GPIO port of the STM32F413.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum GpioPort {
    /// GPIOA
    A,
    /// GPIOB
    B,
    /// GPIOC
    C,
    /// GPIOD
    D,
    /// GPIOE
    E,
}

impl GpioPort {
    /// Index of the port (A = 0) as used by the RCC enable bits and the
    /// Embassy PAC (`pac::GPIOA` … `pac::GPIOE`).
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A single GPIO pin, e.g. `PA13`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PinId {
    /// GPIO port.
    pub port: GpioPort,
    /// Pin number 0..=15.
    pub pin: u8,
}

const fn p(port: GpioPort, pin: u8) -> PinId {
    PinId { port, pin }
}

use GpioPort::{A, B, C, D, E};

// ---------------------------------------------------------------------------
// Power
// ---------------------------------------------------------------------------

/// Main power latch. HIGH keeps the hub on, LOW cuts the main 3.3 V rail
/// (unless USB is plugged in). Must be driven HIGH as early as possible
/// after reset. **Also SWDIO** (see `docs/en/decisions.md`, D-003).
///
/// \[PB\] L651-656 (`pbdrv_reset_power_off`), L1086-1092 (`SystemInit`);
/// \[SR\] L29-31.
pub const POWER_LATCH: PinId = p(A, 13);

/// Enables 3.3 V (VCC, pin 4) on all six I/O ports. Active HIGH.
/// **Also SWCLK** (see D-003).
///
/// \[PB\] L228-231 (`pbdrv_ioport_platform_data_vcc_pin`); \[SR\] L33-34.
pub const PORT_VCC_ENABLE: PinId = p(A, 14);

// ---------------------------------------------------------------------------
// USB OTG FS
// ---------------------------------------------------------------------------

/// USB D- (`OTG_FS_DM`, AF10). \[PB\] L968-974; \[SR\] L37-38.
pub const USB_DM: PinId = p(A, 11);
/// USB D+ (`OTG_FS_DP`, AF10). \[PB\] L968-974; \[SR\] L39-40.
pub const USB_DP: PinId = p(A, 12);
/// USB VBUS sense. HIGH while a cable is plugged in. \[PB\] L976-980,
/// L1009-1015; \[SR\] L41-42.
pub const USB_VBUS: PinId = p(A, 9);
/// Alternate function number of the USB data pins. \[PB\] L973.
pub const USB_AF: u8 = 10;

// ---------------------------------------------------------------------------
// TLC5955 48-channel LED driver (5x5 matrix, status/battery/BT LEDs,
// charger MODE pin)
// ---------------------------------------------------------------------------

/// `SPI1_SCK` (AF5). \[PB\] L874-885; \[SR\] L45-46.
pub const TLC5955_SCK: PinId = p(A, 5);
/// `SPI1_MISO` (AF5), data read back from the TLC5955 shift register.
/// \[PB\] L874-885; \[SR\] L47-48.
pub const TLC5955_MISO: PinId = p(A, 6);
/// `SPI1_MOSI` (AF5). \[PB\] L874-885; \[SR\] L49-50.
pub const TLC5955_MOSI: PinId = p(A, 7);
/// Latch (LAT) GPIO; a HIGH pulse moves the shift register into the
/// grayscale or control latch. \[PB\] L645-646; \[SR\] L51-52.
pub const TLC5955_LAT: PinId = p(A, 15);
/// Grayscale clock GSCLK, `TIM12_CH2` (AF9), 9.6 MHz, 50 % duty.
/// \[PB\] L524-536, L605-613; \[SR\] L53-54.
pub const TLC5955_GSCLK: PinId = p(B, 15);
/// Alternate function of the SPI1 pins. \[PB\] L883.
pub const TLC5955_SPI_AF: u8 = 5;
/// Alternate function of GSCLK (TIM12). \[PB\] L531.
pub const TLC5955_GSCLK_AF: u8 = 9;

// ---------------------------------------------------------------------------
// External SPI flash (W25Q256, SPI2) – not used by this firmware yet
// ---------------------------------------------------------------------------

/// External flash chip select, active LOW. \[PB\] L890-896, L957-960;
/// \[SR\] L57-58.
pub const FLASH_NCS: PinId = p(B, 12);
/// `SPI2_SCK` (AF5). \[PB\] L898-904; \[SR\] L59-60.
pub const FLASH_SCK: PinId = p(B, 13);
/// `SPI2_MISO` (AF5). \[PB\] L906-908; \[SR\] L61-62.
pub const FLASH_MISO: PinId = p(C, 2);
/// `SPI2_MOSI` (AF5). \[PB\] L906-908; \[SR\] L63-64.
pub const FLASH_MOSI: PinId = p(C, 3);

// ---------------------------------------------------------------------------
// IMU LSM6DS3TR-C (I2C2, 7-bit address 0x6A)
// ---------------------------------------------------------------------------

/// `I2C2_SCL` (AF4). \[PB\] L173-190; \[SR\] L69-70.
pub const IMU_SCL: PinId = p(B, 10);
/// `I2C2_SDA` (AF9!). \[PB\] L192-195; \[SR\] L67-68.
pub const IMU_SDA: PinId = p(B, 3);
/// IMU INT1 (data ready), rising edge. \[PB\] L202-210; \[SR\] L71-72.
pub const IMU_INT1: PinId = p(B, 4);
/// Alternate function of SCL. \[PB\] L189.
pub const IMU_SCL_AF: u8 = 4;
/// Alternate function of SDA. \[PB\] L194.
pub const IMU_SDA_AF: u8 = 9;

// ---------------------------------------------------------------------------
// Sound
// ---------------------------------------------------------------------------

/// Speaker signal, `DAC1_OUT1` (analog). \[PB\] L675-698; \[SR\] L75-76.
pub const SPEAKER_DAC: PinId = p(A, 4);
/// Audio amplifier enable, active HIGH. \[PB\] L676-677; \[SR\] L77-78.
pub const SPEAKER_AMP_ENABLE: PinId = p(C, 10);

// ---------------------------------------------------------------------------
// Bluetooth (TI CC2564C) – kept OFF by this firmware
// ---------------------------------------------------------------------------

/// Bluetooth chip enable (nSHUTD), active HIGH. Kept LOW. \[PB\] L91-95;
/// \[SR\] L81-82.
pub const BT_ENABLE: PinId = p(A, 2);
/// `USART2_TX` (AF7). \[PB\] L106-123; \[SR\] L83-84.
pub const BT_TX: PinId = p(D, 5);
/// `USART2_RX` (AF7). \[PB\] L106-123; \[SR\] L85-86.
pub const BT_RX: PinId = p(D, 6);
/// `USART2_CTS` (AF7). \[PB\] L114-117; \[SR\] L87-88.
pub const BT_CTS: PinId = p(D, 3);
/// `USART2_RTS` (AF7). \[PB\] L114-117; \[SR\] L89-90.
pub const BT_RTS: PinId = p(D, 4);
/// 32.768 kHz slow clock for the Bluetooth chip, `TIM8_CH4` (AF3).
/// \[PB\] L538-550, L614-622.
pub const BT_SLOW_CLOCK: PinId = p(C, 9);

// ---------------------------------------------------------------------------
// Analog inputs (ADC1)
// ---------------------------------------------------------------------------

/// An ADC1 input: pin and channel number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdcInput {
    /// The analog pin.
    pub pin: PinId,
    /// ADC1 channel number.
    pub channel: u8,
}

/// Battery current (`ADC_IBAT`). \[PB\] L783-794; \[PB-cfg\] L20;
/// \[SR\] L93-94.
pub const ADC_BATTERY_CURRENT: AdcInput = AdcInput {
    pin: p(C, 0),
    channel: 10,
};
/// Battery voltage (`ADCVBAT`). \[PB\] L796-807; \[PB-cfg\] L16;
/// \[SR\] L95-96.
pub const ADC_BATTERY_VOLTAGE: AdcInput = AdcInput {
    pin: p(C, 1),
    channel: 11,
};
/// Battery NTC thermistor (`BAT_NTC`). \[PB\] L809-820; \[PB-cfg\] L26;
/// \[SR\] L97-98.
pub const ADC_BATTERY_NTC: AdcInput = AdcInput {
    pin: p(B, 0),
    channel: 8,
};
/// USB charger input current (`IBUSBCH`). \[PB\] L822-833, L154;
/// \[SR\] L99-100.
pub const ADC_USB_CHARGE_CURRENT: AdcInput = AdcInput {
    pin: p(A, 3),
    channel: 3,
};
/// Resistor ladder 0: center button (CH_1) and charger `/CHG` (CH_2).
/// \[PB\] L835-846, L662-666; \[SR\] L103-104, L209-218.
pub const ADC_LADDER_CENTER: AdcInput = AdcInput {
    pin: p(C, 4),
    channel: 14,
};
/// Resistor ladder 1: left (CH_0), right (CH_1) and Bluetooth (CH_2)
/// buttons. \[PB\] L848-859, L667-670; \[SR\] L105-106.
pub const ADC_LADDER_BUTTONS: AdcInput = AdcInput {
    pin: p(A, 1),
    channel: 1,
};

// ---------------------------------------------------------------------------
// Battery charger MP2639A
// ---------------------------------------------------------------------------

/// Charger current limit ISET, `TIM5_CH1` (AF2), 96 kHz PWM.
/// \[PB\] L552-561, L623-631, L149-157; spike-rtic `src/power.rs` L54-57.
pub const CHARGER_ISET: PinId = p(A, 0);
/// Alternate function of ISET. \[PB\] L559.
pub const CHARGER_ISET_AF: u8 = 2;

// ---------------------------------------------------------------------------
// TLC5955 channel assignment
// ---------------------------------------------------------------------------

/// TLC5955 channel driving the MP2639A MODE pin. 0 = charging allowed,
/// 0xFFFF = charging disabled. \[PB\] L149-151; \[SR\] L201-217.
pub const TLC_CH_CHARGER_MODE: u8 = 14;

/// RGB LED: (red, green, blue) TLC5955 channels.
pub type RgbChannels = (u8, u8, u8);

/// Center button LED, top half. \[PB\] L367-378; \[SR\] L192.
pub const TLC_STATUS_TOP: RgbChannels = (5, 4, 3);
/// Center button LED, bottom half. \[PB\] L379-389; \[SR\] L193.
pub const TLC_STATUS_BOTTOM: RgbChannels = (8, 7, 6);
/// Battery LED. \[PB\] L390-400; \[SR\] L194.
pub const TLC_BATTERY: RgbChannels = (2, 1, 0);
/// Bluetooth LED. \[PB\] L401-411; \[SR\] L195.
pub const TLC_BLUETOOTH: RgbChannels = (20, 19, 18);

/// 5x5 matrix: TLC5955 channel of each pixel, row-major, index 0 = top
/// left when the hub is upright (USB port at the top… orientation to be
/// verified on hardware). \[PB\] L414-427; \[SR\] L183-189.
pub const TLC_MATRIX: [u8; 25] = [
    38, 36, 41, 46, 33, //
    37, 28, 39, 47, 21, //
    24, 29, 31, 45, 23, //
    26, 27, 32, 34, 22, //
    25, 40, 30, 35, 9, //
];

// ---------------------------------------------------------------------------
// Motor H-bridges (one per I/O port)
// ---------------------------------------------------------------------------

/// Timer used for an H-bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorTimer {
    /// TIM1 (ports A, B)
    Tim1,
    /// TIM3 (ports E, F)
    Tim3,
    /// TIM4 (ports C, D)
    Tim4,
}

/// The two H-bridge inputs of one port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotorPins {
    /// H-bridge input 1 (PWM when driving forward).
    pub pin1: PinId,
    /// H-bridge input 2 (PWM when driving backward).
    pub pin2: PinId,
    /// Timer of both pins.
    pub timer: MotorTimer,
    /// Timer channel (1..=4) of `pin1`.
    pub ch1: u8,
    /// Timer channel (1..=4) of `pin2`.
    pub ch2: u8,
    /// Alternate function number of both pins.
    pub af: u8,
}

/// H-bridge pins, index 0 = port A … 5 = port F.
/// \[PB\] L429-511 (`pbdrv_motor_driver_hbridge_pwm_platform_data`),
/// L567-604 (timers); \[SR\] L108-164.
pub const MOTOR: [MotorPins; 6] = [
    // Port A: PE9 TIM1_CH1, PE11 TIM1_CH2 (AF1). [PB] L433-445
    MotorPins {
        pin1: p(E, 9),
        pin2: p(E, 11),
        timer: MotorTimer::Tim1,
        ch1: 1,
        ch2: 2,
        af: 1,
    },
    // Port B: PE13 TIM1_CH3, PE14 TIM1_CH4 (AF1). [PB] L446-458
    MotorPins {
        pin1: p(E, 13),
        pin2: p(E, 14),
        timer: MotorTimer::Tim1,
        ch1: 3,
        ch2: 4,
        af: 1,
    },
    // Port C: PB6 TIM4_CH1, PB7 TIM4_CH2 (AF2). [PB] L459-471
    MotorPins {
        pin1: p(B, 6),
        pin2: p(B, 7),
        timer: MotorTimer::Tim4,
        ch1: 1,
        ch2: 2,
        af: 2,
    },
    // Port D: PB8 TIM4_CH3, PB9 TIM4_CH4 (AF2). [PB] L472-484
    MotorPins {
        pin1: p(B, 8),
        pin2: p(B, 9),
        timer: MotorTimer::Tim4,
        ch1: 3,
        ch2: 4,
        af: 2,
    },
    // Port E: PC6 TIM3_CH1, PC7 TIM3_CH2 (AF2). [PB] L485-497
    MotorPins {
        pin1: p(C, 6),
        pin2: p(C, 7),
        timer: MotorTimer::Tim3,
        ch1: 1,
        ch2: 2,
        af: 2,
    },
    // Port F: PC8 TIM3_CH3, PB1 TIM3_CH4 (AF2). [PB] L498-510
    MotorPins {
        pin1: p(C, 8),
        pin2: p(B, 1),
        timer: MotorTimer::Tim3,
        ch1: 3,
        ch2: 4,
        af: 2,
    },
];

// ---------------------------------------------------------------------------
// I/O ports (LPF2 connectors): UART + ID pins
// ---------------------------------------------------------------------------

/// UART peripheral used by an I/O port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortUart {
    /// UART4 (port B)
    Uart4,
    /// UART5 (port D)
    Uart5,
    /// UART7 (port A)
    Uart7,
    /// UART8 (port C)
    Uart8,
    /// UART9 (port F)
    Uart9,
    /// UART10 (port E)
    Uart10,
}

/// Pins of one LPF2 I/O port (besides the motor pins).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoPortPins {
    /// Connector pin 5 (ID1 / GPIO).
    pub p5: PinId,
    /// Connector pin 6 (ID2 / GPIO).
    pub p6: PinId,
    /// UART line buffer enable, active LOW.
    pub uart_buf: PinId,
    /// UART TX (hub → device).
    pub uart_tx: PinId,
    /// UART RX (device → hub).
    pub uart_rx: PinId,
    /// Alternate function of TX and RX.
    pub uart_af: u8,
    /// UART peripheral.
    pub uart: PortUart,
}

/// I/O port pins, index 0 = port A … 5 = port F.
/// \[PB\] L233-346 (`pbdrv_ioport_platform_data`), L706-732 (UARTs);
/// \[SR\] L166-178.
pub const IO_PORT: [IoPortPins; 6] = [
    // Port A [PB] L234-255
    IoPortPins {
        p5: p(D, 7),
        p6: p(D, 8),
        uart_buf: p(A, 10),
        uart_tx: p(E, 8),
        uart_rx: p(E, 7),
        uart_af: 8,
        uart: PortUart::Uart7,
    },
    // Port B [PB] L256-273
    IoPortPins {
        p5: p(D, 9),
        p6: p(D, 10),
        uart_buf: p(A, 8),
        uart_tx: p(D, 1),
        uart_rx: p(D, 0),
        uart_af: 11,
        uart: PortUart::Uart4,
    },
    // Port C [PB] L274-291
    IoPortPins {
        p5: p(D, 11),
        p6: p(E, 4),
        uart_buf: p(E, 5),
        uart_tx: p(E, 1),
        uart_rx: p(E, 0),
        uart_af: 8,
        uart: PortUart::Uart8,
    },
    // Port D [PB] L292-309
    IoPortPins {
        p5: p(C, 15),
        p6: p(C, 14),
        uart_buf: p(B, 2),
        uart_tx: p(C, 12),
        uart_rx: p(D, 2),
        uart_af: 8,
        uart: PortUart::Uart5,
    },
    // Port E [PB] L310-327
    IoPortPins {
        p5: p(C, 13),
        p6: p(E, 12),
        uart_buf: p(B, 5),
        uart_tx: p(E, 3),
        uart_rx: p(E, 2),
        uart_af: 11,
        uart: PortUart::Uart10,
    },
    // Port F [PB] L328-345
    IoPortPins {
        p5: p(C, 11),
        p6: p(E, 6),
        uart_buf: p(C, 5),
        uart_tx: p(D, 15),
        uart_rx: p(D, 14),
        uart_af: 11,
        uart: PortUart::Uart9,
    },
];

// ---------------------------------------------------------------------------
// Compile-time consistency check: no GPIO is assigned twice.
// ---------------------------------------------------------------------------

/// 32 single pins + 6 ports * (2 motor + 5 I/O) pins.
const ALL_PINS_LEN: usize = 32 + 6 * 7;

const fn all_pins() -> [PinId; ALL_PINS_LEN] {
    let mut out = [p(A, 0); ALL_PINS_LEN];
    let fixed = [
        POWER_LATCH,
        PORT_VCC_ENABLE,
        USB_DM,
        USB_DP,
        USB_VBUS,
        TLC5955_SCK,
        TLC5955_MISO,
        TLC5955_MOSI,
        TLC5955_LAT,
        TLC5955_GSCLK,
        FLASH_NCS,
        FLASH_SCK,
        FLASH_MISO,
        FLASH_MOSI,
        IMU_SCL,
        IMU_SDA,
        IMU_INT1,
        SPEAKER_DAC,
        SPEAKER_AMP_ENABLE,
        BT_ENABLE,
        BT_TX,
        BT_RX,
        BT_CTS,
        BT_RTS,
        BT_SLOW_CLOCK,
        ADC_BATTERY_CURRENT.pin,
        ADC_BATTERY_VOLTAGE.pin,
        ADC_BATTERY_NTC.pin,
        ADC_USB_CHARGE_CURRENT.pin,
        ADC_LADDER_CENTER.pin,
        ADC_LADDER_BUTTONS.pin,
        CHARGER_ISET,
    ];
    let mut n = 0;
    while n < fixed.len() {
        out[n] = fixed[n];
        n += 1;
    }
    let mut i = 0;
    while i < 6 {
        out[n] = MOTOR[i].pin1;
        out[n + 1] = MOTOR[i].pin2;
        out[n + 2] = IO_PORT[i].p5;
        out[n + 3] = IO_PORT[i].p6;
        out[n + 4] = IO_PORT[i].uart_buf;
        out[n + 5] = IO_PORT[i].uart_tx;
        out[n + 6] = IO_PORT[i].uart_rx;
        n += 7;
        i += 1;
    }
    assert!(n == ALL_PINS_LEN);
    out
}

const fn pins_are_unique(pins: &[PinId]) -> bool {
    let mut i = 0;
    while i < pins.len() {
        let mut j = i + 1;
        while j < pins.len() {
            if pins[i].port.index() == pins[j].port.index() && pins[i].pin == pins[j].pin {
                return false;
            }
            j += 1;
        }
        i += 1;
    }
    true
}

const _: () = assert!(pins_are_unique(&all_pins()), "a GPIO is assigned twice");

/// TLC5955 channels must be unique as well.
const fn tlc_channels_are_unique() -> bool {
    let mut used = [false; 48];
    let mut i = 0;
    while i < TLC_MATRIX.len() {
        if used[TLC_MATRIX[i] as usize] {
            return false;
        }
        used[TLC_MATRIX[i] as usize] = true;
        i += 1;
    }
    let rgb = [
        TLC_STATUS_TOP,
        TLC_STATUS_BOTTOM,
        TLC_BATTERY,
        TLC_BLUETOOTH,
    ];
    let mut k = 0;
    while k < rgb.len() {
        let chs = [rgb[k].0, rgb[k].1, rgb[k].2];
        let mut c = 0;
        while c < 3 {
            if used[chs[c] as usize] {
                return false;
            }
            used[chs[c] as usize] = true;
            c += 1;
        }
        k += 1;
    }
    !used[TLC_CH_CHARGER_MODE as usize]
}

const _: () = assert!(
    tlc_channels_are_unique(),
    "a TLC5955 channel is assigned twice"
);
