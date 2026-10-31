//! Layouts for ledkit: say where your LEDs are, then draw by `(x, y)`.
//!
//! An LED matrix is one long chain of LEDs folded into a grid. A [`Layout`]
//! knows how the chain is folded, so you can ask "which LED is at column 3,
//! row 2?" instead of counting.
//!
//! ## Quick tour
//!
//! Describe your panel once, as a `const`:
//!
//! ```
//! use ledkit_layout::{Layout, MatrixLayout};
//!
//! const LAYOUT: MatrixLayout = MatrixLayout::new(16, 16).serpentine();
//!
//! assert_eq!(LAYOUT.len(), 256);
//! // (0, 0) is the top-left of the picture. Row 1 runs right to left.
//! assert_eq!(LAYOUT.index(0, 0), Some(0));
//! assert_eq!(LAYOUT.index(15, 1), Some(16));
//! // Outside the panel there is no LED.
//! assert_eq!(LAYOUT.index(16, 0), None);
//! ```
//!
//! `x` grows to the right and `y` grows down, as you look at the panel.
//!
//! ## Drawing
//!
//! A [`Canvas`] lets you draw on a strip's pixels by `(x, y)`. Make it in its
//! own block: it borrows the pixels, so the strip can't send them until you
//! are done drawing.
//!
//! ```
//! use ledkit_core::prelude::*;
//! use ledkit_core::testing::MemoryOutput;
//! use ledkit_layout::{Canvas, MatrixLayout};
//!
//! let mut strip = Strip::<Ws2812b, 12, _>::new(MemoryOutput::<_, 12>::new());
//!
//! {
//!     let layout = MatrixLayout::new(4, 3).serpentine();
//!     let mut canvas = Canvas::new(strip.pixels_mut(), layout).unwrap();
//!     canvas.set(0, 1, Rgb::RED);
//!     canvas.set(99, 99, Rgb::RED); // off the canvas: ignored
//! }
//! strip.show().unwrap();
//!
//! assert_eq!(strip.output().last_frame()[7], Rgb::RED);
//! ```
//!
//! ## Segments
//!
//! To use part of a strip as a strip of its own, use a [`Segment`]: a start, a
//! length, and optionally `.reversed()` to count from the far end. This suits a
//! light bar with one side wired back-to-back with the other.
//!
//! ```
//! use ledkit_color::Rgb;
//! use ledkit_layout::Segment;
//!
//! let mut pixels = [Rgb::BLACK; 10];
//! let mut segment = Segment::new(2, 5).reversed().of(&mut pixels).unwrap();
//! segment.set(0, Rgb::RED); // the segment's first LED is its far end
//! assert_eq!(pixels[6], Rgb::RED);
//! ```
//!
//! ## Rotation and tiling
//!
//! Mounted a panel sideways or upside down? Turn the layout, not the wiring.
//! `.rotated(Rotation::Quarter)` turns it clockwise as you look at the panel, and
//! `(0, 0)` stays at the top-left of what you see. A quarter or three-quarter turn
//! swaps the width and height.
//!
//! ```
//! use ledkit_layout::{Layout, MatrixLayout, Rotation};
//!
//! // A tall 8x32 panel lying on its side is a wide 32x8 picture.
//! let sideways = MatrixLayout::new(8, 32).rotated(Rotation::Quarter);
//! assert_eq!((sideways.width(), sideways.height()), (32, 8));
//! ```
//!
//! To join several panels into one big picture, use [`Tiled`]. It takes the
//! panel, then how many panels go across and down. Chain `.serpentine()`,
//! `.columns_first()` or `.starting_at(...)` to say how the *panels* are chained;
//! each panel keeps its own wiring.
//!
//! ```
//! use ledkit_layout::{Layout, MatrixLayout, Tiled};
//!
//! // Four 8x8 panels across and two down make a 32x16 picture.
//! let wall = Tiled::new(MatrixLayout::new(8, 8), 4, 2);
//! assert_eq!((wall.width(), wall.height()), (32, 16));
//! assert_eq!(wall.len(), 512);
//! ```
//!
//! ## Text and shapes
//!
//! With the `embedded-graphics` feature, a [`Canvas`] is an
//! [`embedded-graphics`](https://docs.rs/embedded-graphics) draw target, so
//! you can draw text, lines, shapes and images. Anything that falls off the
//! canvas is clipped.
//!
//! ```
//! # #[cfg(feature = "embedded-graphics")] {
//! use embedded_graphics::mono_font::ascii::FONT_4X6;
//! use embedded_graphics::mono_font::MonoTextStyle;
//! use embedded_graphics::pixelcolor::Rgb888;
//! use embedded_graphics::prelude::*;
//! use embedded_graphics::text::Text;
//! use ledkit_color::Rgb;
//! use ledkit_layout::{Canvas, MatrixLayout};
//!
//! let mut pixels = [Rgb::BLACK; 64];
//! let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(8, 8)).unwrap();
//!
//! let style = MonoTextStyle::new(&FONT_4X6, Rgb888::WHITE);
//! Text::new("Hi", Point::new(0, 6), style).draw(&mut canvas).unwrap();
//! # assert!(pixels.iter().any(|p| *p != Rgb::BLACK));
//! # }
//! ```
//!
//! `embedded-graphics` colors are `Rgb888`. `rgb_from_rgb888` and
//! `rgb888_from_rgb` (with the feature on) convert to and from ledkit's `Rgb`.
//! The matrix example (`examples/esp32c6-matrix`) draws shapes and scrolling
//! text this way.
//!
//! ## Finding your wiring
//!
//! Panels differ in where the data wire goes in and which way the chain
//! snakes, and unbranded ones rarely say. Light the panel with the wiring
//! finder from the matrix example. It lights LED 0 red, LEDs 1 to `WIDTH - 1`
//! green, and the first 3 LEDs of the next line (LEDs `WIDTH` to `WIDTH + 2`)
//! blue. Red plus green together cover `WIDTH` LEDs; the green alone is
//! `WIDTH - 1`. Look at the panel:
//!
//! - Red to the end of the green is where the first line runs.
//! - Where the blue starts is where the second line starts. Blue next to the end
//!   of the green means serpentine; blue back at the red's side means
//!   progressive.
//!
//! The finder assumes the panel is wired rows first. If the green runs down a
//! column instead, or wraps onto a second line, the panel is wired columns first
//! (or `WIDTH` is wrong). Settle rows versus columns before you judge serpentine
//! from the blue LEDs.
//!
//! Then use the table:
//!
//! | What you see | Builder call |
//! |---|---|
//! | Red is in the top-left corner | nothing (the default) |
//! | Red is in the top-right corner | `.starting_at(Corner::TopRight)` |
//! | Red is in the bottom-left corner | `.starting_at(Corner::BottomLeft)` |
//! | Red is in the bottom-right corner | `.starting_at(Corner::BottomRight)` |
//! | Red to the end of the green runs along a row | nothing (rows first) |
//! | Red to the end of the green runs down a column | `.columns_first()` |
//! | Blue starts next to the end of the green | `.serpentine()` |
//! | Blue starts back at the red's side | nothing (progressive) |
//!
//! Use them together, for example
//! `MatrixLayout::new(8, 32).columns_first().serpentine().starting_at(Corner::BottomLeft)`.
//!
//! ## Power on big matrices
//!
//! A matrix draws a lot of current. Each LED can pull about 60 mA at full white,
//! so a 16x16 (256 LEDs) at full white can draw over 10 A, far more than a USB
//! port can give. The matrix example starts with a 500 mA power limit and 10%
//! brightness, which is safe on USB. Use `set_power_limit` and ledkit dims the
//! whole picture just enough to stay under it, so a dim matrix is the limit
//! working:
//!
//! ```
//! use ledkit_core::prelude::*;
//! use ledkit_core::testing::MemoryOutput;
//!
//! let mut strip = Strip::<Ws2812b, 64, _>::new(MemoryOutput::<Rgb, 64>::new());
//! strip.set_power_limit(Milliamps(500)); // the example's default
//! ```
//!
//! To raise it, use a proper 5 V supply rated for the current you set, with
//! thick enough wires, and raise the limit and the brightness together. Don't
//! power a big matrix from the board's USB port. Connect the LED supply's
//! ground to the board's ground, or the data signal has no reference and the
//! LEDs misbehave.
//!
//! ## Tested on hardware
//!
//! **Status: pending hardware test.** The `esp32c6-matrix` example builds in CI but has not been run on a matrix yet.

#![cfg_attr(not(test), no_std)]

mod canvas;
#[cfg(feature = "embedded-graphics")]
mod graphics;
mod layout;
mod segment;
#[cfg(test)]
pub(crate) mod test_support;
mod transform;

pub use canvas::{Canvas, LayoutTooBig};
#[cfg(feature = "embedded-graphics")]
pub use graphics::{rgb888_from_rgb, rgb_from_rgb888};
pub use layout::{Corner, Layout, MatrixLayout, Order, Wiring};
pub use segment::{Segment, SegmentMut};
pub use transform::{Rotated, Rotation, Tiled};
