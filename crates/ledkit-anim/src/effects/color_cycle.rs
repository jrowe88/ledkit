use core::time::Duration;

use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::time::millis;
use crate::{math, Animation, Colors, Frame, Position};

/// Every LED the same color, slowly changing through a palette. Also known as
/// "fade".
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let mood = ColorCycle::new()
///     .palette(&palettes::OCEAN)
///     .cycle_time(Duration::from_secs(30));
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct ColorCycle {
    colors: Colors,
    cycle: u32,
}

impl ColorCycle {
    /// The rainbow, once round every 10 seconds.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::RAINBOW),
            cycle: 10_000,
        }
    }

    color_setters!();

    /// How long one trip through the colors takes. 0 stays on the first color.
    pub const fn cycle_time(mut self, cycle: Duration) -> Self {
        self.cycle = millis(cycle);
        self
    }
}

impl Default for ColorCycle {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for ColorCycle {
    fn color_at(&self, _at: Position, frame: &Frame) -> Rgb {
        self.colors.scrolled(math::phase(frame.millis, self.cycle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn whole_strip_one_color_that_changes() {
        let cycle = ColorCycle::new();
        assert_eq!(strip_at(&cycle, 0, 10, 2500), strip_at(&cycle, 9, 10, 2500));
        assert_ne!(strip_at(&cycle, 0, 10, 0), strip_at(&cycle, 0, 10, 2500));
        assert_eq!(strip_at(&cycle, 0, 10, 0), Rgb::RED);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&ColorCycle::new());
        exercise(&ColorCycle::new().cycle_time(Duration::ZERO).color(Rgb::RED));
    }
}
