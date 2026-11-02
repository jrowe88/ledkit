use core::time::Duration;

use ledkit_color::Rgb;

use crate::effects::color_setters;
use crate::time::millis;
use crate::{math, Animation, Colors, Frame, Position};

/// LEDs that light up at random, glow and fade. Also known as "twinkle" or
/// "glitter".
///
/// Each LED twinkles on its own timer, so it looks random, but the same time
/// always gives the same picture.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// // Slow, warm fairy lights over a dim blue.
/// let fairy = Sparkle::new()
///     .colors_between(Rgb::WARM_WHITE, Rgb::GOLD)
///     .background(Rgb::new(0, 0, 20))
///     .density(25)
///     .twinkle_time(Duration::from_secs(2));
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Sparkle {
    colors: Colors,
    background: Rgb,
    density: u8,
    twinkle: u32,
}

impl Sparkle {
    /// White sparkles on black, about 1 LED in 10 at a time, each lasting a second.
    pub const fn new() -> Self {
        Self {
            colors: Colors::One(Rgb::WHITE),
            background: Rgb::BLACK,
            density: 10,
            twinkle: 1000,
        }
    }

    color_setters!();

    /// The color between sparkles. Black unless you set it.
    pub const fn background(mut self, color: Rgb) -> Self {
        self.background = color;
        self
    }

    /// Roughly what percentage of the LEDs are sparkling at once, 0 to 100.
    pub const fn density(mut self, percent: u8) -> Self {
        self.density = if percent > 100 { 100 } else { percent };
        self
    }

    /// How long one sparkle takes to glow and fade. 0 turns sparkling off.
    pub const fn twinkle_time(mut self, twinkle: Duration) -> Self {
        self.twinkle = millis(twinkle);
        self
    }
}

impl Default for Sparkle {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Sparkle {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        if self.twinkle == 0 {
            return self.background;
        }
        let led = u32::from(at.index);
        // Each LED is on its own timer, offset so they don't all change together.
        let t = frame.millis.wrapping_add(math::hash(led) % self.twinkle);
        let turn = math::hash2(led, t / self.twinkle);
        if turn % 100 >= u32::from(self.density) {
            return self.background;
        }
        let glow = math::triangle(math::phase(t, self.twinkle));
        let color = self.colors.at((turn >> 8) as u8);
        self.background.lerp(color, glow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    fn lit_count(sparkle: &Sparkle, millis: u32) -> usize {
        (0..100)
            .filter(|&x| strip_at(sparkle, x, 100, millis) != Rgb::BLACK)
            .count()
    }

    #[test]
    fn density_controls_how_many() {
        assert_eq!(lit_count(&Sparkle::new().density(0), 500), 0);
        let full = lit_count(&Sparkle::new().density(100), 500);
        assert!(full > 80, "{full}");
        let some = lit_count(&Sparkle::new().density(10), 500);
        assert!((1..30).contains(&some), "{some}");
    }

    #[test]
    fn same_time_same_picture() {
        let sparkle = Sparkle::new().density(50);
        for x in 0..20 {
            assert_eq!(
                strip_at(&sparkle, x, 20, 777),
                strip_at(&sparkle, x, 20, 777)
            );
        }
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Sparkle::new());
        exercise(&Sparkle::new().density(0).twinkle_time(Duration::ZERO));
    }
}
