use core::time::Duration;

use ledkit_color::{palettes, Rgb};

use crate::effects::color_setters;
use crate::effects::geometry::polar;
use crate::time::millis;
use crate::{math, Animation, Colors, Direction, Frame, Position};

/// Colors spiralling round the middle of a matrix. Also known as "spiral".
///
/// `tightness` says how tightly the spiral winds: 0 gives straight spokes,
/// higher numbers wind tighter. On a strip it shows as colors moving out from
/// the middle.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let hypnotic = Swirl::new()
///     .tightness(64)
///     .arms(2)
///     .turn_time(Duration::from_secs(3));
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Swirl {
    colors: Colors,
    tightness: u8,
    arms: u8,
    turn: u32,
    direction: Direction,
    center: Option<(u16, u16)>,
}

impl Swirl {
    /// A one-armed rainbow spiral, turning clockwise once every 4 seconds.
    pub const fn new() -> Self {
        Self {
            colors: Colors::Map(&palettes::RAINBOW),
            tightness: 32,
            arms: 1,
            turn: 4000,
            direction: Direction::Forward,
            center: None,
        }
    }

    color_setters!();

    /// How tightly the spiral winds: 0 is straight spokes, 255 is very tight.
    pub const fn tightness(mut self, tightness: u8) -> Self {
        self.tightness = tightness;
        self
    }

    /// How many times the colors repeat going once round.
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

    /// The LED to spiral around. The default is the middle.
    pub const fn center(mut self, x: u16, y: u16) -> Self {
        self.center = Some((x, y));
        self
    }
}

impl Default for Swirl {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Swirl {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let p = polar(at, *frame, self.center);
        let around = (u32::from(p.angle >> 8) * u32::from(self.arms)) as u8;
        // `tightness` is how far the colors move per LED out from the center.
        let out = (u64::from(p.distance) * u64::from(self.tightness) / 16) as u8;
        let turn = math::phase(frame.millis, self.turn);
        let phase = around.wrapping_add(out);
        let phase = match self.direction {
            Direction::Forward => phase.wrapping_sub(turn),
            Direction::Backward => phase.wrapping_add(turn),
        };
        self.colors.scrolled(phase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, grid_at};

    #[test]
    fn zero_tightness_is_straight_spokes() {
        let swirl = Swirl::new().tightness(0);
        // On a 9x9 the middle is LED (4, 4); these are on the same spoke.
        assert_eq!(
            grid_at(&swirl, (5, 4), (9, 9), 0),
            grid_at(&swirl, (8, 4), (9, 9), 0)
        );
        assert_eq!(
            grid_at(&swirl, (4, 6), (9, 9), 0),
            grid_at(&swirl, (4, 8), (9, 9), 0)
        );
    }

    #[test]
    fn tight_spirals_change_along_a_spoke() {
        let swirl = Swirl::new().tightness(64);
        assert_ne!(
            grid_at(&swirl, (5, 4), (9, 9), 0),
            grid_at(&swirl, (8, 4), (9, 9), 0)
        );
    }

    #[test]
    fn turns() {
        let swirl = Swirl::new();
        assert_ne!(
            grid_at(&swirl, (8, 4), (9, 9), 0),
            grid_at(&swirl, (8, 4), (9, 9), 1000)
        );
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Swirl::new());
        exercise(&Swirl::new().tightness(255).arms(255).center(1000, 1000));
        exercise(&Swirl::new().arms(0).turn_time(Duration::ZERO));
    }
}
