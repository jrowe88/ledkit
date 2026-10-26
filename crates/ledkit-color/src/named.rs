//! Named colors, such as [`Rgb::RED`].

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

    // Values from Adafruit CircuitPython LED_Animation (MIT); see NOTICE.
    /// Yellow, tuned to look yellow on LEDs.
    pub const YELLOW: Self = Self::new(255, 150, 0);
    /// Orange.
    pub const ORANGE: Self = Self::new(255, 40, 0);
    /// Cyan.
    pub const CYAN: Self = Self::new(0, 255, 255);
    /// Teal.
    pub const TEAL: Self = Self::new(0, 255, 120);
    /// Purple.
    pub const PURPLE: Self = Self::new(180, 0, 255);
    /// Magenta.
    pub const MAGENTA: Self = Self::new(255, 0, 20);
    /// Pink.
    pub const PINK: Self = Self::new(242, 90, 255);
    /// Gold.
    pub const GOLD: Self = Self::new(255, 222, 30);
    /// Aqua.
    pub const AQUA: Self = Self::new(50, 255, 255);
    /// Jade.
    pub const JADE: Self = Self::new(0, 255, 40);
    /// Amber.
    pub const AMBER: Self = Self::new(255, 100, 0);
    /// Old lace, a very soft off-white.
    pub const OLD_LACE: Self = Self::new(253, 245, 230);

    /// Warm white, like a 2700 K incandescent bulb.
    pub const WARM_WHITE: Self = Self::new(255, 167, 87);
    /// Cool white, like 6500 K daylight.
    pub const COOL_WHITE: Self = Self::new(255, 254, 250);
}
