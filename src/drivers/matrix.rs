//! 5x5 light matrix on the TLC5955.
//!
//! Pixel → TLC5955 channel mapping from \[PB\] `platform.c` L414-427
//! (`bsp::pins::TLC_MATRIX`). Which corner is "top left" relative to the
//! housing is not verified yet; [`set_orientation`] can correct it.
//!
//! [`show_text`] displays a single character statically and scrolls longer
//! text (one column every [`SCROLL_STEP_MS`]).
//!
//! **Untested on hardware.**

use core::cell::RefCell;

use ekips_hub_logic::led::brightness_to_duty;
use ekips_hub_logic::matrix::{self, Image, Orientation, TextScroller};
use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use heapless::String;

use super::tlc5955;
use crate::bsp::pins;

/// Time per scroll step.
pub const SCROLL_STEP_MS: u64 = 80;
/// Default text brightness in percent.
pub const TEXT_BRIGHTNESS: u8 = 60;
/// Maximum text length shown (longer text is cut).
pub const MAX_TEXT: usize = 32;

enum Content {
    Image(Image),
    Text(String<MAX_TEXT>),
}

static REQUEST: Signal<CriticalSectionRawMutex, Content> = Signal::new();
static ORIENTATION: Mutex<CriticalSectionRawMutex, RefCell<Orientation>> =
    Mutex::new(RefCell::new(Orientation::Up));

/// Shows a static image (replaces any running text).
pub fn show_image(image: Image) {
    REQUEST.signal(Content::Image(image));
}

/// Shows `text`: one character statically, more characters scrolling.
pub fn show_text(text: &str) {
    let mut s = String::new();
    for c in text.chars() {
        if s.push(c).is_err() {
            break;
        }
    }
    REQUEST.signal(Content::Text(s));
}

/// Clears the matrix.
pub fn clear() {
    show_image(Image::BLANK);
}

/// Sets the rotation applied to everything that is shown.
#[allow(dead_code, reason = "used once the IMU drives the orientation")]
pub fn set_orientation(o: Orientation) {
    ORIENTATION.lock(|c| *c.borrow_mut() = o);
}

fn draw(image: &Image) {
    let o = ORIENTATION.lock(|c| *c.borrow());
    let img = image.rotated(o);
    tlc5955::set_many(
        pins::TLC_MATRIX
            .iter()
            .zip(img.0.iter())
            .map(|(&ch, &b)| (ch, brightness_to_duty(b))),
    );
}

/// Renders the requested content.
#[embassy_executor::task]
pub async fn task() {
    let mut content = Content::Image(Image::BLANK);
    loop {
        match &content {
            Content::Image(img) => {
                draw(img);
                content = REQUEST.wait().await;
            }
            Content::Text(text) if text.chars().count() <= 1 => {
                draw(&matrix::char_image(
                    text.chars().next().unwrap_or(' '),
                    TEXT_BRIGHTNESS,
                ));
                content = REQUEST.wait().await;
            }
            Content::Text(text) => {
                let text = text.clone();
                let mut next = None;
                for frame in TextScroller::new(&text, TEXT_BRIGHTNESS) {
                    draw(&frame);
                    match select(Timer::after_millis(SCROLL_STEP_MS), REQUEST.wait()).await {
                        Either::First(()) => {}
                        Either::Second(c) => {
                            next = Some(c);
                            break;
                        }
                    }
                }
                content = match next {
                    Some(c) => c,
                    None => REQUEST.wait().await,
                };
            }
        }
    }
}
