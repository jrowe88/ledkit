//! Animations for LED strips and matrices.
#![cfg_attr(not(test), no_std)]

mod animation;
mod colors;
mod combine;
mod effect;
mod effects;
pub mod math;
mod surface;
mod time;

pub use animation::{Animation, Direction, Frame, Position};
pub use colors::Colors;
pub use combine::*;
pub use effect::Effect;
pub use effects::*;
pub use surface::Surface;
pub use time::Time;

/// Everything you need for a typical animation, in one `use`.
///
/// ```
/// use ledkit_anim::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{Animation, AnimationExt, Colors, Direction, Frame, Position, Surface, Time};
    pub use crate::{Blend, Easing, Edge, Effect, Entry, Playlist, Segments, Style, Transition};
    pub use crate::{
        Blink, Breathe, Chase, ColorCycle, ColorWipe, Comet, Flames, GradientScroll, Pinwheel,
        Plasma, Rain, Rainbow, Rings, Solid, Sparkle, Swirl,
    };
    pub use ledkit_color::{palettes, Color, ColorMap, Rgb};
}
