#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

mod edges;
mod faces;
mod font;
mod paint;

pub use edges::Edges;
pub use font::{Font, Size};
pub use paint::Paint;

/// Everything you need for text on LEDs, in one `use`.
///
/// ```
/// use ledkit_text::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{Edges, Font, Paint, Size};
    pub use ledkit_anim::{
        Blink, Breathe, Chase, ColorCycle, ColorWipe, Comet, Flames, GradientScroll, Pinwheel,
        Plasma, Rain, Rainbow, Rings, Solid, Sparkle, Swirl,
    };
    pub use ledkit_color::Rgb;
}
