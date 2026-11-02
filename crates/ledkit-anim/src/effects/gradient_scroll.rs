use core::time::Duration;

use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::time::millis;
use crate::{math, Animation, Colors, Direction, Frame, Position};

/// A palette spread along the strip, sliding along. Also known as "palette
/// scroll".
///
/// Palettes that wrap (like the rainbow) slide round and round; ones that
/// don't (like jet) are laid out there and back, so there is no hard edge.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let lava_lamp = GradientScroll::new()
///     .palette(&palettes::LAVA)
///     .cycle_time(Duration::from_secs(8));
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct GradientScroll {
    colors: Colors,
    cycle: u32,
    width: u8,
    direction: Direction,
}

impl GradientScroll {
    /// The rainbow palette, once along the strip, moving round every 5 seconds.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::RAINBOW),
            cycle: 5000,
            width: 1,
            direction: Direction::Forward,
        }
    }

    color_setters!();

    /// How long the colors take to slide once round. 0 stands still.
    pub const fn cycle_time(mut self, cycle: Duration) -> Self {
        self.cycle = millis(cycle);
        self
    }

    /// How many times the palette fits along the strip (or across each row).
    pub const fn width(mut self, repeats: u8) -> Self {
        self.width = repeats;
        self
    }

    /// Which way the colors slide.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
}

impl Default for GradientScroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for GradientScroll {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let len = u64::from(frame.width.max(1));
        let spread = (u64::from(at.x) * 256 * u64::from(self.width) / len) as u8;
        let shift = math::phase(frame.millis, self.cycle);
        let phase = match self.direction {
            Direction::Forward => spread.wrapping_sub(shift),
            Direction::Backward => spread.wrapping_add(shift),
        };
        self.colors.scrolled(phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn two_colors_go_there_and_back() {
        let scroll = GradientScroll::new().colors_between(Rgb::BLACK, Rgb::WHITE);
        assert_eq!(strip_at(&scroll, 0, 10, 0), Rgb::BLACK);
        assert!(strip_at(&scroll, 5, 10, 0).r >= 250);
        assert!(strip_at(&scroll, 9, 10, 0).r < 60);
    }

    #[test]
    fn slides_forward() {
        let scroll = GradientScroll::new();
        assert_eq!(strip_at(&scroll, 1, 8, 625), strip_at(&scroll, 0, 8, 0));
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&GradientScroll::new());
        exercise(&GradientScroll::new().cycle_time(Duration::ZERO).width(0));
    }
}
