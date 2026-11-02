use core::time::Duration;

use ledkit_color::{Hsv, Hue, Rgb};

use crate::time::millis;
use crate::{math, Animation, Direction, Frame, Position};

/// A moving rainbow along the strip. Also known as "color loop".
///
/// On a matrix, each row shows the rainbow.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// // Two rainbows along the strip, going round once every 10 seconds.
/// let rainbow = Rainbow::new().width(2).cycle_time(Duration::from_secs(10));
/// let mut leds = [Rgb::BLACK; 30];
/// rainbow.render(&mut leds, Time::from_millis(0));
/// assert_eq!(leds[0], Rgb::RED);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Rainbow {
    cycle: u32,
    width: u8,
    saturation: u8,
    direction: Direction,
}

impl Rainbow {
    /// One rainbow along the strip, moving forward, going round every 5 seconds.
    pub const fn new() -> Self {
        Self {
            cycle: 5000,
            width: 1,
            saturation: 255,
            direction: Direction::Forward,
        }
    }

    /// How long a color takes to go once round the color wheel. 0 stands still.
    pub const fn cycle_time(mut self, cycle: Duration) -> Self {
        self.cycle = millis(cycle);
        self
    }

    /// How many whole rainbows fit along the strip (or across each row).
    /// 0 makes every LED the same color.
    pub const fn width(mut self, rainbows: u8) -> Self {
        self.width = rainbows;
        self
    }

    /// How strong the colors are: 255 is full color, lower is paler, 0 is white.
    pub const fn saturation(mut self, saturation: u8) -> Self {
        self.saturation = saturation;
        self
    }

    /// Which way the colors move.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
}

impl Default for Rainbow {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Rainbow {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let len = u64::from(frame.width.max(1));
        let spread = (u64::from(at.x) * 65536 * u64::from(self.width) / len) as u16;
        let shift = math::phase16(frame.millis, self.cycle);
        let hue = match self.direction {
            Direction::Forward => spread.wrapping_sub(shift),
            Direction::Backward => spread.wrapping_add(shift),
        };
        Rgb::from(Hsv::new(Hue(hue), self.saturation, 255))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn starts_red() {
        assert_eq!(strip_at(&Rainbow::new(), 0, 10, 0), Rgb::RED);
    }

    #[test]
    fn colors_move_forward() {
        let rainbow = Rainbow::new();
        // One LED's worth of travel on a 10-LED strip takes a tenth of the cycle.
        assert_eq!(strip_at(&rainbow, 1, 10, 500), strip_at(&rainbow, 0, 10, 0));
        let backward = Rainbow::new().direction(Direction::Backward);
        assert_eq!(
            strip_at(&backward, 0, 10, 500),
            strip_at(&backward, 1, 10, 0)
        );
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Rainbow::new());
        exercise(
            &Rainbow::new()
                .cycle_time(Duration::ZERO)
                .width(0)
                .saturation(0),
        );
        assert_eq!(
            strip_at(&Rainbow::new().saturation(0), 3, 10, 0),
            Rgb::WHITE
        );
    }
}
