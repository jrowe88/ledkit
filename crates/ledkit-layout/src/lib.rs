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
//! To use part of a strip as a strip of its own, see [`Segment`].
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

#![cfg_attr(not(test), no_std)]

mod canvas;
mod layout;
mod segment;
#[cfg(test)]
pub(crate) mod test_support;
mod transform;

pub use canvas::{Canvas, LayoutTooBig};
pub use layout::{Corner, Layout, MatrixLayout, Order, Wiring};
pub use segment::{Segment, SegmentMut};
pub use transform::{Rotated, Rotation, Tiled};
