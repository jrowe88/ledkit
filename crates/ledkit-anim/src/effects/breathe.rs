use core::time::Duration;

use ledkit_color::Rgb;

use crate::time::millis;
use crate::{math, Animation, Frame, Position};

/// Slowly brightens and dims, like breathing. Also known as "pulse".
///
/// The level follows a smooth curve from the lowest level up to the highest
/// and back, once per period.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// // A calm green glow that never goes fully dark.
/// let calm = Breathe::new()
///     .color(Rgb::GREEN)
///     .period(Duration::from_secs(6))
///     .levels(30, 255);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Breathe {
    color: Rgb,
    period: u32,
    min: u8,
    max: u8,
}

impl Breathe {
    /// Blue, one breath every 4 seconds, from off to full.
    pub const fn new() -> Self {
        Self {
            color: Rgb::BLUE,
            period: 4000,
            min: 0,
            max: 255,
        }
    }

    /// The color at the highest level.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// How long one breath (dim, bright, dim again) takes. A period of 0
    /// stays at the lowest level.
    pub const fn period(mut self, period: Duration) -> Self {
        self.period = millis(period);
        self
    }

    /// The lowest and highest levels, 0 (off) to 255 (the full color).
    pub const fn levels(mut self, min: u8, max: u8) -> Self {
        self.min = min;
        self.max = max;
        self
    }
}

impl Default for Breathe {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Breathe {
    fn color_at(&self, _at: Position, frame: &Frame) -> Rgb {
        // A raised cosine: 0 at the start of the period, 255 halfway through.
        let phase = math::phase(frame.millis, self.period);
        let wave = 255 - math::sine(phase.wrapping_add(64));
        self.color.scale(math::lerp_u8(self.min, self.max, wave))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn dark_then_bright() {
        let breathe = Breathe::new();
        assert_eq!(strip_at(&breathe, 0, 5, 0), Rgb::BLACK);
        assert_eq!(strip_at(&breathe, 0, 5, 2000), Rgb::BLUE);
        assert_eq!(strip_at(&breathe, 0, 5, 4000), Rgb::BLACK);
    }

    #[test]
    fn levels_are_respected() {
        let breathe = Breathe::new().color(Rgb::WHITE).levels(50, 200);
        assert_eq!(strip_at(&breathe, 0, 5, 0), Rgb::WHITE.scale(50));
        assert_eq!(strip_at(&breathe, 0, 5, 2000), Rgb::WHITE.scale(200));
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Breathe::new());
        exercise(&Breathe::new().period(Duration::ZERO).levels(200, 0));
    }
}
