//! Conversions to and from the `rgb` crate's pixel types (feature `rgb`).
//!
//! `smart-leds-trait` drivers take `rgb::RGB8`, so with these conversions a
//! ledkit [`Rgb`] can be handed straight to them.

use crate::{Rgb, Rgbw};

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

/// Converts an `rgb` crate color with alpha into an RGBW color. The alpha
/// channel becomes the white channel.
///
/// `smart-leds-trait` has its own RGBW type; interop with it arrives in
/// `ledkit-core` (milestone 2).
impl From<rgb::RGBA8> for Rgbw {
    fn from(color: rgb::RGBA8) -> Self {
        Self::new(color.r, color.g, color.b, color.a)
    }
}

/// Converts an RGBW color into an `rgb` crate color with alpha. The white
/// channel becomes the alpha channel.
impl From<Rgbw> for rgb::RGBA8 {
    fn from(color: Rgbw) -> Self {
        Self::new(color.r, color.g, color.b, color.w)
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

    #[test]
    fn rgba8_alpha_is_white() {
        assert_eq!(
            Rgbw::from(rgb::RGBA8::new(1, 2, 3, 4)),
            Rgbw::new(1, 2, 3, 4)
        );
    }
}
