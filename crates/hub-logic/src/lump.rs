//! LEGO UART Message Protocol (LUMP) used by SPIKE sensors and motors.
//!
//! Protocol constants from Pybricks `lib/lego/lego/lump.h` and the state
//! machine from `lib/pbio/src/port_lump/protocol.c` (commit `c860b6a`,
//! MIT). spike-rtic's `dev_notes/pybricks-lump-protocol-reference.md` was
//! used as a summary.
//!
//! Message layout: `header [info] payload… checksum`, where
//! `header = type (bits 7:6) | size (bits 5:3) | cmd/mode (bits 2:0)`,
//! payload length `1 << size` (1..=32), `checksum = 0xFF ^ all bytes`.
//! System messages (`SYNC`, `NACK`, `ACK`) are a single byte.

/// Device sends this when it (re)starts.
pub const SYS_SYNC: u8 = 0x00;
/// Host → device keep-alive, at least every 100 ms in data mode.
pub const SYS_NACK: u8 = 0x02;
/// End of handshake (both directions).
pub const SYS_ACK: u8 = 0x04;

/// Message type: system.
pub const MSG_SYS: u8 = 0x00;
/// Message type: command.
pub const MSG_CMD: u8 = 0x40;
/// Message type: info.
pub const MSG_INFO: u8 = 0x80;
/// Message type: data.
pub const MSG_DATA: u8 = 0xC0;

/// Command: device type.
pub const CMD_TYPE: u8 = 0x0;
/// Command: number of modes.
pub const CMD_MODES: u8 = 0x1;
/// Command: baud rate.
pub const CMD_SPEED: u8 = 0x2;
/// Command: select mode (host → device).
pub const CMD_SELECT: u8 = 0x3;
/// Command: write (host → device).
pub const CMD_WRITE: u8 = 0x4;
/// Command: extended mode offset (0 or 8).
pub const CMD_EXT_MODE: u8 = 0x6;
/// Command: firmware/hardware version.
pub const CMD_VERSION: u8 = 0x7;

/// Info: mode name.
pub const INFO_NAME: u8 = 0x00;
/// Info: value format.
pub const INFO_FORMAT: u8 = 0x80;
/// Flag in the info byte: the mode number is +8.
pub const INFO_MODE_PLUS_8: u8 = 0x20;

/// Baud rate during the handshake.
pub const SPEED_MIN: u32 = 2400;
/// Standard data rate of Powered Up devices.
pub const SPEED_LPF2: u32 = 115_200;
/// Keep-alive period in data mode (ms).
pub const KEEP_ALIVE_MS: u32 = 100;
/// Data timeout: device is considered gone after this long without data.
pub const DATA_TIMEOUT_MS: u32 = 3 * KEEP_ALIVE_MS;
/// Maximum payload size.
pub const MAX_PAYLOAD: usize = 32;
/// Maximum number of modes.
pub const MAX_MODES: usize = 16;

/// Device type IDs (`lump.h` `LUMP_TYPE_ID_*`).
pub mod type_id {
    /// SPIKE Medium angular motor.
    pub const SPIKE_M_MOTOR: u8 = 48;
    /// SPIKE Large angular motor.
    pub const SPIKE_L_MOTOR: u8 = 49;
    /// SPIKE Color Sensor.
    pub const SPIKE_COLOR_SENSOR: u8 = 61;
    /// SPIKE Ultrasonic (distance) Sensor.
    pub const SPIKE_ULTRASONIC_SENSOR: u8 = 62;
    /// SPIKE Force Sensor.
    pub const SPIKE_FORCE_SENSOR: u8 = 63;
    /// SPIKE Small angular motor.
    pub const SPIKE_S_MOTOR: u8 = 65;
    /// Technic Large angular motor.
    pub const TECHNIC_L_ANGULAR_MOTOR: u8 = 76;
    /// Technic Medium angular motor.
    pub const TECHNIC_M_ANGULAR_MOTOR: u8 = 75;
}

