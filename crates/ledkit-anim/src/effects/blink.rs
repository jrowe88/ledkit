use core::time::Duration;

use ledkit_color::Rgb;

use crate::time::millis;
use crate::{Animation, Frame, Position};

/// Switches between two colors on a timer. Also known as "strobe" or "flash".
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// // A quick red flash: on for a fifth of every half second.
/// let alarm = Blink::new()
///     .color(Rgb::RED)
///     .period(Duration::from_millis(500))
///     .duty(20);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Blink {
    on: Rgb,
    off: Rgb,
    period: u32,
    duty: u8,
}

impl Blink {
    /// White, on for half of every second.
    pub const fn new() -> Self {
        Self {
            on: Rgb::WHITE,
            off: Rgb::BLACK,
            period: 1000,
            duty: 50,
        }
    }

    /// The "on" color.
    pub const fn color(mut self, color: Rgb) -> Self {
        self.on = color;
        self
    }

    /// The "off" color. Black unless you set it.
    pub const fn off_color(mut self, color: Rgb) -> Self {
        self.off = color;
        self
    }

    /// How long one on-and-off takes. A period of 0 stays on.
    pub const fn period(mut self, period: Duration) -> Self {
        self.period = millis(period);
        self
    }

    /// The percentage of each period that is "on", from 0 to 100.
    pub const fn duty(mut self, percent: u8) -> Self {
        self.duty = if percent > 100 { 100 } else { percent };
        self
    }
}

impl Default for Blink {
    fn default() -> Self {
        Self::new()
    }
}

impl Animation for Blink {
    fn color_at(&self, _at: Position, frame: &Frame) -> Rgb {
        if self.period == 0 {
            return self.on;
        }
        let into = u64::from(frame.millis % self.period) * 100;
        if into < u64::from(self.period) * u64::from(self.duty) {
            self.on
        } else {
            self.off
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};

    #[test]
    fn on_then_off() {
        let blink = Blink::new();
        assert_eq!(strip_at(&blink, 0, 5, 0), Rgb::WHITE);
        assert_eq!(strip_at(&blink, 0, 5, 600), Rgb::BLACK);
        assert_eq!(strip_at(&blink, 0, 5, 1100), Rgb::WHITE);
    }

    #[test]
    fn zero_duty_is_always_off() {
        assert_eq!(strip_at(&Blink::new().duty(0), 0, 5, 0), Rgb::BLACK);
    }

    #[test]
    fn zero_settings_do_not_panic() {
        exercise(&Blink::new());
        exercise(&Blink::new().period(Duration::ZERO).duty(0));
        let steady = Blink::new().period(Duration::ZERO);
        assert_eq!(strip_at(&steady, 0, 5, 700), Rgb::WHITE);
    }
}
