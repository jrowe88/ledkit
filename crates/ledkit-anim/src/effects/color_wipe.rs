use core::time::Duration;

use ledkit_color::Rgb;

use crate::effects::along;
use crate::time::millis;
use crate::{Animation, Direction, Frame, Position};

/// Fills the strip with a color, one LED after another. Also known as "wipe",
/// "fill" or "loading".
///
/// By default it fills once and stays full, which suits a playlist step. With
/// `.looping(true)` it fills, empties again the same way, and repeats.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let loading = ColorWipe::new()
///     .color(Rgb::BLUE)
///     .wipe_time(Duration::from_secs(3))
///     .looping(true);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct ColorWipe {
    color: Rgb,
    background: Rgb,
    wipe: u32,
    direction: Direction,
    looping: bool,
}

impl ColorWipe {
    /// Green over black, filling in 2 seconds, then staying full.
    pub const fn new() -> Self {
        Self {
            color: Rgb::GREEN,
            background: Rgb::BLACK,
            wipe: 2000,
            direction: Direction::Forward,
            looping: false,
        }
    }

    /// The color that fills the strip.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// The color before it is filled. Black unless you set it.
    pub const fn background(mut self, color: Rgb) -> Self {
        self.background = color;
        self
    }

    /// How long filling the whole strip takes. 0 fills it at once.
    pub const fn wipe_time(mut self, wipe: Duration) -> Self {
        self.wipe = millis(wipe);
        self
    }

    /// Which end it fills from: `Forward` starts at the first LED.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// True to empty again and repeat; false (the default) to fill once and stay full.
    pub const fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

impl Default for ColorWipe {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for ColorWipe {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let (x, len) = along(at, *frame, self.direction);
        if self.wipe == 0 {
            return self.color;
        }
        let (x, len, wipe) = (u64::from(x), u64::from(len), u64::from(self.wipe));
        let t = u64::from(frame.millis);
        let lit = if !self.looping || t % (2 * wipe) < wipe {
            // Filling: LEDs before the moving edge are lit.
            let into = if self.looping {
                t % (2 * wipe)
            } else {
                t.min(wipe)
            };
            x < into * len / wipe
        } else {
            // Emptying, from the same end: LEDs after the moving edge stay lit.
            let into = t % (2 * wipe) - wipe;
            x >= into * len / wipe
        };
        if lit {
            self.color
        } else {
            self.background
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn fills_then_holds() {
        let wipe = ColorWipe::new();
        assert_eq!(strip_at(&wipe, 0, 10, 0), Rgb::BLACK);
        assert_eq!(strip_at(&wipe, 4, 10, 1000), Rgb::GREEN);
        assert_eq!(strip_at(&wipe, 5, 10, 1000), Rgb::BLACK);
        assert_eq!(strip_at(&wipe, 9, 10, 5000), Rgb::GREEN);
    }

    #[test]
    fn backward_fills_from_the_far_end() {
        let wipe = ColorWipe::new().direction(Direction::Backward);
        assert_eq!(strip_at(&wipe, 9, 10, 1000), Rgb::GREEN);
        assert_eq!(strip_at(&wipe, 0, 10, 1000), Rgb::BLACK);
    }

    #[test]
    fn looping_empties_and_repeats() {
        let wipe = ColorWipe::new().looping(true);
        assert_eq!(strip_at(&wipe, 8, 10, 1999), Rgb::GREEN);
        assert_eq!(strip_at(&wipe, 0, 10, 3000), Rgb::BLACK);
        assert_eq!(strip_at(&wipe, 9, 10, 3000), Rgb::GREEN);
        assert_eq!(strip_at(&wipe, 9, 10, 4000), Rgb::BLACK);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&ColorWipe::new());
        exercise(&ColorWipe::new().looping(true));
        exercise(&ColorWipe::new().wipe_time(Duration::ZERO).looping(true));
    }
}