/// XOR checksum over `bytes`, starting at 0xFF.
pub fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0xFF, |acc, b| acc ^ b)
}

/// Payload length encoded in a header.
pub fn payload_len(header: u8) -> usize {
    1 << ((header >> 3) & 0x7)
}

/// Total message length for a header (1 for system messages).
pub fn message_len(header: u8) -> usize {
    match header & 0xC0 {
        MSG_SYS => 1,
        MSG_INFO => 1 + 1 + payload_len(header) + 1,
        _ => 1 + payload_len(header) + 1,
    }
}

/// Size field for a payload of at most `len` bytes (rounded up to a power
/// of two). Returns `(size_bits, padded_len)`.
pub fn size_for(len: usize) -> (u8, usize) {
    let mut bits = 0u8;
    while (1usize << bits) < len.max(1) {
        bits += 1;
    }
    (bits << 3, 1 << bits)
}

/// A decoded message (payload copied, so the parser can keep going).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Message {
    /// Single-byte system message.
    Sys(u8),
    /// Command.
    Cmd {
        /// Command number.
        cmd: u8,
        /// Payload.
        data: Payload,
    },
    /// Mode information (handshake only).
    Info {
        /// Mode number (0..16, already includes the +8 flag).
        mode: u8,
        /// Info type (`INFO_*`).
        kind: u8,
        /// Payload.
        data: Payload,
    },
    /// Data of a mode.
    Data {
        /// Mode number (0..8, add the last `EXT_MODE` for 8..16).
        mode: u8,
        /// Payload.
        data: Payload,
    },
}

/// Fixed-size payload buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Payload {
    bytes: [u8; MAX_PAYLOAD],
    len: u8,
}

impl Payload {
    /// Empty payload.
    pub const EMPTY: Self = Self {
        bytes: [0; MAX_PAYLOAD],
        len: 0,
    };

    /// Creates a payload from bytes (truncated to [`MAX_PAYLOAD`]).
    pub fn new(data: &[u8]) -> Self {
        let mut bytes = [0; MAX_PAYLOAD];
        let n = data.len().min(MAX_PAYLOAD);
        bytes[..n].copy_from_slice(&data[..n]);
        Self {
            bytes,
            len: n as u8,
        }
    }

    /// The payload bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }
}

impl core::fmt::Debug for Payload {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

/// Byte-stream parser with resynchronization: bytes that do not form a
/// message with a valid checksum are dropped one at a time.
#[derive(Debug, Clone)]
pub struct Parser {
    buf: [u8; MAX_PAYLOAD + 3],
    len: usize,
}

impl Parser {
    /// Empty parser.
    pub const fn new() -> Self {
        Self {
            buf: [0; MAX_PAYLOAD + 3],
            len: 0,
        }
    }

    /// Drops all buffered bytes (e.g. after a baud rate change).
    pub fn reset(&mut self) {
        self.len = 0;
    }

    fn drop_first(&mut self) {
        self.buf.copy_within(1..self.len, 0);
        self.len -= 1;
    }

