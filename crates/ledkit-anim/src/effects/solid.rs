use ledkit_color::Rgb;

use crate::{Animation, Frame, Position};

/// One color everywhere, all the time. Also known as "static".
///
/// Handy as a background layer, or as a step in a playlist.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// let warm = Solid::new().color(Rgb::WARM_WHITE);
/// let mut leds = [Rgb::BLACK; 10];
/// warm.render(&mut leds, Time::from_millis(0));
/// assert_eq!(leds[9], Rgb::WARM_WHITE);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Solid {
    color: Rgb,
}

impl Solid {
    /// White everywhere.
    pub const fn new() -> Self {
        Self { color: Rgb::WHITE }
    }

    /// The color.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }
}

impl Default for Solid {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Solid {
    fn color_at(&self, _at: Position, _frame: &Frame) -> Rgb {
        self.color
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn same_color_everywhere() {
        let solid = Solid::new().color(Rgb::RED);
        assert_eq!(strip_at(&solid, 0, 10, 0), Rgb::RED);
        assert_eq!(strip_at(&solid, 9, 10, 5000), Rgb::RED);
        exercise(&solid);
    }
}
