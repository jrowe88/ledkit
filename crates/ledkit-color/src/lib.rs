//! Colors for addressable LEDs (neopixels, dotstars and friends).
//!
//! `ledkit-color` provides RGB and RGBW colors, hue-based colors, gamma
//! correction, color temperature and gradients, all without `std` or an
//! allocator. It is part of the ledkit family of crates.
#![cfg_attr(not(test), no_std)]