    /// Feeds one byte. Returns a message when one is complete.
    pub fn push(&mut self, byte: u8) -> Option<Message> {
        if self.len == self.buf.len() {
            self.drop_first();
        }
        self.buf[self.len] = byte;
        self.len += 1;

        while self.len > 0 {
            let header = self.buf[0];
            let need = message_len(header);
            if header & 0xC0 == MSG_SYS {
                self.drop_first();
                return match header {
                    SYS_SYNC | SYS_NACK | SYS_ACK => Some(Message::Sys(header)),
                    _ => None,
                };
            }
            if payload_len(header) > MAX_PAYLOAD {
                self.drop_first();
                continue;
            }
            if self.len < need {
                return None;
            }
            if checksum(&self.buf[..need - 1]) != self.buf[need - 1] {
                self.drop_first();
                continue;
            }
            let low = header & 0x07;
            let msg = match header & 0xC0 {
                MSG_CMD => Message::Cmd {
                    cmd: low,
                    data: Payload::new(&self.buf[1..need - 1]),
                },
                MSG_INFO => {
                    let info = self.buf[1];
                    let plus8 = if info & INFO_MODE_PLUS_8 != 0 { 8 } else { 0 };
                    Message::Info {
                        mode: low + plus8,
                        kind: info & !INFO_MODE_PLUS_8,
                        data: Payload::new(&self.buf[2..need - 1]),
                    }
                }
                _ => Message::Data {
                    mode: low,
                    data: Payload::new(&self.buf[1..need - 1]),
                },
            };
            self.buf.copy_within(need..self.len, 0);
            self.len -= need;
            return Some(msg);
        }
        None
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Encodes a host → device command with a 1-byte payload into `out`.
/// Returns the used length (3).
pub fn encode_cmd1(cmd: u8, value: u8, out: &mut [u8; 3]) -> usize {
    out[0] = MSG_CMD | cmd;
    out[1] = value;
    out[2] = checksum(&out[..2]);
    3
}

/// Encodes `SELECT mode` (with a preceding `EXT_MODE` for modes ≥ 8) into
/// `out`. Returns the used length.
pub fn encode_select(mode: u8, out: &mut [u8; 6]) -> usize {
    let mut a = [0u8; 3];
    let mut b = [0u8; 3];
    encode_cmd1(CMD_EXT_MODE, if mode >= 8 { 8 } else { 0 }, &mut a);
    encode_cmd1(CMD_SELECT, mode & 0x07, &mut b);
    out[..3].copy_from_slice(&a);
    out[3..].copy_from_slice(&b);
    6
}

/// Encodes a data write (host → device) for `mode`: `EXT_MODE` followed by
/// the padded data message. Returns the used length of `out`.
pub fn encode_write(mode: u8, data: &[u8], out: &mut [u8; MAX_PAYLOAD + 5]) -> usize {
    let mut ext = [0u8; 3];
    encode_cmd1(CMD_EXT_MODE, if mode >= 8 { 8 } else { 0 }, &mut ext);
    out[..3].copy_from_slice(&ext);
    let data = &data[..data.len().min(MAX_PAYLOAD)];
    let (size, padded) = size_for(data.len());
    out[3] = MSG_DATA | size | (mode & 0x07);
    out[4..4 + padded].fill(0);
    out[4..4 + data.len()].copy_from_slice(data);
    out[4 + padded] = checksum(&out[3..4 + padded]);
    5 + padded
}

/// Data format of one mode, from `INFO_FORMAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ModeFormat {
    /// Number of values.
    pub num_values: u8,
    /// 0 = int8, 1 = int16, 2 = int32, 3 = float.
    pub data_type: u8,
}

/// Information collected during the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceInfo {
    /// Device type ID.
    pub type_id: u8,
    /// Number of modes.
    pub num_modes: u8,
    /// Baud rate for data mode.
    pub speed: u32,
    /// Format of each mode.
    pub formats: [ModeFormat; MAX_MODES],
}

/// What the port driver has to do after feeding a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Action {
    /// Nothing.
    None,
    /// Handshake complete: send `ACK`, switch to `speed`, then select the
    /// default mode.
    Synced(DeviceInfo),
    /// New data for `mode`.
    Data {
        /// Mode (0..16).
        mode: u8,
        /// Payload.
        data: Payload,
    },
    /// The device restarted (SYNC/TYPE seen in data mode); go back to
    /// [`SPEED_MIN`].
    Lost,
}

/// Handshake and data-mode state machine of one port.
#[derive(Debug, Clone)]
pub struct Link {
    synced: bool,
    have_type: bool,
    info: DeviceInfo,
    ext_mode: u8,
}

impl Link {
    /// New link, waiting for the handshake.
    pub const fn new() -> Self {
        Self {
            synced: false,
            have_type: false,
            info: DeviceInfo {
                type_id: 0,
                num_modes: 0,
                speed: SPEED_LPF2,
                formats: [ModeFormat {
                    num_values: 0,
                    data_type: 0,
                }; MAX_MODES],
            },
            ext_mode: 0,
        }
    }

