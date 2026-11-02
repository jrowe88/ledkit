//! Ways to combine animations: modifiers, layers, transitions, playlists and
//! segments.

mod layer;
mod modifiers;
mod playlist;
mod segments;
pub(crate) mod then;
mod transition;

use core::time::Duration;

pub use layer::{Blend, Layer};
pub use modifiers::{AtSpeed, Dimmed, Mirrored, Reversed, StartingAt};
pub use playlist::{Entry, Playlist};
pub use segments::Segments;
pub use then::Then;
pub use transition::{Easing, Edge, Style, Transition};

use crate::time::millis;
use crate::Animation;

/// Extra methods on every animation, to change it or combine it with others.
///
/// They read left to right: `Rainbow::new().reversed().dimmed(128)` is a
/// rainbow, running backwards, at half brightness.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let show = Rainbow::new()
///     .layer(Comet::new().color(Rgb::WHITE), Blend::Over)
///     .crossfade_to(Flames::new(), Duration::from_secs(20), Duration::from_secs(2));
/// ```
pub trait AnimationExt: Animation + Sized {
    /// The animation run from the far end: the last LED becomes the first.
    fn reversed(self) -> Reversed<Self> {
        Reversed { inner: self }
    }

    /// The animation drawn on the first half of the strip (or of each row)
    /// and mirrored onto the second half.
    fn mirrored(self) -> Mirrored<Self> {
        Mirrored { inner: self }
    }

    /// The animation sped up or slowed down: 200 is twice as fast, 50 is half
    /// speed, 0 freezes it at its first frame.
    ///
    /// Sped up, its clock runs out sooner: at 200 it starts again from 0 after
    /// about 24 days instead of 49, at 1000 after about 5.
    fn at_speed(self, percent: u16) -> AtSpeed<Self> {
        AtSpeed {
            inner: self,
            percent,
        }
    }

    /// The animation held at its first frame until `start`, then played from
    /// its beginning. Useful to restart an effect part-way through a program.
    fn starting_at(self, start: Duration) -> StartingAt<Self> {
        StartingAt {
            inner: self,
            start: millis(start),
        }
    }

    /// The animation made dimmer: 255 leaves it alone, 128 is about half, 0 is off.
    fn dimmed(self, level: u8) -> Dimmed<Self> {
        Dimmed { inner: self, level }
    }

    /// `top` laid over this animation, combined with `blend`.
    fn layer<T: Animation>(self, top: T, blend: Blend) -> Layer<Self, T> {
        Layer {
            base: self,
            top,
            blend,
        }
    }

    /// This animation until `at`, then `transition` into `next`. `next` starts
    /// from its beginning when the transition starts.
    fn then<B: Animation>(self, next: B, at: Duration, transition: Transition) -> Then<Self, B> {
        Then {
            first: self,
            next,
            at: millis(at),
            transition,
        }
    }

    /// This animation until `at`, then a crossfade lasting `over` into `next`.
    fn crossfade_to<B: Animation>(self, next: B, at: Duration, over: Duration) -> Then<Self, B> {
        self.then(next, at, Transition::fade(over))
    }
}

impl<A: Animation> AnimationExt for A {}
