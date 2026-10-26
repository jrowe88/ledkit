//! Hue-based colors: [`Hue`] and [`Hsv`].

use crate::Rgb;

/// Turns a position on the color wheel into a fully bright, fully saturated color.
///
/// Integer only, so it is fast on chips without a floating point unit.
const fn hue_to_rgb(hue: u16) -> Rgb {
    let scaled = hue as u32 * 6;
    let sector = scaled >> 16;
    let rising = ((scaled & 0xFFFF) >> 8) as u8;
    let falling = 255 - rising;
    match sector {
        0 => Rgb::new(255, rising, 0),
        1 => Rgb::new(falling, 255, 0),
        2 => Rgb::new(0, 255, rising),
        3 => Rgb::new(0, falling, 255),
        4 => Rgb::new(rising, 0, 255),
        _ => Rgb::new(255, 0, falling),
    }
}

/// Moves one channel toward white by `saturation` (255 keeps it, 0 gives 255).
const fn saturate_channel(c: u8, saturation: u8) -> u8 {
    (255 - ((255 - c as u32) * saturation as u32 + 127) / 255) as u8
}

/// Dims one channel by `value` (255 keeps it, 0 gives 0).
const fn dim_channel(c: u8, value: u8) -> u8 {
    ((c as u32 * value as u32 + 127) / 255) as u8
}

/// Converts hue, saturation and value to RGB.
const fn hsv_to_rgb(hsv: Hsv) -> Rgb {
    let full = hue_to_rgb(hsv.hue.0);
    let s = hsv.saturation;
    let v = hsv.value;
    Rgb::new(
        dim_channel(saturate_channel(full.r, s), v),
        dim_channel(saturate_channel(full.g, s), v),
        dim_channel(saturate_channel(full.b, s), v),
    )
}

/// Maps a float to a hue step, wrapping around a full turn of `modulus`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // value is wrapped into 0..=65536 first
fn wrap_to_hue(x: f32, modulus: f32) -> Hue {
    let mut wrapped = libm::fmodf(x, modulus);
    if wrapped < 0.0 {
        wrapped += modulus;
    }
    // Rounding can land exactly on 65536; the mask turns that into 0.
    Hue((((wrapped / modulus) * 65536.0 + 0.5) as u32 & 0xFFFF) as u16)
}

/// A spot on the color wheel. A full turn is 65536 steps.
///
/// Red is 0, yellow, green, cyan, blue and magenta follow every 60 degrees,
/// and going past the end wraps back to red. Turn a hue into a color with
/// [`Rgb::from_hue`] or through [`Hsv`].
///
/// # Examples
///
/// ```
/// use ledkit_color::{Hue, Rgb};
///
/// let orange = Rgb::from_hue(Hue::from_degrees(30.0));
/// assert_eq!(orange, Rgb::new(255, 127, 0));
///
/// // Rotate a rainbow by adding to the hue.
/// assert_eq!(Hue::RED.shifted(Hue::GREEN.0), Hue::GREEN);
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Hue(pub u16);

impl Hue {
    /// Red, at 0 degrees.
    pub const RED: Self = Self(0);
    /// Yellow, at 60 degrees.
    pub const YELLOW: Self = Self(10923);
    /// Green, at 120 degrees.
    pub const GREEN: Self = Self(21845);
    /// Cyan, at 180 degrees.
    pub const CYAN: Self = Self(32768);
    /// Blue, at 240 degrees.
    pub const BLUE: Self = Self(43691);
    /// Magenta, at 300 degrees.
    pub const MAGENTA: Self = Self(54613);

    /// Makes a hue from degrees around the color wheel.
    ///
    /// Any value works: 360 is the same as 0, and negative values count
    /// backwards, so -60 is the same as 300.
    ///
    /// ```
    /// use ledkit_color::Hue;
    /// assert_eq!(Hue::from_degrees(-60.0), Hue::MAGENTA);
    /// ```
    pub fn from_degrees(degrees: f32) -> Self {
        wrap_to_hue(degrees, 360.0)
    }

    /// Makes a hue from a fraction of a turn: 0.0 to 1.0 is a full turn, and
    /// larger or negative values wrap around.
    ///
    /// ```
    /// use ledkit_color::Hue;
    /// assert_eq!(Hue::from_fraction(1.5), Hue::CYAN);
    /// ```
    pub fn from_fraction(fraction: f32) -> Self {
        wrap_to_hue(fraction, 1.0)
    }

