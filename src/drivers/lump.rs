//! LUMP (LEGO UART Message Protocol) on the six I/O ports.
//!
//! One task per port:
//!
//! 1. Port VCC on, UART line buffer enabled (active LOW), 2400 baud.
//! 2. Handshake: wait for `TYPE`, collect `MODES`/`SPEED`/`INFO`, answer
//!    the device's `ACK` with `ACK`, switch to the negotiated speed
//!    (normally 115200), select the default mode.
//! 3. Data mode: store every data message, send `NACK` every 100 ms as
//!    keep-alive, restart the handshake after 300 ms without data.
//!
//! Protocol logic: `ekips_hub_logic::lump`. Port pins and UARTs:
//! \[PB\] `platform.c` L233-346, L706-732.
//!
//! Not implemented (yet): passive device detection on P5/P6 (DCM, needed
//! only for simple non-UART devices) and the Powered Up fast sync at
//! 115200 baud that Pybricks tries first.
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_api::Port;
use ekips_hub_logic::lump::{
    self, Action, DATA_TIMEOUT_MS, DeviceClass, KEEP_ALIVE_MS, Link, MAX_PAYLOAD, Parser, Payload,
    SPEED_MIN, SYS_ACK, SYS_NACK,
};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use embassy_stm32::peripherals::{
    PC12, PD0, PD1, PD2, PD14, PD15, PE0, PE1, PE2, PE3, PE7, PE8, UART4, UART5, UART7, UART8,
    UART9, UART10,
};
use embassy_stm32::usart::{self, BufferedUart};
use embassy_stm32::{Peri, bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer, with_timeout};
use embedded_io_async::{Read, Write};
use static_cell::StaticCell;

use super::power;

bind_interrupts!(struct Irqs {
    UART4 => usart::BufferedInterruptHandler<peripherals::UART4>;
    UART5 => usart::BufferedInterruptHandler<peripherals::UART5>;
    UART7 => usart::BufferedInterruptHandler<peripherals::UART7>;
    UART8 => usart::BufferedInterruptHandler<peripherals::UART8>;
    UART9 => usart::BufferedInterruptHandler<peripherals::UART9>;
    UART10 => usart::BufferedInterruptHandler<peripherals::UART10>;
});

/// State of one port as seen by the rest of the firmware.
#[derive(Debug, Clone, Copy)]
pub struct PortState {
    /// Device class, `None` while nothing is synced.
    pub class: Option<DeviceClass>,
    /// LUMP type ID of the device (0 = none).
    pub type_id: u8,
    /// Mode of the last data message.
    pub mode: u8,
    /// Payload of the last data message.
    pub data: Payload,
    /// When the last data message arrived.
    pub updated: Instant,
}

impl PortState {
    const EMPTY: Self = Self {
        class: None,
        type_id: 0,
        mode: 0,
        data: Payload::EMPTY,
        updated: Instant::from_ticks(0),
    };
}

static STATE: Mutex<CriticalSectionRawMutex, RefCell<[PortState; 6]>> =
    Mutex::new(RefCell::new([PortState::EMPTY; 6]));

/// Requests from other drivers to a port task.
#[derive(Debug, Clone, Copy)]
enum Request {
    Select(u8),
    Write { mode: u8, data: Payload },
}

static REQUESTS: [Signal<CriticalSectionRawMutex, Request>; 6] = [const { Signal::new() }; 6];

/// Current state of `port`.
#[allow(dead_code, reason = "used by the Hardware implementation later")]
pub fn state(port: Port) -> PortState {
    STATE.lock(|s| s.borrow()[port.index()])
}

/// Asks the device on `port` to switch to `mode`.
#[allow(dead_code, reason = "used once sensor modes can be chosen by programs")]
pub fn select_mode(port: Port, mode: u8) {
    REQUESTS[port.index()].signal(Request::Select(mode));
}

/// Writes data to `mode` of the device on `port` (e.g. sensor lights).
#[allow(dead_code, reason = "used once sensor lights are exposed to programs")]
pub fn write(port: Port, mode: u8, data: &[u8]) {
    REQUESTS[port.index()].signal(Request::Write {
        mode,
        data: Payload::new(data),
    });
}

fn set_state(port: Port, f: impl FnOnce(&mut PortState)) {
    STATE.lock(|s| f(&mut s.borrow_mut()[port.index()]));
}

/// UARTs and pins of all six ports.
pub struct LumpResources {
    /// Port A: UART7, RX PE7, TX PE8, buffer PA10.
    pub a: (
        Peri<'static, UART7>,
        Peri<'static, PE7>,
        Peri<'static, PE8>,
        Peri<'static, AnyPin>,
    ),
    /// Port B: UART4, RX PD0, TX PD1, buffer PA8.
    pub b: (
        Peri<'static, UART4>,
        Peri<'static, PD0>,
        Peri<'static, PD1>,
        Peri<'static, AnyPin>,
    ),
    /// Port C: UART8, RX PE0, TX PE1, buffer PE5.
    pub c: (
        Peri<'static, UART8>,
        Peri<'static, PE0>,
        Peri<'static, PE1>,
        Peri<'static, AnyPin>,
    ),
    /// Port D: UART5, RX PD2, TX PC12, buffer PB2.
    pub d: (
        Peri<'static, UART5>,
        Peri<'static, PD2>,
        Peri<'static, PC12>,
        Peri<'static, AnyPin>,
    ),
    /// Port E: UART10, RX PE2, TX PE3, buffer PB5.
    pub e: (
        Peri<'static, UART10>,
        Peri<'static, PE2>,
        Peri<'static, PE3>,
        Peri<'static, AnyPin>,
    ),
    /// Port F: UART9, RX PD14, TX PD15, buffer PC5.
    pub f: (
        Peri<'static, UART9>,
        Peri<'static, PD14>,
        Peri<'static, PD15>,
        Peri<'static, AnyPin>,
    ),
}

