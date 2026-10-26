//! Colors for addressable LEDs (neopixels, dotstars and friends).
//!
//! `ledkit-color` provides RGB and RGBW colors, hue-based colors, gamma
//! correction, color temperature and gradients, all without `std` or an
//! allocator. It is part of the ledkit family of crates.
#![cfg_attr(not(test), no_std)]

mod named;
mod rgb;

pub use rgb::{Rgb, Rgbw};

/// A friendlier name for [`Rgb`], so you can write `Color::BLUE`.
pub type Color = Rgb;
