use core::time::Duration;

use ledkit_color::Rgb;

use crate::effects::geometry::polar;
use crate::time::millis;
use crate::{math, Animation, Direction, Frame, Position};

/// Arms spinning round the middle of a matrix, like a pinwheel. Also known as
/// "rotating arms"; with 4 arms (the default) it's a rotating X or cross.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// // A slow orange propeller with three blades.
/// let propeller = Pinwheel::new()
///     .color(Rgb::ORANGE)
///     .arms(3)
///     .turn_time(Duration::from_secs(5));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Pinwheel {
    color: Rgb,
    background: Rgb,
    arms: u8,
    turn: u32,
    direction: Direction,
    center: Option<(u16, u16)>,
}

impl Pinwheel {
    /// Four white arms on black, turning clockwise once every 3 seconds.
    pub const fn new() -> Self {
        Self {
            color: Rgb::WHITE,
            background: Rgb::BLACK,
            arms: 4,
            turn: 3000,
            direction: Direction::Forward,
            center: None,
        }
    }

    /// The color of the arms.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// The color between the arms. Black unless you set it.
    pub const fn background(mut self, color: Rgb) -> Self {
        self.background = color;
        self
    }

    /// How many arms. 0 shows only the background.
    pub const fn arms(mut self, arms: u8) -> Self {
        self.arms = arms;
        self
    }

    /// How long one full turn takes. 0 stands still.
    pub const fn turn_time(mut self, turn: Duration) -> Self {
        self.turn = millis(turn);
        self
    }

    /// Which way it turns: `Forward` is clockwise.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    /// The LED to spin around. The default is the middle.
    pub const fn center(mut self, x: u16, y: u16) -> Self {
        self.center = Some((x, y));
        self
    }
}

impl Default for Pinwheel {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Pinwheel {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        if self.arms == 0 {
            return self.background;
        }
        let p = polar(at, *frame, self.center);
        let turn = math::phase16(frame.millis, self.turn);
        let angle = match self.direction {
            Direction::Forward => p.angle.wrapping_sub(turn),
            Direction::Backward => p.angle.wrapping_add(turn),
        };
        // Where this LED is within one arm and the gap after it.
        let within = (u32::from(angle) * u32::from(self.arms)) as u16;
        // A sine wave, steepened so the arms have crisp but not jagged edges.
        let wave = i32::from(math::sine((within >> 8) as u8));
        let level = u8::try_from(((wave - 128) * 4 + 128).clamp(0, 255)).unwrap_or(255);
        self.background.lerp(self.color, level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, grid_at};

    #[test]
    fn four_arms_look_the_same_turned_a_quarter() {
        let pinwheel = Pinwheel::new();
        for t in [0, 400, 1234] {
            for y in 0..8 {
                for x in 0..8 {
                    assert_eq!(
                        grid_at(&pinwheel, (x, y), (8, 8), t),
                        grid_at(&pinwheel, (7 - y, x), (8, 8), t),
                        "({x}, {y}) at {t}"
                    );
                }
            }
        }
    }

    #[test]
    fn has_arms_and_gaps() {
        let pinwheel = Pinwheel::new();
        let colors: [Rgb; 64] = core::array::from_fn(|i| {
            grid_at(&pinwheel, ((i % 8) as u16, (i / 8) as u16), (8, 8), 0)
        });
        assert!(colors.contains(&Rgb::WHITE));
        assert!(colors.contains(&Rgb::BLACK));
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Pinwheel::new());
        exercise(&Pinwheel::new().arms(0).turn_time(Duration::ZERO));
        exercise(&Pinwheel::new().arms(255).center(0, 0));
    }
}
