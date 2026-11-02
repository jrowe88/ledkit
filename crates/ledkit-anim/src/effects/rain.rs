use ledkit_color::Rgb;

use crate::effects::{fade, travel16};
use crate::{math, Animation, Frame, Position};

/// Drops falling down the columns of a matrix, with fading trails. Also known
/// as "matrix rain" or "digital rain".
///
/// On a strip each LED is a column one LED tall, so drops show as flickers.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// // A steady blue drizzle.
/// let drizzle = Rain::new()
///     .color(Rgb::new(40, 80, 255))
///     .density(70)
///     .speed(12);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Rain {
    color: Rgb,
    density: u8,
    speed: u16,
    trail: u16,
}

impl Rain {
    /// Green drops in about 40% of the columns, falling 8 LEDs a second, with
    /// trails 4 LEDs long.
    pub const fn new() -> Self {
        Self {
            color: Rgb::GREEN,
            density: 40,
            speed: 8,
            trail: 4,
        }
    }

    /// The color of the drops.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// Roughly what percentage of columns have a drop falling, 0 to 100.
    pub const fn density(mut self, percent: u8) -> Self {
        self.density = if percent > 100 { 100 } else { percent };
        self
    }

    /// How fast the drops fall, in LEDs per second. 0 stops the rain.
    pub const fn speed(mut self, leds_per_second: u16) -> Self {
        self.speed = leds_per_second;
        self
    }

    /// How many LEDs the trail behind each drop fades over.
    pub const fn trail_length(mut self, leds: u16) -> Self {
        self.trail = leds;
        self
    }
}

impl Default for Rain {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Rain {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        if self.speed == 0 {
            return Rgb::BLACK;
        }
        let trail = u64::from(self.trail.max(1));
        // One drop falls the whole column and its trail leaves the bottom.
        let fall = (u64::from(frame.height) + trail) * 1000 / u64::from(self.speed);
        let fall = u32::try_from(fall).unwrap_or(u32::MAX).max(1);
        let column = u32::from(at.x);
        let t = frame.millis.wrapping_add(math::hash(column) % fall);
        if math::hash2(column, t / fall) % 100 >= u32::from(self.density) {
            return Rgb::BLACK;
        }
        let head16 = i64::try_from(travel16(t % fall, self.speed)).unwrap_or(i64::MAX);
        let behind = head16 - i64::from(at.y) * 16;
        let trail16 = i64::from(self.trail.max(1)) * 16;
        if (0..trail16).contains(&behind) {
            self.color.scale(fade(behind, trail16))
        } else {
            Rgb::BLACK
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, grid_at};

    #[test]
    fn no_density_no_rain() {
        let dry = Rain::new().density(0);
        for t in (0..5000).step_by(100) {
            for x in 0..8 {
                for y in 0..8 {
                    assert_eq!(grid_at(&dry, (x, y), (8, 8), t), Rgb::BLACK);
                }
            }
        }
    }

    #[test]
    fn full_density_wets_every_column() {
        let storm = Rain::new().density(100);
        for x in 0..8 {
            let wet = (0..1500)
                .step_by(50)
                .any(|t| (0..8).any(|y| grid_at(&storm, (x, y), (8, 8), t) != Rgb::BLACK));
            assert!(wet, "column {x}");
        }
    }

    #[test]
    fn drops_fall() {
        let storm = Rain::new().density(100).trail_length(1);
        // Find the drop in column 0, then check it is lower a little later.
        let row_of = |t| (0..8).find(|&y| grid_at(&storm, (0, y), (8, 8), t) != Rgb::BLACK);
        let t = (0..1500)
            .step_by(10)
            .find(|&t| row_of(t) == Some(1))
            .unwrap();
        assert_eq!(row_of(t + 250), Some(3));
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Rain::new());
        exercise(&Rain::new().speed(0).trail_length(0).density(100));
        exercise(&Rain::new().speed(u16::MAX));
    }
}
