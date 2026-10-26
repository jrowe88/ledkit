//! Colors for addressable LEDs (NeoPixels, DotStars and friends).
//!
//! `ledkit-color` provides RGB and RGBW colors, hue-based colors, gamma
//! correction, color temperature and gradients, all without `std` or an
//! allocator. It is part of the ledkit family of crates.
//!
//! ## Quick tour
//!
//! ```
//! use ledkit_color::{Color, Hue, Palette, Rgb, GAMMA_2_6};
//!
//! // A named color.
//! let red = Color::RED;
//!
//! // Dim it to about a quarter brightness (0 is off, 255 is unchanged).
//! let dim_red = red.scale(64);
//!
//! // A fully saturated color from a position on the color wheel.
//! let sky = Rgb::from_hue(Hue::BLUE);
//!
//! // Gamma correction makes mid-level colors look right to your eyes.
//! let corrected = GAMMA_2_6.apply_rgb(Rgb::new(128, 64, 32));
//!
//! // A warm white, like a candle or an old light bulb.
//! let candle = Rgb::from_kelvin(2700);
//!
//! // A rainbow stretched along a 30-LED strip.
//! let mut strip = [Rgb::BLACK; 30];
//! for (index, pixel) in strip.iter_mut().enumerate() {
//!     let position = (index * 255 / 29) as u8;
//!     *pixel = Palette::RAINBOW.color_at(position);
//! }
//! assert_eq!(strip[0], Palette::RAINBOW.color_at(0));
//! # let _ = (dim_red, sky, corrected, candle);
//! ```
//!
//! ## Why integer math?
//!
//! Small chips such as the Cortex-M0+ and many RISC-V parts have no floating-point
//! unit, so float math is slow there. Every per-pixel operation in this crate
//! uses integers only. Floating point appears just where a value is computed once,
//! such as building a gamma table.
//!
//! ## Features
//!
//! - `rgb`: conversions to and from the [`rgb`](https://crates.io/crates/rgb)
//!   crate's `RGB8` and `RGBA8` types, so ledkit colors work directly with
//!   `smart-leds-trait` drivers.
//!
#![cfg_attr(not(test), no_std)]

mod gamma;
mod gradient;
mod hue;
#[cfg(feature = "rgb")]
mod interop;
mod named;
mod rgb;
mod temperature;

pub use gamma::{GammaTable, GAMMA_2_2, GAMMA_2_6};
pub use gradient::{Gradient, Palette};
pub use hue::{Hsv, Hue};
pub use rgb::{Rgb, Rgbw};
pub use temperature::correction;

/// A friendlier name for [`Rgb`], so you can write `Color::BLUE`.
pub type Color = Rgb;
