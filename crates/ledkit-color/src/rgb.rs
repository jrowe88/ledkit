//! The basic color types: [`Rgb`] and [`Rgbw`].

/// Scales one channel: `factor` 255 leaves it alone, 0 makes it 0.
const fn scale_channel(c: u8, factor: u8) -> u8 {
    ((c as u16 * factor as u16 + 127) / 255) as u8
}

/// Blends one channel from `a` (at `t` = 0) to `b` (at `t` = 255).
const fn lerp_channel(a: u8, b: u8, t: u8) -> u8 {
    ((a as u16 * (255 - t as u16) + b as u16 * t as u16 + 127) / 255) as u8
}

/// A color made of red, green and blue brightness, each from 0 to 255.
///
/// Use it for ordinary RGB LEDs such as WS2812 ("neopixels") and APA102
/// ("dotstars"). There are named colors like [`Rgb::RED`] and
/// [`Rgb::WARM_WHITE`], and you can also write `Color::RED` because
/// [`Color`](crate::Color) is another name for `Rgb`.
///
/// # Examples
///
/// ```
/// use ledkit_color::Rgb;
///
/// let dim_red = Rgb::RED.scale(64);
/// assert_eq!(dim_red, Rgb::new(64, 0, 0));
///
/// let halfway = Rgb::BLACK.lerp(Rgb::WHITE, 128);
/// assert_eq!(halfway, Rgb::new(128, 128, 128));
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    /// Red brightness, 0 to 255.
    pub r: u8,
    /// Green brightness, 0 to 255.
    pub g: u8,
    /// Blue brightness, 0 to 255.
    pub b: u8,
}

impl Rgb {
    /// Makes a color from red, green and blue brightness.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Dims the color. A `factor` of 255 leaves it unchanged and 0 gives black.
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// assert_eq!(Rgb::new(255, 100, 0).scale(128), Rgb::new(128, 50, 0));
    /// ```
    pub const fn scale(self, factor: u8) -> Self {
        Self::new(
            scale_channel(self.r, factor),
            scale_channel(self.g, factor),
            scale_channel(self.b, factor),
        )
    }

    /// Blends toward `target`. At `t` = 0 you get `self`, at 255 you get
    /// `target`, and values in between mix the two.
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// assert_eq!(Rgb::RED.lerp(Rgb::BLUE, 255), Rgb::BLUE);
    /// ```
    pub const fn lerp(self, target: Self, t: u8) -> Self {
        Self::new(
            lerp_channel(self.r, target.r, t),
            lerp_channel(self.g, target.g, t),
            lerp_channel(self.b, target.b, t),
        )
    }
}

/// A color for RGBW LEDs (such as SK6812 RGBW), which have a separate white
/// emitter next to the red, green and blue ones.
///
/// # Examples
///
/// ```
/// use ledkit_color::{Rgb, Rgbw};
///
/// // A pastel orange: the shared white part moves to the white emitter.
/// let c = Rgbw::from_rgb_extracting_white(Rgb::new(255, 200, 100));
/// assert_eq!(c, Rgbw::new(155, 100, 0, 100));
///
/// let dim = c.scale(64);
/// assert_eq!(dim.w, 25);
/// ```
#[must_use]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Rgbw {
    /// Red brightness, 0 to 255.
    pub r: u8,
    /// Green brightness, 0 to 255.
    pub g: u8,
    /// Blue brightness, 0 to 255.
    pub b: u8,
    /// White brightness, 0 to 255.
    pub w: u8,
}

impl Rgbw {
    /// Makes a color from red, green, blue and white brightness.
    pub const fn new(r: u8, g: u8, b: u8, w: u8) -> Self {
        Self { r, g, b, w }
    }