    /// Moves the hue forward by `steps`, wrapping around. Handy for rotating a rainbow.
    ///
    /// ```
    /// use ledkit_color::Hue;
    /// assert_eq!(Hue(65000).shifted(1000), Hue(464));
    /// ```
    pub const fn shifted(self, steps: u16) -> Self {
        Self(self.0.wrapping_add(steps))
    }
}

/// A color described by hue, saturation and value, which is often the
/// easiest way to pick a color you like.
///
/// `saturation` runs from 0 (white or gray) to 255 (vivid), and `value` from
/// 0 (black) to 255 (full brightness).
///
/// # Examples
///
/// ```
/// use ledkit_color::{Hsv, Hue, Rgb};
///
/// let half_bright_red = Rgb::from(Hsv::new(Hue::RED, 255, 128));
/// assert_eq!(half_bright_red, Rgb::new(128, 0, 0));
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Hsv {
    /// Where on the color wheel the color sits.
    pub hue: Hue,
    /// How vivid the color is, 0 (white) to 255 (full color).
    pub saturation: u8,
    /// How bright the color is, 0 (black) to 255 (full brightness).
    pub value: u8,
}

impl Hsv {
    /// Makes a color from hue, saturation and value.
    pub const fn new(hue: Hue, saturation: u8, value: u8) -> Self {
        Self {
            hue,
            saturation,
            value,
        }
    }
}

impl From<Hsv> for Rgb {
    fn from(hsv: Hsv) -> Self {
        hsv_to_rgb(hsv)
    }
}

impl Rgb {
    /// Makes the fully bright, fully vivid color at `hue`.
    ///
    /// ```
    /// use ledkit_color::{Hue, Rgb};
    /// assert_eq!(Rgb::from_hue(Hue::BLUE), Rgb::new(0, 0, 255));
    /// ```
    pub const fn from_hue(hue: Hue) -> Self {
        hsv_to_rgb(Hsv::new(hue, 255, 255))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primaries_are_exact() {
        for (deg, rgb) in [
            (0.0, (255, 0, 0)),
            (60.0, (255, 255, 0)),
            (120.0, (0, 255, 0)),
            (180.0, (0, 255, 255)),
            (240.0, (0, 0, 255)),
            (300.0, (255, 0, 255)),
        ] {
            let c = Rgb::from_hue(Hue::from_degrees(deg));
            assert_eq!((c.r, c.g, c.b), rgb, "at {deg} degrees");
        }
    }
    #[test]
    fn degrees_wrap_around() {
        assert_eq!(Hue::from_degrees(360.0), Hue::RED);
        assert_eq!(Hue::from_degrees(-60.0), Hue::from_degrees(300.0));
        assert_eq!(Hue::from_degrees(780.0), Hue::from_degrees(60.0));
        assert_eq!(Hue::from_degrees(-1e-7), Hue::RED);
    }
    #[test]
    fn fraction_wraps() {
        assert_eq!(Hue::from_fraction(1.25), Hue::from_fraction(0.25));
    }
    #[test]
    fn shifted_wraps() {
        assert_eq!(Hue(65000).shifted(1000), Hue(464));
    }
    #[test]
    fn zero_saturation_is_white() {
        assert_eq!(Rgb::from(Hsv::new(Hue::BLUE, 0, 255)), Rgb::WHITE);
    }
    #[test]
    fn zero_value_is_black() {
        assert_eq!(Rgb::from(Hsv::new(Hue::GREEN, 255, 0)), Rgb::BLACK);
    }
    #[test]
    fn half_value_red() {
        assert_eq!(Rgb::from(Hsv::new(Hue::RED, 255, 128)), Rgb::new(128, 0, 0));
    }
    #[test]
    fn hue_constants_match_degrees() {
        assert_eq!(Hue::from_degrees(120.0), Hue::GREEN);
        assert_eq!(Hue::from_degrees(240.0), Hue::BLUE);
    }
    #[test]
    fn rainbow_has_no_jumps() {
        // Neighboring hues differ by a few levels per channel: no visible seams, including at the wrap.
        let mut prev = Rgb::from_hue(Hue(65535 - 255));
        for h in (0..=65535u32).step_by(256) {
            let c = Rgb::from_hue(Hue(h as u16));
            for (a, b) in [(prev.r, c.r), (prev.g, c.g), (prev.b, c.b)] {
                assert!(a.abs_diff(b) <= 7);
            }
            prev = c;
        }
    }
}
