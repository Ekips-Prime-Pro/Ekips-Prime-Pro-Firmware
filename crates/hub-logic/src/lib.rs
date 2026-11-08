//! Hardware-independent logic of the Ekips Prime Pro SPIKE Prime firmware.
//!
//! Everything in this crate is pure computation: no registers, no async,
//! no Embassy. That makes it possible to unit test it on the host with
//! `cargo test -p ekips-hub-logic --target host-tuple`, even though no hub
//! is available. The firmware crate wraps these pieces in drivers.

#![no_std]

pub mod battery;
pub mod buttons;
pub mod imu;
pub mod ladder;
pub mod led;
pub mod lump;
pub mod matrix;
pub mod motor;
pub mod sound;
pub mod tlc5955;
