//! [`Then`]: one animation, then another.

use ledkit_color::Rgb;

use crate::math::scale_u8;
use crate::{Animation, Frame, Position, Transition};

/// One animation until a set time, then a transition into another. Made by
/// [`AnimationExt::then`](crate::AnimationExt::then) or
/// [`AnimationExt::crossfade_to`](crate::AnimationExt::crossfade_to).
///
/// The second animation starts its own clock at 0 when the transition begins,
/// so it always starts from its beginning.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Then<A, B> {
    pub(crate) first: A,
    pub(crate) next: B,
    pub(crate) at: u32,
    pub(crate) transition: Transition,
}

impl<A: Animation, B: Animation> Animation for Then<A, B> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        if frame.millis < self.at {
            return self.first.color_at(at, frame);
        }
        let into = frame.millis - self.at;
        let next = self.next.color_at(at, &frame.at_millis(into));
        let length = self.transition.length_millis();
        if into >= length {
            return next;
        }
        let first = self.first.color_at(at, frame);
        self.transition
            .mix(first, next, scale_u8(into, length), at, frame)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use core::time::Duration;

    use super::*;
    use crate::effects::test_support::{exercise, strip_at};
    use crate::{AnimationExt, Solid};

    /// Shows the time it is given, as a color.
    pub(crate) struct Clock;

    impl Animation for Clock {
        fn color_at(&self, _at: Position, frame: &Frame) -> Rgb {
            let t = frame.millis;
            Rgb::new((t >> 16) as u8, (t >> 8) as u8, t as u8)
        }
    }

    pub(crate) fn time_shown(color: Rgb) -> u32 {
        (u32::from(color.r) << 16) | (u32::from(color.g) << 8) | u32::from(color.b)
    }

    #[test]
    fn first_then_next_on_its_own_clock() {
        let show =
            Solid::new()
                .color(Rgb::RED)
                .then(Clock, Duration::from_secs(1), Transition::cut());
        assert_eq!(strip_at(&show, 0, 5, 999), Rgb::RED);
        assert_eq!(time_shown(strip_at(&show, 0, 5, 1000)), 0);
        assert_eq!(time_shown(strip_at(&show, 0, 5, 1500)), 500);
    }

    #[test]
    fn crossfade_blends_then_finishes() {
        let show = Solid::new().color(Rgb::RED).crossfade_to(
            Solid::new().color(Rgb::BLUE),
            Duration::from_secs(1),
            Duration::from_secs(1),
        );
        let middle = strip_at(&show, 0, 5, 1500);
        assert!(middle.r > 50 && middle.b > 50, "{middle:?}");
        assert_eq!(strip_at(&show, 0, 5, 2000), Rgb::BLUE);
        exercise(&show);
    }
}
