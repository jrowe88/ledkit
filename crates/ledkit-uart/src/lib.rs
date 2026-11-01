#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

mod clockless;
mod error;
#[cfg(feature = "smart-leds")]
mod smart_leds;
#[cfg(test)]
mod testing;

pub use clockless::{buffer_len, ClocklessUart};
pub use error::Error;
