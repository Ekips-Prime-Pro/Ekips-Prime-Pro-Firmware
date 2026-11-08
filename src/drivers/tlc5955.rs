//! TLC5955 LED driver on SPI1 (5x5 matrix, status, battery and Bluetooth
//! LEDs, charger MODE pin).
//!
//! - SPI1 at 24 MHz (96 MHz / 4 like Pybricks), mode 0, MSB first, TX via
//!   DMA2 stream 3 channel 3 (\[PB\] `platform.c` L634-649).
//! - LAT on PA15: a HIGH pulse after each 97-byte frame latches it.
//! - GSCLK on PB15 = TIM12_CH2, 9.6 MHz, 50 % duty (\[PB\] L524-536,
//!   L605-613).
//! - Start-up: the control frame is sent **twice**, because the maximum
//!   current setting only takes effect the second time (Pybricks comment in
//!   `pwm_tlc5955_stm32.c`).
//!
//! Other drivers change channels with [`set`] / [`set_rgb`]; the task sends
//! a new grayscale frame whenever something changed.
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_logic::led::{Rgb, rgb_to_duty};
use ekips_hub_logic::tlc5955::{ControlData, FRAME_LEN, Grayscale};
use embassy_stm32::gpio::{Level, Output, OutputType, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::{DMA2_CH3, PA5, PA7, PA15, PB15, SPI1, TIM12};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{Ch1, Ch3, Ch4, low_level::CountingMode};
use embassy_stm32::{Peri, bind_interrupts, dma, peripherals};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

use crate::bsp::pins::RgbChannels;

bind_interrupts!(struct Irqs {
    DMA2_STREAM3 => dma::InterruptHandler<peripherals::DMA2_CH3>;
});

/// SPI clock. \[PB\] `pwm_tlc5955_stm32.c`: `SPI_BAUDRATEPRESCALER_4` on
/// the 96 MHz APB2 clock.
const SPI_HZ: u32 = 24_000_000;
/// Grayscale clock. \[PB\] `platform.c` L605-613.
const GSCLK_HZ: u32 = 9_600_000;

static STATE: Mutex<CriticalSectionRawMutex, RefCell<Grayscale>> =
    Mutex::new(RefCell::new(Grayscale::new()));
static CHANGED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Sets channel `ch` to `duty` and schedules a frame update.
#[allow(dead_code, reason = "used by the charger driver later")]
pub fn set(ch: u8, duty: u16) {
    STATE.lock(|s| s.borrow_mut().set(ch, duty));
    CHANGED.signal(());
}

/// Sets several channels at once with a single frame update.
pub fn set_many(values: impl IntoIterator<Item = (u8, u16)>) {
    STATE.lock(|s| {
        let mut s = s.borrow_mut();
        for (ch, duty) in values {
            s.set(ch, duty);
        }
    });
    CHANGED.signal(());
}

/// Sets an RGB LED to a color (Pybricks color correction).
pub fn set_rgb(chs: RgbChannels, color: Rgb) {
    let (r, g, b) = rgb_to_duty(color);
    set_many([(chs.0, r), (chs.1, g), (chs.2, b)]);
}

/// Pins and peripherals owned by the TLC5955 task.
pub struct Tlc5955Resources {
    /// SPI1.
    pub spi: Peri<'static, SPI1>,
    /// PA5, SPI1_SCK.
    pub sck: Peri<'static, PA5>,
    /// PA7, SPI1_MOSI.
    pub mosi: Peri<'static, PA7>,
    /// DMA2 stream 3 (SPI1_TX, channel 3).
    pub tx_dma: Peri<'static, DMA2_CH3>,
    /// PA15, LAT.
    pub lat: Peri<'static, PA15>,
    /// TIM12 for GSCLK.
    pub gsclk_timer: Peri<'static, TIM12>,
    /// PB15, GSCLK.
    pub gsclk: Peri<'static, PB15>,
}

struct Tlc5955 {
    spi: Spi<'static, Async, spi::mode::Master>,
    lat: Output<'static>,
}

impl Tlc5955 {
    async fn send(&mut self, frame: &[u8; FRAME_LEN]) {
        if let Err(e) = self.spi.write(frame).await {
            defmt::warn!("TLC5955 SPI error: {}", e);
            return;
        }
        // The SPI future completes when DMA is done; the TLC5955 needs the
        // last bit shifted out before LAT rises. A few cycles are enough
        // at 24 MHz (one bit = 4 CPU cycles).
        cortex_m::asm::delay(64);
        self.lat.set_high();
        // Minimum LAT pulse width is a few tens of ns; ~1 us is plenty.
        cortex_m::asm::delay(96);
        self.lat.set_low();
    }
}

/// Initializes the TLC5955 and keeps it updated.
#[embassy_executor::task]
pub async fn task(r: Tlc5955Resources) {
    let lat = Output::new(r.lat, Level::Low, Speed::Low);

    let mut config = spi::Config::default();
    config.frequency = Hertz(SPI_HZ);
    config.mode = spi::MODE_0;
    config.bit_order = spi::BitOrder::MsbFirst;
    let spi = Spi::new_txonly(r.spi, r.sck, r.mosi, r.tx_dma, Irqs, config);

    let mut gsclk = SimplePwm::new(
        r.gsclk_timer,
        None::<PwmPin<'static, TIM12, Ch1>>,
        Some(PwmPin::new(r.gsclk, OutputType::PushPull)),
        None::<PwmPin<'static, TIM12, Ch3>>,
        None::<PwmPin<'static, TIM12, Ch4>>,
        Hertz(GSCLK_HZ),
        CountingMode::EdgeAlignedUp,
    );
    let mut ch = gsclk.ch2();
    ch.set_duty_cycle_fraction(1, 2);
    ch.enable();

    let mut dev = Tlc5955 { spi, lat };
    let control = ControlData::PYBRICKS_SPIKE_PRIME.frame();
    dev.send(&control).await;
    dev.send(&control).await;
    defmt::info!("TLC5955 initialized");

    loop {
        let frame = STATE.lock(|s| *s.borrow().frame());
        dev.send(&frame).await;
        CHANGED.wait().await;
    }
}
