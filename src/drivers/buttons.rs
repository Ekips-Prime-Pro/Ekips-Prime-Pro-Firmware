//! Hub buttons and power-off by long press of the center button.
//!
//! Reads the two resistor ladders from [`super::analog`], debounces them
//! with `ekips_hub_logic::buttons::ButtonTracker` and publishes the state.
//! Holding the center button for 2 s (after it was released once since
//! boot) powers the hub off.
//!
//! **Untested on hardware.**

use core::cell::Cell;

use ekips_hub_logic::buttons::{self, ButtonEvent, ButtonTracker, Buttons};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Ticker};

use super::analog;

const POLL_MS: u32 = 10;

static STATE: Mutex<CriticalSectionRawMutex, Cell<Buttons>> = Mutex::new(Cell::new(Buttons {
    center: false,
    left: false,
    right: false,
    bluetooth: false,
}));

/// Signalled once when the user requests power-off.
pub static POWER_OFF_REQUEST: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Current debounced button state.
#[allow(dead_code, reason = "used by the Hardware implementation later")]
pub fn state() -> Buttons {
    STATE.lock(|c| c.get())
}

/// Polls the buttons forever.
#[embassy_executor::task]
pub async fn task() {
    let mut tracker = ButtonTracker::new();
    let mut ticker = Ticker::every(Duration::from_millis(u64::from(POLL_MS)));
    loop {
        ticker.next().await;
        let s = analog::latest();
        if s.count == 0 {
            continue;
        }
        // Invalid readings (ADC not settled yet) are skipped, like Pybricks
        // does during the first tens of milliseconds after boot.
        let Some(sample) = buttons::from_adc(s.ladder_center, s.ladder_buttons) else {
            continue;
        };
        match tracker.update(sample, POLL_MS) {
            Some(ButtonEvent::Changed(b)) => {
                STATE.lock(|c| c.set(b));
                defmt::debug!("buttons: {}", b);
            }
            Some(ButtonEvent::PowerOffRequested) => {
                defmt::info!("power-off requested by center button");
                POWER_OFF_REQUEST.signal(());
            }
            None => {}
        }
    }
}
