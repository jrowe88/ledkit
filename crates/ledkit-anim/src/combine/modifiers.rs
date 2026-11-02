//! Small changes to one animation: reverse, mirror, speed, start time, dim.

use ledkit_color::Rgb;

use crate::{Animation, Frame, Position};

/// An animation run backwards along the strip. Made by
/// [`AnimationExt::reversed`](crate::AnimationExt::reversed).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Reversed<A> {
    pub(crate) inner: A,
}

impl<A: Animation> Animation for Reversed<A> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let last_index = u16::try_from(frame.len().saturating_sub(1)).unwrap_or(u16::MAX);
        let flipped = Position {
            x: frame.width.saturating_sub(1).saturating_sub(at.x),
            y: at.y,
            index: last_index.saturating_sub(at.index),
        };
        self.inner.color_at(flipped, frame)
    }
}

/// An animation shown on the first half and mirrored onto the second. Made by
/// [`AnimationExt::mirrored`](crate::AnimationExt::mirrored).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Mirrored<A> {
    pub(crate) inner: A,
}

impl<A: Animation> Animation for Mirrored<A> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let half = frame.width.div_ceil(2);
        let x = if at.x < half {
            at.x
        } else {
            frame.width.saturating_sub(1).saturating_sub(at.x)
        };
        // On a strip the index is the same as x; on a matrix, rows are mirrored.
        let index = if frame.height <= 1 { x } else { at.index };
        let inner_frame = Frame {
            width: half,
            ..*frame
        };
        self.inner
            .color_at(Position { x, y: at.y, index }, &inner_frame)
    }
}

/// An animation sped up or slowed down. Made by
/// [`AnimationExt::at_speed`](crate::AnimationExt::at_speed).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct AtSpeed<A> {
    pub(crate) inner: A,
    pub(crate) percent: u16,
}

impl<A: Animation> Animation for AtSpeed<A> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let millis = (u64::from(frame.millis) * u64::from(self.percent) / 100) as u32;
        self.inner.color_at(at, &frame.at_millis(millis))
    }
}

/// An animation that starts later. Made by
/// [`AnimationExt::starting_at`](crate::AnimationExt::starting_at).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct StartingAt<A> {
    pub(crate) inner: A,
    pub(crate) start: u32,
}

impl<A: Animation> Animation for StartingAt<A> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let millis = frame.millis.saturating_sub(self.start);
        self.inner.color_at(at, &frame.at_millis(millis))
    }
}

/// An animation made dimmer. Made by
/// [`AnimationExt::dimmed`](crate::AnimationExt::dimmed).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Dimmed<A> {
    pub(crate) inner: A,
    pub(crate) level: u8,
}

impl<A: Animation> Animation for Dimmed<A> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        self.inner.color_at(at, frame).scale(self.level)
    }
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use crate::effects::test_support::{exercise, strip_at};
    use crate::{AnimationExt, Rainbow};

    use super::*;

    #[test]
    fn reversed_runs_from_the_far_end() {
        let plain = Rainbow::new();
        let reversed = Rainbow::new().reversed();
        assert_eq!(
            strip_at(&reversed, 0, 10, 300),
            strip_at(&plain, 9, 10, 300)
        );
        exercise(&reversed);
    }

    #[test]
    fn mirrored_halves_match() {
        let mirrored = Rainbow::new().mirrored();
        assert_eq!(strip_at(&mirrored, 2, 10, 0), strip_at(&mirrored, 7, 10, 0));
        assert_eq!(strip_at(&mirrored, 4, 9, 0), strip_at(&mirrored, 4, 9, 0));
        assert_eq!(strip_at(&mirrored, 3, 9, 0), strip_at(&mirrored, 5, 9, 0));
        exercise(&mirrored);
    }

    #[test]
    fn at_speed_scales_time() {
        let fast = Rainbow::new().at_speed(200);
        assert_eq!(
            strip_at(&fast, 3, 10, 700),
            strip_at(&Rainbow::new(), 3, 10, 1400)
        );
        exercise(&Rainbow::new().at_speed(1000));
        exercise(&Rainbow::new().at_speed(0));
    }

    #[test]
    fn starting_at_waits() {
        let late = Rainbow::new().starting_at(Duration::from_secs(1));
        let first = strip_at(&Rainbow::new(), 3, 10, 0);
        assert_eq!(strip_at(&late, 3, 10, 0), first);
        assert_eq!(strip_at(&late, 3, 10, 1000), first);
        exercise(&late);
    }

    #[test]
    fn dimmed_to_zero_is_black() {
        assert_eq!(strip_at(&Rainbow::new().dimmed(0), 3, 10, 0), Rgb::BLACK);
        exercise(&Rainbow::new().dimmed(128));
    }
}
