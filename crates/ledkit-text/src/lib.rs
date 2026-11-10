#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

mod edges;
mod faces;
mod font;
mod paint;
mod style;
mod text_area;
mod ticker;

pub use edges::Edges;
pub use font::{Font, Size};
pub use paint::Paint;
pub use text_area::{Across, TextArea, UpDown};
pub use ticker::Ticker;

/// Everything you need for text on LEDs, in one `use`.
///
/// ```
/// use ledkit_text::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{Across, Edges, Font, Paint, Size, TextArea, Ticker, UpDown};
    pub use ledkit_anim::{
        Blink, Breathe, Chase, ColorCycle, ColorWipe, Comet, Flames, GradientScroll, Pinwheel,
        Plasma, Rain, Rainbow, Rings, Solid, Sparkle, Swirl, Time,
    };
    pub use ledkit_color::Rgb;
}
