//! Conversions to and from the `rgb` crate's pixel types (feature `rgb`).
//!
//! `smart-leds-trait` drivers take `rgb::RGB8`, so with these conversions a
//! ledkit [`Rgb`] can be handed straight to them. The `smart-leds` feature
//! adds the same conversions for `smart-leds-trait`'s RGBW type.

use crate::Rgb;
#[cfg(feature = "smart-leds")]
use crate::Rgbw;

/// Converts an `rgb` crate color into a ledkit color.
impl From<rgb::RGB8> for Rgb {
    fn from(color: rgb::RGB8) -> Self {
        Self::new(color.r, color.g, color.b)
    }
}

/// Converts a ledkit color into an `rgb` crate color, ready for any
/// `smart-leds-trait` driver.
impl From<Rgb> for rgb::RGB8 {
    fn from(color: Rgb) -> Self {
        Self::new(color.r, color.g, color.b)
    }
}

/// Converts a `smart-leds-trait` RGBW color into a ledkit [`Rgbw`].
#[cfg(feature = "smart-leds")]
impl From<smart_leds_trait::RGBW<u8>> for Rgbw {
    fn from(color: smart_leds_trait::RGBW<u8>) -> Self {
        Self::new(color.r, color.g, color.b, color.a.0)
    }
}

/// Converts a ledkit [`Rgbw`] into a `smart-leds-trait` RGBW color, ready for
/// any RGBW driver.
#[cfg(feature = "smart-leds")]
impl From<Rgbw> for smart_leds_trait::RGBW<u8> {
    fn from(color: Rgbw) -> Self {
        Self {
            r: color.r,
            g: color.g,
            b: color.b,
            a: smart_leds_trait::White(color.w),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_rgb8() {
        let c = Rgb::new(1, 2, 3);
        let theirs: rgb::RGB8 = c.into();
        assert_eq!((theirs.r, theirs.g, theirs.b), (1, 2, 3));
        assert_eq!(Rgb::from(theirs), c);
    }

    #[cfg(feature = "smart-leds")]
    #[test]
    fn round_trips_smart_leds_rgbw() {
        let c = Rgbw::new(1, 2, 3, 4);
        let theirs: smart_leds_trait::RGBW<u8> = c.into();
        assert_eq!((theirs.r, theirs.g, theirs.b, theirs.a.0), (1, 2, 3, 4));
        assert_eq!(Rgbw::from(theirs), c);
    }
}
