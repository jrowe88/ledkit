//! [`Colors`]: the colors an effect uses.

use core::fmt;

use ledkit_color::{ColorMap, Rgb};

use crate::math;

/// The colors an effect draws with: one color, a blend between two, or a
/// color map such as a ready-made palette.
///
/// Effects that use colors have setters for each kind, for example
/// `.color(Rgb::RED)`, `.colors_between(Rgb::RED, Rgb::YELLOW)` and
/// `.palette(&palettes::FIRE)`.
///
/// # Example
///
/// ```
/// use ledkit_anim::Colors;
/// use ledkit_color::{palettes, Rgb};
///
/// let one = Colors::One(Rgb::RED);
/// let two = Colors::Two(Rgb::RED, Rgb::YELLOW);
/// let map = Colors::Map(&palettes::OCEAN);
/// assert_eq!(two.at(0), Rgb::RED);
/// assert_eq!(two.at(255), Rgb::YELLOW);
/// ```
#[derive(Copy, Clone)]
pub enum Colors {
    /// Always this color.
    One(Rgb),
    /// A smooth blend from the first color (at 0) to the second (at 255).
    Two(Rgb, Rgb),
    /// A color map, such as one from [`palettes`](ledkit_color::palettes).
    /// A palette you make yourself must be a `const` or a `static`.
    Map(&'static dyn ColorMap),
}

impl Colors {
    /// The color at `position`, 0 to 255.
    ///
    /// ```
    /// use ledkit_anim::Colors;
    /// use ledkit_color::Rgb;
    ///
    /// assert_eq!(Colors::One(Rgb::BLUE).at(99), Rgb::BLUE);
    /// ```
    pub fn at(&self, position: u8) -> Rgb {
        match self {
            Self::One(color) => *color,
            Self::Two(start, end) => start.lerp(*end, position),
            Self::Map(map) => map.color_at(position),
        }
    }

    /// True if the end blends smoothly back into the start (only some color
    /// maps do).
    ///
    /// ```
    /// use ledkit_anim::Colors;
    /// use ledkit_color::{palettes, Rgb};
    ///
    /// assert!(Colors::Map(&palettes::RAINBOW).wraps());
    /// assert!(!Colors::Two(Rgb::RED, Rgb::BLUE).wraps());
    /// ```
    pub fn wraps(&self) -> bool {
        match self {
            Self::One(_) | Self::Two(..) => false,
            Self::Map(map) => map.wraps(),
        }
    }

    /// The color at `phase` for something that scrolls: colors that wrap go
    /// round and round, and colors that don't go there and back, so there is
    /// never a hard jump.
    ///
    /// ```
    /// use ledkit_anim::Colors;
    /// use ledkit_color::Rgb;
    ///
    /// let two = Colors::Two(Rgb::BLACK, Rgb::WHITE);
    /// assert_eq!(two.scrolled(0), Rgb::BLACK);
    /// assert_eq!(two.scrolled(128), Rgb::WHITE); // halfway round: the far end
    /// ```
    pub fn scrolled(&self, phase: u8) -> Rgb {
        if self.wraps() {
            self.at(phase)
        } else {
            self.at(math::triangle(phase))
        }
    }
}

impl From<Rgb> for Colors {
    fn from(color: Rgb) -> Self {
        Self::One(color)
    }
}

impl fmt::Debug for Colors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One(color) => f.debug_tuple("One").field(color).finish(),
            Self::Two(start, end) => f.debug_tuple("Two").field(start).field(end).finish(),
            Self::Map(_) => f.write_str("Map(..)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledkit_color::palettes;

    #[test]
    fn colors_scrolled_ping_pongs() {
        let two = Colors::Two(Rgb::BLACK, Rgb::WHITE);
        assert_eq!(two.scrolled(0), Rgb::BLACK);
        assert!(two.scrolled(128).r >= 254);
        assert!(two.scrolled(255).r <= 2);
    }

    #[test]
    fn wrapping_maps_go_round() {
        let rainbow = Colors::Map(&palettes::RAINBOW);
        assert_eq!(rainbow.scrolled(200), palettes::RAINBOW.color_at(200));
    }
}
