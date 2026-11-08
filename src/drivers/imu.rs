//! LSM6DS3TR-C IMU on I2C2 (SCL PB10 AF4, SDA PB3 AF9, INT1 PB4).
//!
//! - Before the bus is configured, SCL is toggled 10 times as GPIO to free
//!   a chip that is stuck in a transfer (same as Pybricks,
//!   \[PB\] `platform.c` L173-190).
//! - 400 kHz I2C with DMA (TX DMA1 stream 7, RX DMA1 stream 2).
//! - Every gyro data-ready pulse on INT1 (833 Hz) the task reads 12 bytes
//!   and updates the heading (`ekips_hub_logic::imu::HeadingTracker`).
//! - If INT1 stays silent for 50 ms the task polls anyway, so a missing
//!   interrupt cannot freeze the heading.
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_logic::imu::{
    self, CTRL3_SW_RESET, HeadingTracker, I2C_ADDR, INIT_SEQUENCE, Sample, WHO_AM_I_VALUE, reg,
};
use embassy_futures::select::select;
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::i2c::{self, I2c};
use embassy_stm32::mode::Async;
use embassy_stm32::peripherals::{DMA1_CH2, DMA1_CH7, EXTI4, I2C2, PB3, PB4, PB10};
use embassy_stm32::time::Hertz;
use embassy_stm32::{Peri, bind_interrupts, dma, interrupt, peripherals};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Instant, Timer};

bind_interrupts!(struct Irqs {
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
    DMA1_STREAM2 => dma::InterruptHandler<peripherals::DMA1_CH2>;
    EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
});

struct Shared {
    tracker: HeadingTracker,
    last: Sample,
    ok: bool,
}

static SHARED: Mutex<CriticalSectionRawMutex, RefCell<Shared>> = Mutex::new(RefCell::new(Shared {
    tracker: HeadingTracker::new(),
    last: Sample {
        gyro_dps: [0.0; 3],
        accel_ms2: [0.0; 3],
    },
    ok: false,
}));

/// Heading in degrees since the last reset (clockwise positive).
#[allow(dead_code, reason = "used by the Hardware implementation later")]
pub fn heading() -> f32 {
    SHARED.lock(|s| s.borrow().tracker.heading())
}

/// Sets the heading to zero.
#[allow(dead_code, reason = "used by the Hardware implementation later")]
pub fn reset_heading() {
    SHARED.lock(|s| s.borrow_mut().tracker.reset());
}

/// Last converted sample and whether the IMU is working.
#[allow(dead_code, reason = "for tilt/orientation features later")]
pub fn last_sample() -> (Sample, bool) {
    SHARED.lock(|s| {
        let s = s.borrow();
        (s.last, s.ok)
    })
}

/// Pins and peripherals owned by the IMU task.
pub struct ImuResources {
    /// I2C2.
    pub i2c: Peri<'static, I2C2>,
    /// PB10, SCL.
    pub scl: Peri<'static, PB10>,
    /// PB3, SDA.
    pub sda: Peri<'static, PB3>,
    /// PB4, INT1.
    pub int1: Peri<'static, PB4>,
    /// EXTI line 4.
    pub exti: Peri<'static, EXTI4>,
    /// DMA1 stream 7 (I2C2_TX).
    pub tx_dma: Peri<'static, DMA1_CH7>,
    /// DMA1 stream 2 (I2C2_RX).
    pub rx_dma: Peri<'static, DMA1_CH2>,
}

type Bus = I2c<'static, Async, i2c::Master>;

async fn write_reg(bus: &mut Bus, r: u8, v: u8) -> Result<(), i2c::Error> {
    bus.write(I2C_ADDR, &[r, v]).await
}

async fn read_reg(bus: &mut Bus, r: u8) -> Result<u8, i2c::Error> {
    let mut b = [0u8];
    bus.write_read(I2C_ADDR, &[r], &mut b).await?;
    Ok(b[0])
}

async fn init(bus: &mut Bus) -> Result<(), i2c::Error> {
    let id = read_reg(bus, reg::WHO_AM_I).await?;
    if id != WHO_AM_I_VALUE {
        defmt::error!("IMU: unexpected WHO_AM_I {=u8:#x}", id);
        return Err(i2c::Error::Bus);
    }
    write_reg(bus, reg::CTRL3_C, CTRL3_SW_RESET).await?;
    for _ in 0..100 {
        if read_reg(bus, reg::CTRL3_C).await? & CTRL3_SW_RESET == 0 {
            break;
        }
        Timer::after_millis(1).await;
    }
    for (r, v) in INIT_SEQUENCE {
        write_reg(bus, r, v).await?;
    }
    Ok(())
}

/// Runs the IMU forever (re-initializes after errors).
#[embassy_executor::task]
pub async fn task(mut r: ImuResources) {
    // Bus recovery: 10 clock pulses on SCL.
    {
        let mut scl = Output::new(r.scl.reborrow(), Level::High, Speed::VeryHigh);
        for _ in 0..10 {
            scl.set_high();
            cortex_m::asm::delay(96); // ~1 us
            scl.set_low();
            cortex_m::asm::delay(96);
        }
        scl.set_high();
    }

    let mut config = i2c::Config::default();
    config.frequency = Hertz(400_000);
    let mut bus = I2c::new(r.i2c, r.scl, r.sda, r.tx_dma, r.rx_dma, Irqs, config);
    let mut int1 = ExtiInput::new(r.int1, r.exti, Pull::None, Irqs);

    loop {
        if let Err(e) = init(&mut bus).await {
            defmt::warn!("IMU init failed: {}", e);
            SHARED.lock(|s| s.borrow_mut().ok = false);
            Timer::after_millis(500).await;
            continue;
        }
        defmt::info!("IMU initialized");
        let mut last = Instant::now();
        loop {
            select(int1.wait_for_rising_edge(), Timer::after_millis(50)).await;
            let mut raw = [0u8; 12];
            if let Err(e) = bus.write_read(I2C_ADDR, &[reg::OUTX_L_G], &mut raw).await {
                defmt::warn!("IMU read failed: {}", e);
                break;
            }
            let now = Instant::now();
            let dt = (now - last).as_micros() as f32 / 1e6;
            last = now;
            let sample = imu::convert(&raw);
            SHARED.lock(|s| {
                let mut s = s.borrow_mut();
                // Ignore absurd gaps (e.g. after a stall) instead of
                // integrating them.
                s.tracker.update(&sample, dt.min(0.05));
                s.last = sample;
                s.ok = true;
            });
        }
    }
}
