//! Named colors, such as [`Rgb::RED`].
//!
//! The basic colors (red, green, blue, yellow, cyan, magenta) match the
//! matching [`Hue`](crate::Hue) constants. LEDs often look too green; use
//! [`correction::TYPICAL_LED_STRIP`](crate::correction::TYPICAL_LED_STRIP)
//! to compensate rather than tuning colors by hand.

use crate::Rgb;

impl Rgb {
    /// Black (all off).
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White (all on).
    pub const WHITE: Self = Self::new(255, 255, 255);
    /// Red.
    pub const RED: Self = Self::new(255, 0, 0);
    /// Green.
    pub const GREEN: Self = Self::new(0, 255, 0);
    /// Blue.
    pub const BLUE: Self = Self::new(0, 0, 255);

    /// Yellow.
    pub const YELLOW: Self = Self::new(255, 255, 0);
    /// Cyan.
    pub const CYAN: Self = Self::new(0, 255, 255);
    /// Magenta.
    pub const MAGENTA: Self = Self::new(255, 0, 255);

    // Values from Adafruit CircuitPython LED_Animation (MIT); see NOTICE.
    /// Orange, tuned to look right on LEDs.
    pub const ORANGE: Self = Self::new(255, 40, 0);
    /// Teal, a blue-green, tuned to look right on LEDs.
    pub const TEAL: Self = Self::new(0, 255, 120);
    /// Purple, tuned to look right on LEDs.
    pub const PURPLE: Self = Self::new(180, 0, 255);
    /// Pink, a light purple-pink, tuned to look right on LEDs.
    pub const PINK: Self = Self::new(242, 90, 255);
    /// Gold, a yellow with a touch of orange, tuned to look right on LEDs.
    pub const GOLD: Self = Self::new(255, 222, 30);
    /// Aqua, a pale cyan, tuned to look right on LEDs.
    pub const AQUA: Self = Self::new(50, 255, 255);
    /// Jade, a green with a hint of blue, tuned to look right on LEDs.
    pub const JADE: Self = Self::new(0, 255, 40);
    /// Amber, a deep orange-yellow, tuned to look right on LEDs.
    pub const AMBER: Self = Self::new(255, 100, 0);
    /// Old lace, a very soft off-white, tuned to look right on LEDs.
    pub const OLD_LACE: Self = Self::new(253, 245, 230);

    /// Warm white, like a 2700 K incandescent bulb.
    pub const WARM_WHITE: Self = Self::new(255, 167, 87);
    /// Cool white, like 6500 K daylight.
    pub const COOL_WHITE: Self = Self::new(255, 254, 250);
}
