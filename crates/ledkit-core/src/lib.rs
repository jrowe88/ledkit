//! Core of ledkit: LED chip catalogue, signal encoders and the pixel buffer.
//!
//! This crate works without `std` or an allocator. It is part of the ledkit
//! family of crates.
//!
//! ## Features
//!
//! - `smart-leds`: interoperability with the `smart-leds-trait` crate.
#![cfg_attr(not(test), no_std)]

mod brightness;
mod chip;
mod chips;
pub mod encode;
mod error;
mod output;
pub mod pipeline;
mod pixel;
pub mod power;
mod strip;
pub mod testing;
mod timing;
pub mod wire;

pub use ledkit_color as color;
pub use ledkit_color::{Color, Rgb, Rgbw};

pub use brightness::Brightness;
pub use chip::Chip;
pub use chips::*;
pub use error::*;
pub use output::{Output, OutputAsync};
pub use pixel::{Channel, ChannelOrder, Pixel};
pub use power::{Milliamps, PowerModel};
pub use strip::Strip;
pub use timing::*;

/// Everything you usually need, in one import: `use ledkit_core::prelude::*;`.
pub mod prelude {
    pub use crate::chips::*;
    pub use crate::{Brightness, Chip, Color, Milliamps, Output, OutputAsync, Rgb, Rgbw, Strip};
    pub use ledkit_color::{correction, Hsv, Hue, GAMMA_2_2, GAMMA_2_6};
}
