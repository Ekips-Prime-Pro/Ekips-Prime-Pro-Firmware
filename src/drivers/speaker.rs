//! Speaker: DAC1 channel 1 on PA4, amplifier enable on PC10.
//!
//! One period of a square wave is played in a loop by circular DMA
//! (DMA1 stream 5, channel 7); TIM6 TRGO paces the DAC. Same structure as
//! Pybricks `sound_stm32_hal_dac.c` (\[PB\] `platform.c` L673-702). The
//! amplifier is only enabled while a beep plays.
//!
//! [`beep`] never blocks: the request is queued and a new beep replaces a
//! running one.
//!
//! **Untested on hardware.**

use ekips_hub_logic::sound::{self, SAMPLES_PER_PERIOD};
use embassy_futures::select::{Either, select};
use embassy_stm32::dac::{DacChannel, Value, ValueArray};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::peripherals::{DAC1, DMA1_CH5, PA4, PC10, TIM6};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::{MasterMode, RoundTo, Timer as LlTimer};
use embassy_stm32::{Peri, bind_interrupts, dma, peripherals, triggers};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

bind_interrupts!(struct Irqs {
    DMA1_STREAM5 => dma::InterruptHandler<peripherals::DMA1_CH5>;
});

/// A beep request.
#[derive(Debug, Clone, Copy, defmt::Format)]
pub struct Beep {
    /// Frequency in Hz (clamped to 20..=10000).
    pub freq_hz: u16,
    /// Duration in ms (0 = stop).
    pub duration_ms: u16,
    /// Volume in percent.
    pub volume: u8,
}

/// Default volume in percent.
pub const DEFAULT_VOLUME: u8 = 50;

static REQUEST: Signal<CriticalSectionRawMutex, Beep> = Signal::new();

/// Plays a beep in the background (replaces a running one).
pub fn beep(freq_hz: u16, duration_ms: u16) {
    REQUEST.signal(Beep {
        freq_hz,
        duration_ms,
        volume: DEFAULT_VOLUME,
    });
}

/// Pins and peripherals owned by the speaker task.
pub struct SpeakerResources {
    /// DAC1.
    pub dac: Peri<'static, DAC1>,
    /// PA4, DAC1_OUT1.
    pub out: Peri<'static, PA4>,
    /// DMA1 stream 5 (DAC1 channel 1).
    pub dma: Peri<'static, DMA1_CH5>,
    /// TIM6, sample clock.
    pub timer: Peri<'static, TIM6>,
    /// PC10, amplifier enable.
    pub amp_enable: Peri<'static, PC10>,
}

/// Plays beep requests forever.
#[embassy_executor::task]
pub async fn task(r: SpeakerResources) {
    let mut amp = Output::new(r.amp_enable, Level::Low, Speed::Low);
    let tim = LlTimer::new(r.timer);
    tim.regs_basic()
        .cr2()
        .modify(|w| w.set_mms(MasterMode::UPDATE));
    let mut dac = DacChannel::new_triggered(r.dac, r.dma, triggers::TIM6_TRGO, Irqs, r.out);
    dac.set(Value::Bit12Left(sound::MID_LEVEL));
    tim.generate_update_event();

    let mut wave = [0u16; SAMPLES_PER_PERIOD];
    let mut next = REQUEST.wait().await;
    loop {
        let req = next;
        if req.duration_ms == 0 || req.volume == 0 {
            next = REQUEST.wait().await;
            continue;
        }
        sound::square_wave(sound::amplitude(req.volume), &mut wave);
        tim.stop();
        tim.set_frequency(Hertz(sound::sample_rate_hz(req.freq_hz)), RoundTo::Slower);
        tim.start();
        amp.set_high();

        let play = dac.write(ValueArray::Bit12Left(&wave), true);
        let stop = select(
            Timer::after_millis(u64::from(req.duration_ms)),
            REQUEST.wait(),
        );
        let replaced = match select(play, stop).await {
            // A circular transfer never finishes on its own.
            Either::First(()) => None,
            Either::Second(Either::First(())) => None,
            Either::Second(Either::Second(b)) => Some(b),
        };

        tim.stop();
        dac.set(Value::Bit12Left(sound::MID_LEVEL));
        dac.enable();
        // With triggering enabled the DAC only takes the new value on the
        // next TRGO, so generate one update event by hand.
        tim.generate_update_event();
        if replaced.is_none() {
            amp.set_low();
        }
        next = match replaced {
            Some(b) => b,
            None => REQUEST.wait().await,
        };
    }
}