    /// Dims the color. A `factor` of 255 leaves it unchanged and 0 gives black.
    ///
    /// ```
    /// use ledkit_color::Rgbw;
    /// assert_eq!(Rgbw::new(255, 0, 0, 255).scale(0), Rgbw::default());
    /// ```
    pub const fn scale(self, factor: u8) -> Self {
        Self::new(
            scale_channel(self.r, factor),
            scale_channel(self.g, factor),
            scale_channel(self.b, factor),
            scale_channel(self.w, factor),
        )
    }

    /// Blends toward `target`. At `t` = 0 you get `self`, at 255 you get
    /// `target`, and values in between mix the two.
    ///
    /// ```
    /// use ledkit_color::Rgbw;
    /// let a = Rgbw::new(0, 0, 0, 0);
    /// assert_eq!(a.lerp(Rgbw::new(0, 0, 0, 255), 255), Rgbw::new(0, 0, 0, 255));
    /// ```
    pub const fn lerp(self, target: Self, t: u8) -> Self {
        Self::new(
            lerp_channel(self.r, target.r, t),
            lerp_channel(self.g, target.g, t),
            lerp_channel(self.b, target.b, t),
            lerp_channel(self.w, target.w, t),
        )
    }

    /// Converts an [`Rgb`] color, moving the white part to the white emitter.
    ///
    /// The white amount is the smallest of red, green and blue, and it is
    /// subtracted from each of them. Saturated colors get no white at all.
    ///
    /// ```
    /// use ledkit_color::{Rgb, Rgbw};
    /// assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::WHITE), Rgbw::new(0, 0, 0, 255));
    /// ```
    pub const fn from_rgb_extracting_white(rgb: Rgb) -> Self {
        let mut w = rgb.r;
        if rgb.g < w {
            w = rgb.g;
        }
        if rgb.b < w {
            w = rgb.b;
        }
        Self::new(rgb.r - w, rgb.g - w, rgb.b - w, w)
    }
}

impl From<Rgb> for Rgbw {
    /// Keeps red, green and blue as they are and sets white to 0.
    fn from(rgb: Rgb) -> Self {
        Self::new(rgb.r, rgb.g, rgb.b, 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Rgb, Rgbw};
    #[test] fn scale_255_is_identity() { let c = Rgb::new(10, 128, 255); assert_eq!(c.scale(255), c); }
    #[test] fn scale_0_is_black() { assert_eq!(Rgb::new(10, 128, 255).scale(0), Rgb::BLACK); }
    #[test] fn scale_half() { assert_eq!(Rgb::new(255, 100, 0).scale(128), Rgb::new(128, 50, 0)); }
    #[test] fn lerp_endpoints_exact() {
        let (a, b) = (Rgb::new(10, 20, 30), Rgb::new(200, 100, 0));
        assert_eq!(a.lerp(b, 0), a);
        assert_eq!(a.lerp(b, 255), b);
    }
    #[test] fn lerp_midpoint() { assert_eq!(Rgb::BLACK.lerp(Rgb::WHITE, 128), Rgb::new(128, 128, 128)); }
    #[test] fn extract_white_from_pure_white() {
        assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::WHITE), Rgbw::new(0, 0, 0, 255));
    }
    #[test] fn extract_white_from_saturated_color_is_zero() {
        assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::new(255, 0, 40)), Rgbw::new(255, 0, 40, 0));
    }
    #[test] fn extract_white_from_pastel() {
        assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::new(255, 200, 100)), Rgbw::new(155, 100, 0, 100));
    }
    #[test] fn extract_white_from_black() { assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::BLACK), Rgbw::default()); }
    #[test] fn rgb_into_rgbw_has_no_white() { assert_eq!(Rgbw::from(Rgb::new(1, 2, 3)), Rgbw::new(1, 2, 3, 0)); }
    #[test] fn rgbw_scale_and_lerp_endpoints() {
        let w = Rgbw::new(10, 20, 30, 40);
        assert_eq!(w.scale(255), w);
        assert_eq!(w.lerp(Rgbw::new(0, 0, 0, 255), 255), Rgbw::new(0, 0, 0, 255));
    }
}
