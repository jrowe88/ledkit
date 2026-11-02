//! The built-in effects.

mod blink;
mod breathe;
mod chase;
mod color_cycle;
mod color_wipe;
mod comet;
mod flames;
pub(crate) mod geometry;
mod gradient_scroll;
mod pinwheel;
mod plasma;
mod rain;
mod rainbow;
mod rings;
mod solid;
mod sparkle;
mod swirl;

pub use blink::Blink;
pub use breathe::Breathe;
pub use chase::Chase;
pub use color_cycle::ColorCycle;
pub use color_wipe::ColorWipe;
pub use comet::Comet;
pub use flames::Flames;
pub use gradient_scroll::GradientScroll;
pub use pinwheel::Pinwheel;
pub use plasma::Plasma;
pub use rain::Rain;
pub use rainbow::Rainbow;
pub use rings::Rings;
pub use solid::Solid;
pub use sparkle::Sparkle;
pub use swirl::Swirl;

use crate::{Direction, Frame, Position};

/// The four color setters shared by every effect with a `colors` field.
macro_rules! color_setters {
    () => {
        /// The colors to draw with: one color, two to blend between, or a color map.
        pub const fn colors(mut self, colors: $crate::Colors) -> Self {
            self.colors = colors;
            self
        }

        /// Draws with a color map, such as one from
        /// [`palettes`](ledkit_color::palettes), for example `.palette(&palettes::OCEAN)`.
        pub const fn palette(mut self, palette: &'static dyn ledkit_color::ColorMap) -> Self {
            self.colors = $crate::Colors::Map(palette);
            self
        }

        /// Draws with a smooth blend from `start` to `end`.
        pub const fn colors_between(
            mut self,
            start: ledkit_color::Rgb,
            end: ledkit_color::Rgb,
        ) -> Self {
            self.colors = $crate::Colors::Two(start, end);
            self
        }

        /// Draws with a single color.
        pub const fn color(mut self, color: ledkit_color::Rgb) -> Self {
            self.colors = $crate::Colors::One(color);
            self
        }
    };
}
pub(crate) use color_setters;

/// Where an LED is along a strip-like effect, and how long the strip is: the
/// column and the width (on a matrix, effects along a strip run along each row).
/// `Backward` counts from the far end.
pub(crate) fn along(at: Position, frame: Frame, direction: Direction) -> (u32, u32) {
    let len = u32::from(frame.width);
    let x = u32::from(at.x).min(len.saturating_sub(1));
    match direction {
        Direction::Forward => (x, len),
        Direction::Backward => (len.saturating_sub(1) - x, len),
    }
}

/// The brightness of a tail or trail `behind` the head (from 0 at the head
/// to `length`), fading in a straight line: 255 at the head, near 0 at the end.
pub(crate) fn fade(behind: i64, length: i64) -> u8 {
    u8::try_from(255 - behind * 255 / length.max(1)).unwrap_or(0)
}

/// How far something moving at `speed` LEDs per second has gone after
/// `millis`, in 1/16ths of an LED.
pub(crate) const fn travel16(millis: u32, speed: u16) -> u64 {
    millis as u64 * speed as u64 * 16 / 1000
}

#[cfg(test)]
pub(crate) mod test_support {
    use ledkit_color::Rgb;
    use ledkit_layout::{Canvas, MatrixLayout};

    use crate::{Animation, Frame, Position, Time};

    /// Renders `animation` onto awkward surfaces at awkward times; it must not panic.
    pub(crate) fn exercise(animation: &impl Animation) {
        for t in [0, 1, 999, 1234, 60_000, u32::MAX / 2, u32::MAX] {
            let now = Time::from_millis(t);
            let mut none: [Rgb; 0] = [];
            animation.render(&mut none, now);
            let mut one = [Rgb::BLACK; 1];
            animation.render(&mut one, now);
            let mut two = [Rgb::BLACK; 2];
            animation.render(&mut two, now);
            let mut strip = [Rgb::BLACK; 300];
            animation.render(&mut strip, now);
            let mut pixel = [Rgb::BLACK; 1];
            animation.render(
                &mut Canvas::new(&mut pixel, MatrixLayout::new(1, 1)).unwrap(),
                now,
            );
            let mut grid = [Rgb::BLACK; 64];
            animation.render(
                &mut Canvas::new(&mut grid, MatrixLayout::new(8, 8)).unwrap(),
                now,
            );
            let mut odd = [Rgb::BLACK; 35];
            animation.render(
                &mut Canvas::new(&mut odd, MatrixLayout::new(7, 5)).unwrap(),
                now,
            );
            let mut big = [Rgb::BLACK; 2048];
            animation.render(
                &mut Canvas::new(&mut big, MatrixLayout::new(64, 32)).unwrap(),
                now,
            );
        }
    }

    /// The color of LED `x` on a strip `len` long at `millis`.
    pub(crate) fn strip_at(animation: &impl Animation, x: u16, len: u16, millis: u32) -> Rgb {
        animation.color_at(Position { x, y: 0, index: x }, &Frame::new(millis, len, 1))
    }

    /// The color at `(x, y)` on a `width` x `height` matrix (wired row by row) at `millis`.
    pub(crate) fn grid_at(
        animation: &impl Animation,
        (x, y): (u16, u16),
        (width, height): (u16, u16),
        millis: u32,
    ) -> Rgb {
        animation.color_at(
            Position {
                x,
                y,
                index: y * width + x,
            },
            &Frame::new(millis, width, height),
        )
    }

    /// Adds up all three channels: a rough "how bright".
    pub(crate) fn brightness(color: Rgb) -> u32 {
        u32::from(color.r) + u32::from(color.g) + u32::from(color.b)
    }
}
