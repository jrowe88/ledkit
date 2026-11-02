use core::time::Duration;

use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::effects::geometry::polar;
use crate::time::millis;
use crate::{math, Animation, Colors, Frame, Position};

/// Soft blobs of color flowing into each other, the classic demo-scene plasma.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let lava = Plasma::new()
///     .palette(&palettes::LAVA)
///     .scale(6)
///     .cycle_time(Duration::from_secs(10));
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Plasma {
    colors: Colors,
    scale: u8,
    cycle: u32,
}

impl Plasma {
    /// Rainbow colors, blobs about 8 LEDs across, a slow 6-second cycle.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::RAINBOW),
            scale: 8,
            cycle: 6000,
        }
    }

    color_setters!();

    /// Roughly how many LEDs across a blob is. Bigger is smoother.
    pub const fn scale(mut self, leds: u8) -> Self {
        self.scale = leds;
        self
    }

    /// How long the slowest wave takes to come round. 0 stands still.
    pub const fn cycle_time(mut self, cycle: Duration) -> Self {
        self.cycle = millis(cycle);
        self
    }
}

impl Default for Plasma {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Plasma {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        // Phase steps per LED, so one wave spans `scale` LEDs.
        let step = 256 / u32::from(self.scale.max(1));
        let (x, y) = (u32::from(at.x) * step, u32::from(at.y) * step);
        let distance = polar(at, *frame, None).distance * step / 16;
        // Each wave moves at its own speed, so the pattern never quite repeats.
        let t = |speed: u32| u32::from(math::phase(frame.millis.wrapping_mul(speed), self.cycle));
        let waves = [
            math::sine((x + t(1)) as u8),
            math::sine((y + 256 - t(2)) as u8),
            math::sine(((x + y) / 2 + t(3)) as u8),
            math::sine((distance + 256 - t(1)) as u8),
        ];
        let sum: u32 = waves.iter().map(|&w| u32::from(w)).sum();
        self.colors.scrolled((sum / 2) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, grid_at};

    #[test]
    fn flows_over_time() {
        let plasma = Plasma::new();
        let a: [Rgb; 8] = core::array::from_fn(|x| grid_at(&plasma, (x as u16, 2), (8, 8), 0));
        let b: [Rgb; 8] = core::array::from_fn(|x| grid_at(&plasma, (x as u16, 2), (8, 8), 500));
        assert_ne!(a, b);
        let again: [Rgb; 8] =
            core::array::from_fn(|x| grid_at(&plasma, (x as u16, 2), (8, 8), 500));
        assert_eq!(b, again);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Plasma::new());
        exercise(&Plasma::new().scale(0).cycle_time(Duration::ZERO));
    }
}