/// Switches on port VCC and starts one task per port.
pub fn start(spawner: Spawner, r: LumpResources) {
    power::set_port_vcc(true);

    let mut config = usart::Config::default();
    config.baudrate = SPEED_MIN;

    macro_rules! port {
        ($port:expr, $res:expr) => {{
            static TX: StaticCell<[u8; 64]> = StaticCell::new();
            static RX: StaticCell<[u8; 128]> = StaticCell::new();
            let (uart, rx, tx, buf) = $res;
            match BufferedUart::new(
                uart,
                rx,
                tx,
                TX.init([0; 64]),
                RX.init([0; 128]),
                Irqs,
                config,
            ) {
                Ok(u) => {
                    // Buffer disabled (HIGH) until the task starts.
                    let buf = Output::new(buf, Level::High, Speed::Low);
                    spawner.spawn(port_task($port, u, buf).unwrap());
                }
                Err(e) => defmt::error!("port {}: UART config error {}", $port.as_char() as u8, e),
            }
        }};
    }
    port!(Port::A, r.a);
    port!(Port::B, r.b);
    port!(Port::C, r.c);
    port!(Port::D, r.d);
    port!(Port::E, r.e);
    port!(Port::F, r.f);
}

async fn send(uart: &mut BufferedUart<'static>, bytes: &[u8]) {
    if uart.write_all(bytes).await.is_ok() {
        let _ = uart.flush().await;
    }
}

#[embassy_executor::task(pool_size = 6)]
async fn port_task(port: Port, mut uart: BufferedUart<'static>, mut buf_en: Output<'static>) {
    buf_en.set_low(); // enable the line buffer (active LOW)
    let mut parser = Parser::new();
    let mut rx = [0u8; 32];

    loop {
        // ---- Handshake at 2400 baud ----
        set_state(port, |s| *s = PortState::EMPTY);
        let _ = uart.set_baudrate(SPEED_MIN);
        parser.reset();
        let mut link = Link::new();
        let info = loop {
            let Ok(Ok(n)) = with_timeout(Duration::from_millis(500), uart.read(&mut rx)).await
            else {
                continue;
            };
            let mut synced = None;
            for &b in &rx[..n] {
                if let Some(msg) = parser.push(b)
                    && let Action::Synced(info) = link.handle(msg)
                {
                    synced = Some(info);
                }
            }
            if let Some(info) = synced {
                break info;
            }
        };

        send(&mut uart, &[SYS_ACK]).await;
        // Give the device time to switch its baud rate.
        Timer::after_millis(10).await;
        let _ = uart.set_baudrate(info.speed);
        parser.reset();
        let class = lump::classify(info.type_id);
        defmt::info!(
            "port {}: device type {} ({}) at {} baud",
            port.as_char() as u8,
            info.type_id,
            class,
            info.speed
        );
        let mut sel = [0u8; 6];
        let n = lump::encode_select(lump::default_mode(class), &mut sel);
        send(&mut uart, &sel[..n]).await;
        set_state(port, |s| {
            s.class = Some(class);
            s.type_id = info.type_id;
        });

        // ---- Data mode ----
        let mut last_data = Instant::now();
        let mut next_keep_alive = Instant::now();
        'data: loop {
            let now = Instant::now();
            if now >= next_keep_alive {
                send(&mut uart, &[SYS_NACK]).await;
                next_keep_alive = now + Duration::from_millis(u64::from(KEEP_ALIVE_MS));
            }
            if now - last_data > Duration::from_millis(u64::from(DATA_TIMEOUT_MS)) {
                defmt::info!("port {}: device lost", port.as_char() as u8);
                break 'data;
            }
            let wait = Timer::at(next_keep_alive);
            let event = select(
                select(uart.read(&mut rx), REQUESTS[port.index()].wait()),
                wait,
            )
            .await;
            match event {
                Either::First(Either::First(Ok(n))) => {
                    for &b in &rx[..n] {
                        let Some(msg) = parser.push(b) else { continue };
                        match link.handle(msg) {
                            Action::Data { mode, data } => {
                                last_data = Instant::now();
                                set_state(port, |s| {
                                    s.mode = mode;
                                    s.data = data;
                                    s.updated = last_data;
                                });
                            }
                            Action::Lost => break 'data,
                            Action::Synced(_) | Action::None => {}
                        }
                    }
                }
                Either::First(Either::First(Err(e))) => {
                    defmt::warn!("port {}: UART error {}", port.as_char() as u8, e);
                }
                Either::First(Either::Second(req)) => match req {
                    Request::Select(mode) => {
                        let n = lump::encode_select(mode, &mut sel);
                        send(&mut uart, &sel[..n]).await;
                    }
                    Request::Write { mode, data } => {
                        let mut out = [0u8; MAX_PAYLOAD + 5];
                        let n = lump::encode_write(mode, data.as_slice(), &mut out);
                        send(&mut uart, &out[..n]).await;
                    }
                },
                Either::Second(()) => {}
            }
        }
    }
}
