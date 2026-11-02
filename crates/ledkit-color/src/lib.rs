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
//! let blue = Rgb::from_hue(Hue::BLUE);
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
//! # let _ = (dim_red, blue, corrected, candle);
//! ```
//!
//! ## Why integer math?
//!
//! Small chips such as the Cortex-M0+ and many RISC-V parts have no floating-point
//! unit, so float math is slow there. Per-pixel operations (`scale`, `lerp`,
//! `from_hue`, gamma `apply` and `color_at`) use integers only, and gamma tables
//! are computed at compile time. `Hue::from_degrees`, `Hue::from_fraction` and
//! `Rgb::from_kelvin` use floating point on every call, so on chips without a
//! floating-point unit compute them once and reuse the result. In per-pixel code,
//! use `Hue(u16)` and `Hue::shifted` instead.
//!
//! ## Features
//!
//! - `rgb`: conversions to and from the [`rgb`](https://crates.io/crates/rgb)
//!   crate's `RGB8` type, so ledkit colors work directly with
//!   `smart-leds-trait` drivers.
//! - `smart-leds`: also converts between [`Rgbw`] and `smart-leds-trait`'s RGBW
//!   type (implies `rgb`).
//!
#![cfg_attr(not(test), no_std)]

mod channels;
mod gamma;
mod gradient;
mod hue;
#[cfg(feature = "rgb")]
mod interop;
mod named;
pub mod palettes;
mod temperature;

pub use channels::{Rgb, Rgbw};
pub use gamma::{GammaTable, GAMMA_2_2, GAMMA_2_6};
pub use gradient::{ColorMap, Gradient, Palette};
pub use hue::{Hsv, Hue};
pub use temperature::correction;

/// A friendlier name for [`Rgb`], so you can write `Color::BLUE`.
pub type Color = Rgb;
