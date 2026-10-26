//! Overall strip brightness.

use ledkit_color::GAMMA_2_2;

/// How bright the whole strip is.
///
/// By default brightness is *perceived*: `0.5` looks about half as bright to
/// your eyes, even though the LEDs only get about a fifth of full power
/// (eyes are much more sensitive to dim light than to bright light). Use
/// [`Brightness::from_fraction`] or [`Brightness::from_percent`] for this.
///
/// If you would rather talk to the LEDs directly (for example to limit power
/// or match a measured level), use [`Brightness::exact`]. It sends the number
/// you give it straight to the LEDs with no curve applied, so `exact(128)` is
/// half of full power, which looks much brighter than "half".
///
/// Two brightnesses are equal only if they were made the same way: the
/// perceived [`Brightness::FULL`] and `Brightness::exact(255)` both send 255
/// to the LEDs, but `Brightness::exact(255) != Brightness::FULL`. To compare
/// what the LEDs actually get, compare [`output_level`](Brightness::output_level).
/// [`Strip::effective_brightness`](crate::Strip::effective_brightness) is
/// always an exact brightness.
///
/// # Example
///
/// ```
/// use ledkit_core::Brightness;
///
/// // Looks half as bright; the LEDs get a level of 56 out of 255.
/// assert_eq!(Brightness::from_fraction(0.5).output_level(), 56);
/// // Exactly 128 out of 255, no curve.
/// assert_eq!(Brightness::exact(128).output_level(), 128);
///
/// // Same output level, made differently, so not equal.
/// assert_ne!(Brightness::exact(255), Brightness::FULL);
/// assert_eq!(Brightness::exact(255).output_level(), Brightness::FULL.output_level());
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Brightness {
    level: u8,
    exact: bool,
}

impl Brightness {
    /// Full brightness.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// assert_eq!(Brightness::FULL.output_level(), 255);
    /// ```
    pub const FULL: Self = Self {
        level: 255,
        exact: false,
    };

    /// Completely off.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// assert_eq!(Brightness::OFF.output_level(), 0);
    /// ```
    pub const OFF: Self = Self {
        level: 0,
        exact: false,
    };

    /// Perceived brightness from a fraction: `0.0` is off, `1.0` is full.
    ///
    /// Values outside that range are clamped, and NaN means off.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// let half = Brightness::from_fraction(0.5); // looks half as bright
    /// assert_eq!(half.output_level(), 56);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255 first
    pub fn from_fraction(fraction: f32) -> Self {
        if fraction.is_nan() {
            return Self::OFF;
        }
        Self {
            level: (fraction.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            exact: false,
        }
    }

    /// Perceived brightness from a percentage: `0` is off, `100` is full.
    ///
    /// Anything above 100 counts as 100.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// assert_eq!(Brightness::from_percent(50), Brightness::from_fraction(0.5));
    /// assert_eq!(Brightness::from_percent(200), Brightness::FULL);
    /// ```
    #[must_use]
    pub const fn from_percent(percent: u8) -> Self {
        let p = if percent > 100 { 100 } else { percent } as u16;
        Self {
            #[allow(clippy::cast_possible_truncation)] // at most (100 * 255 + 50) / 100 = 255
            level: ((p * 255 + 50) / 100) as u8,
            exact: false,
        }
    }

    /// An exact output level from 0 to 255, sent to the LEDs with no curve.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// // Half the power, with no curve: looks much brighter than half.
    /// assert_eq!(Brightness::exact(128).output_level(), 128);
    /// ```
    #[must_use]
    pub const fn exact(raw: u8) -> Self {
        Self {
            level: raw,
            exact: true,
        }
    }

    /// The raw level (0 to 255) that is actually applied to the LEDs.
    ///
    /// For perceived brightness this is the level after the 2.2 curve; for
    /// [`Brightness::exact`] it is the number you gave.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Brightness;
    ///
    /// assert_eq!(Brightness::from_percent(50).output_level(), 56);
    /// assert_eq!(Brightness::exact(56).output_level(), 56);
    /// ```
    #[must_use]
    pub const fn output_level(self) -> u8 {
        if self.exact {
            self.level
        } else {
            GAMMA_2_2.apply(self.level)
        }
    }
}

impl Default for Brightness {
    /// Full brightness.
    fn default() -> Self {
        Self::FULL
    }
}

impl From<f32> for Brightness {
    /// Same as [`Brightness::from_fraction`].
    fn from(fraction: f32) -> Self {
        Self::from_fraction(fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perceived_brightness_maps_through_gamma_2_2() {
        for (f, raw) in [
            (0.1, 2),
            (0.2, 7),
            (0.3, 18),
            (0.5, 56),
            (0.75, 135),
            (1.0, 255),
            (0.0, 0),
        ] {
            assert_eq!(
                Brightness::from_fraction(f).output_level(),
                raw,
                "fraction {f}"
            );
        }
        assert_eq!(Brightness::from_percent(50).output_level(), 56);
        assert_eq!(Brightness::from_percent(250), Brightness::FULL);
        assert_eq!(Brightness::from_fraction(2.0), Brightness::FULL);
        assert_eq!(Brightness::from_fraction(-1.0), Brightness::OFF);
        assert_eq!(Brightness::from_fraction(f32::NAN), Brightness::OFF);
        assert_eq!(Brightness::from(0.5), Brightness::from_fraction(0.5));
    }
    #[test]
    fn float_literal_into_brightness() {
        let _: Brightness = 0.5.into();
    }
    #[test]
    fn equality_compares_how_it_was_made() {
        assert_ne!(Brightness::exact(255), Brightness::FULL);
        assert_ne!(Brightness::exact(0), Brightness::OFF);
        assert_eq!(
            Brightness::exact(255).output_level(),
            Brightness::FULL.output_level()
        );
    }
    #[test]
    fn exact_brightness_has_no_curve() {
        assert_eq!(Brightness::exact(128).output_level(), 128);
        assert_eq!(Brightness::FULL.output_level(), 255);
        assert_eq!(Brightness::OFF.output_level(), 0);
    }
}
