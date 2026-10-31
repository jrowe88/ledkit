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
//! ## Finding your wiring
//!
//! Panels differ in where the data wire goes in and which way the chain
//! snakes, and unbranded ones rarely say. Light the panel with the wiring
//! finder from the matrix example. It lights LED 0 red, LEDs 1 to `WIDTH - 1`
//! green, and the first 3 LEDs of the next line (LEDs `WIDTH` to `WIDTH + 2`)
//! blue. Look at the panel:
//!
//! - Red to the end of the green is where the first line runs.
//! - Where the blue starts is where the second line starts. Blue next to the
//!   end of the green means serpentine; blue back at the red's side means
//!   progressive.
//!
//! The green run is as long as the `WIDTH` you set. If the green wraps around
//! onto a second line, the panel is wired columns first, or the width is
//! wrong.
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

mod layout;
#[cfg(test)]
pub(crate) mod test_support;
mod transform;

pub use layout::{Corner, Layout, MatrixLayout, Order, Wiring};
pub use transform::{Rotated, Rotation, Tiled};
