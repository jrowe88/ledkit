//! Color temperature (warm to cool white) and white-balance correction.

use crate::rgb::scale_channel;
use crate::Rgb;

/// Turns a float into a channel value: clamped to 0-255 and rounded.
// The value is clamped to 0..=255 first, so the cast cannot truncate or wrap.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_channel(v: f32) -> u8 {
    (v.clamp(0.0, 255.0) + 0.5) as u8
}

impl Rgb {
    /// Makes the color of a glowing hot object at the given temperature in
    /// kelvin (K). Low numbers are warm and orange, high numbers are cool and
    /// bluish. Values are limited to 1000 to 40000 K.
    ///
    /// This uses Tanner Helland's curve fit to blackbody colors, so it is an
    /// approximation, but it looks right for lighting.
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// let warm = Rgb::from_kelvin(2700); // warm, like an incandescent bulb
    /// assert!(warm.r > warm.b);
    /// ```
    // The published curve-fit coefficients are kept exactly as written.
    #[allow(clippy::excessive_precision)]
    pub fn from_kelvin(kelvin: u16) -> Self {
        let t = f32::from(kelvin.clamp(1000, 40000)) / 100.0;

        let r = if t <= 66.0 {
            255.0
        } else {
            329.698_727_446 * libm::powf(t - 60.0, -0.133_204_759_2)
        };
        let g = if t <= 66.0 {
            99.470_802_586_1 * libm::logf(t) - 161.119_568_166_1
        } else {
            288.122_169_528_3 * libm::powf(t - 60.0, -0.075_514_849_2)
        };
        let b = if t >= 66.0 {
            255.0
        } else if t <= 19.0 {
            0.0
        } else {
            138.517_731_223_1 * libm::logf(t - 10.0) - 305.044_792_730_7
        };

        Self::new(to_channel(r), to_channel(g), to_channel(b))
    }

    /// Adjusts the color to fix an LED's tint. Each channel is dimmed by the
    /// matching channel of `correction`, so 255 leaves it alone. See the
    /// [`correction`] constants for ready-made values.
    ///
    /// ```
    /// use ledkit_color::{correction, Rgb};
    /// let fixed = Rgb::WHITE.corrected(correction::TYPICAL_LED_STRIP);
    /// assert_eq!(fixed, Rgb::new(255, 176, 240));
    /// ```
    pub const fn corrected(self, correction: Self) -> Self {
        Self::new(
            scale_channel(self.r, correction.r),
            scale_channel(self.g, correction.g),
            scale_channel(self.b, correction.b),
        )
    }
}

/// Ready-made white-balance corrections for [`Rgb::corrected`].
pub mod correction {
    use crate::Rgb;

    /// No correction: every color comes out unchanged.
    pub const NONE: Rgb = Rgb::WHITE;

    // From FastLED (MIT); see NOTICE. Compensates for the strong green of most WS2812-type strips.
    /// A good starting point for most WS2812-type strips, which look too green.
    pub const TYPICAL_LED_STRIP: Rgb = Rgb::new(255, 176, 240);
}

#[cfg(test)]
mod tests {
    use crate::{correction, Rgb};

    fn close(a: Rgb, b: (u8, u8, u8)) -> bool {
        a.r.abs_diff(b.0) <= 1 && a.g.abs_diff(b.1) <= 1 && a.b.abs_diff(b.2) <= 1
    }
    #[test]
    fn known_temperatures() {
        for (k, rgb) in [
            (1900, (255, 132, 0)),
            (2700, (255, 167, 87)),
            (4000, (255, 206, 166)),
            (6500, (255, 254, 250)),
            (10000, (202, 218, 255)),
        ] {
            let c = Rgb::from_kelvin(k);
            assert!(close(c, rgb), "{k} K gave {c:?}");
        }
    }
    #[test]
    fn out_of_range_is_clamped() {
        assert_eq!(Rgb::from_kelvin(0), Rgb::from_kelvin(1000));
        assert_eq!(Rgb::from_kelvin(u16::MAX), Rgb::from_kelvin(40000));
    }
    #[test]
    fn named_whites_match_formula() {
        assert!(close(
            Rgb::from_kelvin(2700),
            (Rgb::WARM_WHITE.r, Rgb::WARM_WHITE.g, Rgb::WARM_WHITE.b)
        ));
        assert!(close(
            Rgb::from_kelvin(6500),
            (Rgb::COOL_WHITE.r, Rgb::COOL_WHITE.g, Rgb::COOL_WHITE.b)
        ));
    }
    #[test]
    fn no_correction_is_identity() {
        let c = Rgb::new(9, 99, 199);
        assert_eq!(c.corrected(correction::NONE), c);
    }
    #[test]
    fn typical_strip_correction() {
        assert_eq!(
            Rgb::WHITE.corrected(correction::TYPICAL_LED_STRIP),
            Rgb::new(255, 176, 240)
        );
    }
}