    /// Whether the handshake has completed.
    pub fn synced(&self) -> bool {
        self.synced
    }

    /// Info of the connected device (valid when [`synced`](Self::synced)).
    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    /// Feeds a parsed message.
    pub fn handle(&mut self, msg: Message) -> Action {
        if self.synced {
            return match msg {
                Message::Data { mode, data } => Action::Data {
                    mode: mode + self.ext_mode,
                    data,
                },
                Message::Cmd {
                    cmd: CMD_EXT_MODE,
                    data,
                } => {
                    self.ext_mode = data.as_slice().first().copied().unwrap_or(0).min(8);
                    Action::None
                }
                Message::Cmd { cmd: CMD_TYPE, .. } => {
                    *self = Self::new();
                    Action::Lost
                }
                _ => Action::None,
            };
        }
        match msg {
            Message::Cmd {
                cmd: CMD_TYPE,
                data,
            } => {
                *self = Self::new();
                if let Some(&t) = data.as_slice().first() {
                    self.info.type_id = t;
                    self.have_type = true;
                }
                Action::None
            }
            _ if !self.have_type => Action::None,
            Message::Cmd {
                cmd: CMD_MODES,
                data,
            } => {
                let d = data.as_slice();
                // Powered Up devices send the extended mode count in byte 2.
                let n = if d.len() >= 4 && d[2] != 0 {
                    d[2]
                } else {
                    d[0]
                };
                self.info.num_modes = (n + 1).min(MAX_MODES as u8);
                Action::None
            }
            Message::Cmd {
                cmd: CMD_SPEED,
                data,
            } => {
                let d = data.as_slice();
                if d.len() >= 4 {
                    let s = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
                    if (SPEED_MIN..=460_800).contains(&s) {
                        self.info.speed = s;
                    }
                }
                Action::None
            }
            Message::Info { mode, kind, data } if kind == INFO_FORMAT => {
                let d = data.as_slice();
                if (mode as usize) < MAX_MODES && d.len() >= 2 {
                    self.info.formats[mode as usize] = ModeFormat {
                        num_values: d[0],
                        data_type: d[1],
                    };
                }
                Action::None
            }
            Message::Sys(SYS_ACK) => {
                self.synced = true;
                self.ext_mode = 0;
                Action::Synced(self.info)
            }
            _ => Action::None,
        }
    }
}

impl Default for Link {
    fn default() -> Self {
        Self::new()
    }
}

/// Little-endian i16 value `index` of a payload.
pub fn i16_at(data: &[u8], index: usize) -> Option<i16> {
    let b = data.get(index * 2..index * 2 + 2)?;
    Some(i16::from_le_bytes([b[0], b[1]]))
}

