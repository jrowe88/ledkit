use core::time::Duration;

use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::effects::geometry::polar;
use crate::time::millis;
use crate::{math, Animation, Colors, Frame, Position};

/// Rings of color spreading out from the middle, like ripples on a pond. Also
/// known as "concentric".
///
/// On a strip they spread out from the middle LED both ways.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let ripples = Rings::new()
///     .palette(&palettes::OCEAN)
///     .ring_width(3)
///     .outward(false); // rings shrink into the middle
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Rings {
    colors: Colors,
    ring_width: u8,
    cycle: u32,
    outward: bool,
    center: Option<(u16, u16)>,
}

impl Rings {
    /// Rainbow rings, each trip through the colors 4 LEDs wide, moving out
    /// one trip every 2 seconds.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::RAINBOW),
            ring_width: 4,
            cycle: 2000,
            outward: true,
            center: None,
        }
    }

    color_setters!();

    /// How many LEDs one whole trip through the colors spans, from the center
    /// outwards.
    pub const fn ring_width(mut self, leds: u8) -> Self {
        self.ring_width = leds;
        self
    }

    /// How long the rings take to move out by one `ring_width`. 0 stands still.
    pub const fn cycle_time(mut self, cycle: Duration) -> Self {
        self.cycle = millis(cycle);
        self
    }

    /// True (the default) for rings that grow outwards, false for rings that shrink inwards.
    pub const fn outward(mut self, outward: bool) -> Self {
        self.outward = outward;
        self
    }

    /// The LED the rings spread from. The default is the middle.
    pub const fn center(mut self, x: u16, y: u16) -> Self {
        self.center = Some((x, y));
        self
    }
}

impl Default for Rings {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Rings {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let p = polar(at, *frame, self.center);
        let width16 = u64::from(self.ring_width.max(1)) * 16;
        let out = (u64::from(p.distance) * 256 / width16) as u8;
        let shift = math::phase(frame.millis, self.cycle);
        let phase = if self.outward {
            out.wrapping_sub(shift)
        } else {
            out.wrapping_add(shift)
        };
        self.colors.scrolled(phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, grid_at};

    #[test]
    fn same_distance_same_color() {
        let rings = Rings::new();
        let middle = grid_at(&rings, (3, 3), (8, 8), 300);
        for led in [(4, 3), (3, 4), (4, 4)] {
            assert_eq!(grid_at(&rings, led, (8, 8), 300), middle);
        }
        assert_eq!(
            grid_at(&rings, (0, 0), (8, 8), 300),
            grid_at(&rings, (7, 7), (8, 8), 300)
        );
    }

    #[test]
    fn rings_move_out() {
        let rings = Rings::new()
            .ring_width(2)
            .cycle_time(Duration::from_secs(1));
        // After half a cycle the color at the middle has moved out by 1 LED.
        assert_eq!(
            grid_at(&rings, (4, 5), (9, 9), 500),
            grid_at(&rings, (4, 4), (9, 9), 0)
        );
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Rings::new());
        exercise(
            &Rings::new()
                .ring_width(0)
                .cycle_time(Duration::ZERO)
                .outward(false),
        );
    }
}
