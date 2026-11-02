use ledkit_color::Rgb;

use crate::effects::{along, fade, travel16};
use crate::{Animation, Direction, Frame, Position};

/// A bright dot with a fading tail, running along the strip. Also known as
/// "meteor"; with `.bounce(true)` it's the "scanner" (Larson scanner, Cylon,
/// KITT).
///
/// On a matrix, every row has a comet.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// // A red scanner going back and forth.
/// let scanner = Comet::new().color(Rgb::RED).tail_length(4).bounce(true);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Comet {
    color: Rgb,
    tail: u16,
    speed: u16,
    bounce: bool,
    direction: Direction,
}

impl Comet {
    /// Cyan, with a tail 8 LEDs long, at 30 LEDs a second.
    pub const fn new() -> Self {
        Self {
            color: Rgb::CYAN,
            tail: 8,
            speed: 30,
            bounce: false,
            direction: Direction::Forward,
        }
    }

    /// The color of the head.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.color = color;
        self
    }

    /// How many LEDs the tail fades over. 0 is just the head.
    pub const fn tail_length(mut self, leds: u16) -> Self {
        self.tail = leds;
        self
    }

    /// How fast the head moves, in LEDs per second. 0 stands still.
    pub const fn speed(mut self, leds_per_second: u16) -> Self {
        self.speed = leds_per_second;
        self
    }

    /// True to go back and forth; false (the default) to run off the end and start again.
    pub const fn bounce(mut self, bounce: bool) -> Self {
        self.bounce = bounce;
        self
    }

    /// Which way it starts moving.
    pub const fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
}

impl Default for Comet {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Comet {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let (x, len) = along(at, *frame, self.direction);
        let (x16, len16) = (i64::from(x) * 16, i64::from(len) * 16);
        let tail16 = i64::from(self.tail.max(1)) * 16;
        let travel = i64::try_from(travel16(frame.millis, self.speed)).unwrap_or(i64::MAX);
        // How far behind the head this LED is, in 1/16ths of an LED.
        let behind = if self.bounce {
            let range = (len16 - 16).max(0);
            if range == 0 {
                x16
            } else {
                let p = travel % (2 * range);
                if p < range {
                    p - x16
                } else {
                    x16 - (2 * range - p)
                }
            }
        } else {
            (travel % (len16 + tail16)) - x16
        };
        if (0..tail16).contains(&behind) {
            self.color.scale(fade(behind, tail16))
        } else {
            Rgb::BLACK
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{brightness, exercise, strip_at};

    fn brightest(comet: &Comet, millis: u32) -> u16 {
        (0..10)
            .max_by_key(|&x| brightness(strip_at(comet, x, 10, millis)))
            .unwrap()
    }

    #[test]
    fn head_then_fading_tail() {
        let comet = Comet::new();
        assert_eq!(strip_at(&comet, 0, 10, 0), Rgb::CYAN);
        assert_eq!(strip_at(&comet, 1, 10, 0), Rgb::BLACK);
        // At 30 LEDs a second, after 200 ms the head is at LED 6.
        assert_eq!(strip_at(&comet, 6, 10, 200), Rgb::CYAN);
        assert_eq!(strip_at(&comet, 7, 10, 200), Rgb::BLACK);
        let tail: [u32; 7] =
            core::array::from_fn(|i| brightness(strip_at(&comet, 6 - i as u16, 10, 200)));
        assert!(tail.windows(2).all(|w| w[0] > w[1]), "{tail:?}");
    }

    #[test]
    fn bounce_comes_back() {
        let comet = Comet::new().bounce(true);
        // 10 LEDs: 9 LEDs out and 9 back is 600 ms at 30 LEDs a second.
        assert_eq!(brightest(&comet, 100), 3);
        assert_eq!(brightest(&comet, 500), 3);
        // On the way back, the tail is on the far side of the head.
        assert_eq!(strip_at(&comet, 4, 10, 500), Rgb::CYAN.scale(224));
        assert_eq!(strip_at(&comet, 2, 10, 500), Rgb::BLACK);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Comet::new());
        exercise(&Comet::new().bounce(true));
        exercise(&Comet::new().tail_length(0).speed(0));
        exercise(&Comet::new().speed(u16::MAX).bounce(true));
    }
}
