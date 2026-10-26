//! Gamma correction: making LED fades look smooth to your eyes.
//!
//! Your eye does not see brightness in a straight line. An LED driven at
//! half power looks much brighter than "half as bright", so a fade written
//! as plain numbers looks wrong: it seems to jump quickly at the dim end and
//! barely change at the bright end. A gamma table bends the numbers to match
//! your eye. The tables here are built while your program is compiled, so
//! they cost no time and no maths on the microcontroller.

use crate::{Rgb, Rgbw};

/// Raises `base` to a whole-number `power` using only multiplication, so it
/// works in `const fn`.
const fn powu(base: f64, power: u32) -> f64 {
    let mut result = 1.0;
    let mut i = 0;
    while i < power {
        result *= base;
        i += 1;
    }
    result
}

/// A lookup table that turns a brightness (0 to 255) into a gamma-corrected
/// brightness, so fades look even to your eye.
///
/// Use the ready-made [`GAMMA_2_6`] (a good default for LEDs) or
/// [`GAMMA_2_2`], or build your own with [`GammaTable::new`].
///
/// # Examples
///
/// ```
/// use ledkit_color::{Rgb, GAMMA_2_6};
///
/// // Half brightness on the dial becomes much lower LED power...
/// assert_eq!(GAMMA_2_6.apply(128), 42);
/// // ...and it works on whole colors too.
/// assert_eq!(GAMMA_2_6.apply_rgb(Rgb::new(128, 255, 0)), Rgb::new(42, 255, 0));
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GammaTable([u8; 256]);

impl GammaTable {
    /// A table that changes nothing: every brightness maps to itself.
    pub const LINEAR: Self = {
        let mut table = [0u8; 256];
        let mut i = 0;
        while i < 256 {
            table[i] = i as u8;
            i += 1;
        }
        Self(table)
    };

    /// Builds a table for the gamma `numerator / denominator`, for example
    /// `GammaTable::new(13, 5)` is gamma 2.6. Bigger gamma means a stronger
    /// correction (dim levels get dimmer).
    ///
    /// This is a `const fn`, so put it in a `const` and the table is made at
    /// compile time. Keep the numbers small (like 13 and 5).
    ///
    /// # Panics
    ///
    /// Panics if `denominator` is 0 (at compile time, if used in a `const`).
    ///
    /// ```
    /// use ledkit_color::GammaTable;
    ///
    /// const GAMMA_3: GammaTable = GammaTable::new(3, 1);
    /// assert_eq!(GAMMA_3.apply(255), 255);
    /// assert!(GAMMA_3.apply(128) < 128);
    /// ```
    #[allow(clippy::cast_possible_truncation)] // x and y are at most 255
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        assert!(denominator > 0);
        let mut table = [0u8; 256];
        let mut y: u32 = 0;
        let mut x: u32 = 0;
        while x < 256 {
            let target = powu(x as f64 / 255.0, numerator);
            // Move up while the true value is at least y + 1/2 (so it rounds
            // above y). Outputs never decrease, so we resume from the last y.
            while y < 255 && powu((y as f64 + 0.5) / 255.0, denominator) <= target {
                y += 1;
            }
            table[x as usize] = y as u8;
            x += 1;
        }
        Self(table)
    }

    /// Looks up the corrected brightness for one level.
    pub const fn apply(&self, level: u8) -> u8 {
        self.0[level as usize]
    }

    /// Corrects each of red, green and blue.
    pub const fn apply_rgb(&self, color: Rgb) -> Rgb {
        Rgb::new(
            self.apply(color.r),
            self.apply(color.g),
            self.apply(color.b),
        )
    }

    /// Corrects each of red, green, blue and white.
    pub const fn apply_rgbw(&self, color: Rgbw) -> Rgbw {
        Rgbw {
            r: self.apply(color.r),
            g: self.apply(color.g),
            b: self.apply(color.b),
            w: self.apply(color.w),
        }
    }
}

/// Gamma 2.6, the exponent Adafruit's NeoPixel library uses. A good default for LEDs.
pub const GAMMA_2_6: GammaTable = GammaTable::new(13, 5);

/// Gamma 2.2, the exponent of typical screens. Gentler than [`GAMMA_2_6`].
pub const GAMMA_2_2: GammaTable = GammaTable::new(11, 5);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Rgb, Rgbw};

    #[test]
    fn gamma_2_6_known_values() {
        for (x, y) in [
            (0, 0),
            (1, 0),
            (23, 0),
            (24, 1),
            (32, 1),
            (64, 7),
            (128, 42),
            (200, 136),
            (254, 252),
            (255, 255),
        ] {
            assert_eq!(GAMMA_2_6.apply(x), y, "input {x}");
        }
    }
    #[test]
    fn gamma_2_2_midpoint() {
        assert_eq!(GAMMA_2_2.apply(128), 56);
    }
    #[test]
    fn linear_is_identity() {
        for x in 0..=255u8 {
            assert_eq!(GammaTable::LINEAR.apply(x), x);
        }
    }
    #[test]
    fn tables_never_decrease() {
        for t in [GAMMA_2_6, GAMMA_2_2] {
            for x in 1..=255u8 {
                assert!(t.apply(x) >= t.apply(x - 1));
            }
        }
    }
    #[test]
    #[allow(clippy::cast_sign_loss)] // the reference value is always in 0.0..=255.0
    fn matches_float_reference_exactly() {
        for x in 0..=255u8 {
            let expected = (255.0 * (f64::from(x) / 255.0).powf(2.6)).round() as u8;
            assert_eq!(GAMMA_2_6.apply(x), expected, "input {x}");
        }
    }
    #[test]
    fn apply_rgb_per_channel() {
        assert_eq!(
            GAMMA_2_6.apply_rgb(Rgb::new(128, 255, 0)),
            Rgb::new(42, 255, 0)
        );
    }
    #[test]
    fn apply_rgbw_per_channel() {
        assert_eq!(
            GAMMA_2_6.apply_rgbw(Rgbw {
                r: 128,
                g: 255,
                b: 0,
                w: 64
            }),
            Rgbw {
                r: 42,
                g: 255,
                b: 0,
                w: 7
            }
        );
    }
    #[test]
    fn table_is_built_at_compile_time() {
        const T: GammaTable = GammaTable::new(13, 5);
        assert_eq!(T.apply(128), 42);
    }
}
