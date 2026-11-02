use ledkit_color::Rgb;

use crate::{Animation, Direction, Frame, Position};

/// Groups of lit LEDs marching along, like the lights around a theater sign.
/// Also known as "theater chase", "marquee" or "running lights".
///
/// It follows the wiring order, so on a matrix it snakes along the chain.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// // Two red, three dark, moving at 5 LEDs a second.
/// let marquee = Chase::new()
///     .color(Rgb::RED)
///     .size(2)
///     .spacing(3)
///     .speed(5);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Chase {
    color: Rgb,
    background: Rgb,
    size: u16,
    spacing: u16,
    speed: u16,
    direction: Direction,
}

impl Chase {
    /// One white LED in every three, moving at 10 LEDs a second.
    pub const fn new() -> Self {
        Self {
            color: Rgb::WHITE,
            background: Rgb::BLACK,
            size: 1,
            spacing: 2,
            speed: 10,
            direction: Direction::Forward,
        }
    }

    /// The color of the lit LEDs.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// The color of the gaps. Black unless you set it.
    pub const fn background(mut self, color: Rgb) -> Self {
        self.background = color;
        self
    }

    /// How many LEDs are lit in each group.
    pub const fn size(mut self, leds: u16) -> Self {
        self.size = leds;
        self
    }

    /// How many dark LEDs are between the groups.
    pub const fn spacing(mut self, leds: u16) -> Self {
        self.spacing = leds;
        self
    }

    /// How fast the groups move, in LEDs per second. 0 stands still.
    pub const fn speed(mut self, leds_per_second: u16) -> Self {
        self.speed = leds_per_second;
        self
    }

    /// Which way the groups move.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
}

impl Default for Chase {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Chase {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let period = u64::from(self.size) + u64::from(self.spacing);
        if period == 0 {
            return self.background;
        }
        let moved = u64::from(frame.millis) * u64::from(self.speed) / 1000 % period;
        let index = u64::from(at.index) % period;
        let place = match self.direction {
            Direction::Forward => (index + period - moved) % period,
            Direction::Backward => (index + moved) % period,
        };
        if place < u64::from(self.size) {
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

    fn lit(chase: &Chase, millis: u32) -> [bool; 9] {
        core::array::from_fn(|x| strip_at(chase, x as u16, 9, millis) == Rgb::WHITE)
    }

    #[test]
    fn one_in_three_moving_forward() {
        let chase = Chase::new();
        let t = true;
        let f = false;
        assert_eq!(lit(&chase, 0), [t, f, f, t, f, f, t, f, f]);
        assert_eq!(lit(&chase, 100), [f, t, f, f, t, f, f, t, f]);
        let back = Chase::new().direction(Direction::Backward);
        assert_eq!(lit(&back, 100), [f, f, t, f, f, t, f, f, t]);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Chase::new());
        exercise(&Chase::new().size(0).spacing(0).speed(0));
        exercise(&Chase::new().speed(u16::MAX));
    }
}
