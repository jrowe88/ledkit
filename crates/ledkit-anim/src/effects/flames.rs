use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::{math, Animation, Colors, Frame, Position};

/// Flickering flames. Also known as "fire" or "candle".
///
/// On a matrix the flames rise from the bottom row; on a strip they burn from
/// the first LED. `size` is how far up they reach. The colors run from cold
/// (position 0) to hottest (255); the default is the
/// [`FIRE`](ledkit_color::palettes::FIRE) palette, and `.colors_between(start, end)`
/// makes flames that go from `start` at the tips to `end` at the base.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// // Short, magical green flames.
/// let spooky = Flames::new()
///     .colors_between(Rgb::new(0, 80, 0), Rgb::new(150, 255, 60))
///     .size(50);
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Flames {
    colors: Colors,
    size: u8,
    speed: u16,
}

/// How far apart flame "tongues" are, in 1/256ths of a noise hill per LED.
const LED_STEP: u32 = 85;

impl Flames {
    /// The fire palette, reaching 80% of the way up, rising at 8 LEDs a second.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::FIRE),
            size: 80,
            speed: 8,
        }
    }

    color_setters!();

    /// How far up the flames reach, as a percentage of the height (or of the
    /// strip's length), 0 to 100.
    pub const fn size(mut self, percent: u8) -> Self {
        self.size = if percent > 100 { 100 } else { percent };
        self
    }

    /// How fast the flicker rises, in LEDs per second. 0 freezes the flames.
    pub const fn speed(mut self, leds_per_second: u16) -> Self {
        self.speed = leds_per_second;
        self
    }
}

impl Default for Flames {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Flames {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        // `up` counts from the base of the fire, `across` along it.
        let (up, across, extent) = if frame.height <= 1 {
            (u32::from(at.x), 0, u32::from(frame.width))
        } else {
            (
                u32::from(frame.height - 1 - at.y.min(frame.height - 1)),
                u32::from(at.x),
                u32::from(frame.height),
            )
        };
        let reach = extent * u32::from(self.size);
        if up * 100 >= reach {
            return Rgb::BLACK;
        }
        // Hotter near the base, cooling to nothing at the reach.
        let falloff = 255 - up * 100 * 255 / reach;
        let rise =
            (u64::from(frame.millis) * u64::from(self.speed) * u64::from(LED_STEP) / 1000) as u32;
        let flicker = u32::from(math::noise(
            across * LED_STEP * 2,
            (up * LED_STEP).wrapping_sub(rise),
        ));
        let heat = (falloff * (96 + flicker * 159 / 255) / 255) as u8;
        self.colors.at(heat).scale(heat.saturating_mul(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{brightness, exercise, grid_at, strip_at};

    #[test]
    fn hot_at_the_base_dark_above_the_reach() {
        let flames = Flames::new();
        let (mut base, mut higher) = (0, 0);
        for t in (0..4000).step_by(200) {
            for x in 0..8 {
                base += brightness(grid_at(&flames, (x, 7), (8, 8), t));
                higher += brightness(grid_at(&flames, (x, 3), (8, 8), t));
                // 80% of 8 rows is 6.4, so the top row (7 rows up) is dark.
                assert_eq!(grid_at(&flames, (x, 0), (8, 8), t), Rgb::BLACK);
            }
        }
        assert!(base > higher * 2, "{base} vs {higher}");
    }

    #[test]
    fn strips_burn_from_the_start() {
        let flames = Flames::new();
        assert_ne!(strip_at(&flames, 0, 20, 0), Rgb::BLACK);
        assert_eq!(strip_at(&flames, 19, 20, 0), Rgb::BLACK);
    }

    #[test]
    fn flickers_over_time() {
        let flames = Flames::new();
        let a: [Rgb; 8] = core::array::from_fn(|x| grid_at(&flames, (x as u16, 5), (8, 8), 0));
        let b: [Rgb; 8] = core::array::from_fn(|x| grid_at(&flames, (x as u16, 5), (8, 8), 400));
        assert_ne!(a, b);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Flames::new());
        exercise(&Flames::new().size(0).speed(0));
        exercise(&Flames::new().size(100).speed(u16::MAX));
    }
}