/// Little-endian i32 value `index` of a payload.
pub fn i32_at(data: &[u8], index: usize) -> Option<i32> {
    let b = data.get(index * 4..index * 4 + 4)?;
    Some(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// i8 value `index` of a payload.
pub fn i8_at(data: &[u8], index: usize) -> Option<i8> {
    data.get(index).map(|&b| b as i8)
}

/// Kind of device, derived from the type ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DeviceClass {
    /// Angular motor with absolute encoder.
    AngularMotor,
    /// SPIKE Color Sensor.
    ColorSensor,
    /// SPIKE Ultrasonic Sensor.
    DistanceSensor,
    /// SPIKE Force Sensor.
    ForceSensor,
    /// Something else.
    Unknown,
}

/// Classifies a device type ID.
pub fn classify(type_id: u8) -> DeviceClass {
    use type_id::*;
    match type_id {
        SPIKE_M_MOTOR
        | SPIKE_L_MOTOR
        | SPIKE_S_MOTOR
        | TECHNIC_L_ANGULAR_MOTOR
        | TECHNIC_M_ANGULAR_MOTOR => DeviceClass::AngularMotor,
        SPIKE_COLOR_SENSOR => DeviceClass::ColorSensor,
        SPIKE_ULTRASONIC_SENSOR => DeviceClass::DistanceSensor,
        SPIKE_FORCE_SENSOR => DeviceClass::ForceSensor,
        _ => DeviceClass::Unknown,
    }
}

/// Mode numbers used by this firmware (`device.h` `LEGO_DEVICE_MODE_*`).
pub mod mode {
    /// Angular motor: speed in percent (1x int8).
    pub const MOTOR_SPEED: u8 = 1;
    /// Angular motor: relative position in degrees (1x int32).
    pub const MOTOR_POS: u8 = 2;
    /// Color sensor: discrete color (1x int8).
    pub const COLOR_COLOR: u8 = 0;
    /// Color sensor: reflected light in percent (1x int8).
    pub const COLOR_REFLT: u8 = 1;
    /// Ultrasonic sensor: distance in mm, long range (1x int16).
    pub const DIST_DISTL: u8 = 0;
    /// Force sensor: force (1x int8, LEGO mode 0, unverified).
    pub const FORCE_FORCE: u8 = 0;
    /// Force sensor: touched (1x int8, LEGO mode 1, unverified).
    pub const FORCE_TOUCH: u8 = 1;
}

/// The mode to select after the handshake.
pub fn default_mode(class: DeviceClass) -> u8 {
    match class {
        DeviceClass::AngularMotor => mode::MOTOR_POS,
        DeviceClass::ColorSensor => mode::COLOR_COLOR,
        DeviceClass::DistanceSensor => mode::DIST_DISTL,
        DeviceClass::ForceSensor => mode::FORCE_FORCE,
        DeviceClass::Unknown => 0,
    }
}

/// Colors reported by the color sensor in mode 0 (LEGO color numbers).
/// `None` for "no color".
///
/// Mapping from the LEGO SPIKE color numbers: 0 black, 1 magenta/violet,
/// 3 blue, 4 azure, 5 turquoise, 6 green, 7 yellow, 9 red, 10 white,
/// -1 none. **Unverified on hardware.**
pub fn spike_color(id: i8) -> Option<SpikeColor> {
    Some(match id {
        0 => SpikeColor::Black,
        1 | 2 => SpikeColor::Violet,
        3 => SpikeColor::Blue,
        4 | 5 => SpikeColor::Azure,
        6 => SpikeColor::Green,
        7 | 8 => SpikeColor::Yellow,
        9 => SpikeColor::Red,
        10 => SpikeColor::White,
        _ => return None,
    })
}

/// Color as decoded from the sensor (maps 1:1 to `ekips_hub_api::Color`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SpikeColor {
    /// Black
    Black,
    /// Violet
    Violet,
    /// Blue
    Blue,
    /// Azure
    Azure,
    /// Green
    Green,
    /// Yellow
    Yellow,
    /// Red
    Red,
    /// White
    White,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(header: u8, payload: &[u8]) -> ([u8; 40], usize) {
        let mut f = [0u8; 40];
        f[0] = header;
        f[1..1 + payload.len()].copy_from_slice(payload);
        let n = 1 + payload.len();
        f[n] = checksum(&f[..n]);
        (f, n + 1)
    }

    fn feed(p: &mut Parser, bytes: &[u8]) -> Option<Message> {
        let mut out = None;
        for &b in bytes {
            if let Some(m) = p.push(b) {
                out = Some(m);
            }
        }
        out
    }

    #[test]
    fn known_frames_from_reference() {
        // SELECT mode 0 = [0x43, 0x00, 0xBC] (spike-rtic reference).
        let mut s = [0u8; 6];
        encode_select(0, &mut s);
        assert_eq!(&s[3..], &[0x43, 0x00, 0xBC]);
        assert_eq!(&s[..3], &[0x46, 0x00, 0xB9]);
        // Color sensor light write mode 3, all white. The spike-rtic reference
        // lists checksum 0x68, but 0xFF ^ 0xD3 ^ 0x64 ^ 0x64 ^ 0x64 ^ 0x00 is
        // 0x48 (its "lights off" example 0x2C agrees with this function).
        let mut w = [0u8; MAX_PAYLOAD + 5];
        let n = encode_write(3, &[100, 100, 100], &mut w);
        assert_eq!(
            &w[..n],
            &[0x46, 0x00, 0xB9, 0xD3, 0x64, 0x64, 0x64, 0x00, 0x48]
        );
    }

    #[test]
    fn parses_data_and_resyncs() {
        let mut p = Parser::new();
        let (f, n) = frame(MSG_DATA | 0x08, &[0x96, 0x00]); // DISTL 150 mm
        // garbage first
        let mut stream = [0xFFu8, 0x33, 0x11].to_vec();
        stream.extend_from_slice(&f[..n]);
        match feed(&mut p, &stream) {
            Some(Message::Data { mode: 0, data }) => {
                assert_eq!(i16_at(data.as_slice(), 0), Some(150));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn info_message_with_plus8() {
        let mut p = Parser::new();
        // INFO, size 2, mode 1, info = FORMAT | PLUS_8
        let mut f = [0u8; 5];
        f[0] = MSG_INFO | 0x08 | 1;
        f[1] = INFO_FORMAT | INFO_MODE_PLUS_8;
        f[2] = 7;
        f[3] = 1;
        f[4] = checksum(&f[..4]);
        assert_eq!(
            feed(&mut p, &f),
            Some(Message::Info {
                mode: 9,
                kind: INFO_FORMAT,
                data: Payload::new(&[7, 1]),
            })
        );
    }

    #[test]
    fn handshake_to_data() {
        let mut p = Parser::new();
        let mut link = Link::new();
        let mut run = |bytes: &[u8], link: &mut Link| {
            let mut last = Action::None;
            for &b in bytes {
                if let Some(m) = p.push(b) {
                    let a = link.handle(m);
                    if a != Action::None {
                        last = a;
                    }
                }
            }
            last
        };
        let (t, tn) = frame(MSG_CMD | CMD_TYPE, &[type_id::SPIKE_ULTRASONIC_SENSOR]);
        let (m, mn) = frame(MSG_CMD | 0x08 | CMD_MODES, &[7, 7]);
        let (s, sn) = frame(MSG_CMD | 0x10 | CMD_SPEED, &115_200u32.to_le_bytes());
        assert_eq!(run(&[SYS_SYNC], &mut link), Action::None);
        run(&t[..tn], &mut link);
        run(&m[..mn], &mut link);
        run(&s[..sn], &mut link);
        let a = run(&[SYS_ACK], &mut link);
        let Action::Synced(info) = a else {
            panic!("{a:?}")
        };
        assert_eq!(info.type_id, 62);
        assert_eq!(info.num_modes, 8);
        assert_eq!(info.speed, 115_200);
        assert_eq!(classify(info.type_id), DeviceClass::DistanceSensor);

        let (d, dn) = frame(MSG_DATA | 0x08, &[0xD0, 0x07]);
        match run(&d[..dn], &mut link) {
            Action::Data { mode: 0, data } => assert_eq!(i16_at(data.as_slice(), 0), Some(2000)),
            other => panic!("{other:?}"),
        }
        // Device restarts.
        assert_eq!(run(&t[..tn], &mut link), Action::Lost);
        assert!(!link.synced());
    }

    #[test]
    fn ack_without_type_is_ignored() {
        let mut link = Link::new();
        assert_eq!(link.handle(Message::Sys(SYS_ACK)), Action::None);
    }

    #[test]
    fn helpers() {
        assert_eq!(size_for(3), (0x10, 4));
        assert_eq!(size_for(1), (0x00, 1));
        assert_eq!(size_for(8), (0x18, 8));
        assert_eq!(i32_at(&(-90i32).to_le_bytes(), 0), Some(-90));
        assert_eq!(spike_color(-1), None);
        assert_eq!(spike_color(9), Some(SpikeColor::Red));
        assert_eq!(default_mode(classify(48)), mode::MOTOR_POS);
    }
}
