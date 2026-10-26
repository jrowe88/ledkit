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
pub mod pipeline;
mod pixel;
mod timing;
pub mod wire;

pub use ledkit_color as color;
pub use ledkit_color::{Color, Rgb, Rgbw};

pub use brightness::Brightness;
pub use chip::Chip;
pub use chips::*;
pub use error::*;
pub use pixel::{Channel, ChannelOrder, Pixel};
pub use timing::*;
