//! [`Segments`]: different animations on different parts of a strip.

use ledkit_color::Rgb;
use ledkit_layout::Segment;

use crate::{Animation, Frame, Position};

/// Different animations on different parts of a strip, as one animation.
///
/// Each animation sees its own segment as a whole strip: its first LED is
/// LED 0, and its length is the segment's length. LEDs outside every segment
/// stay dark; where segments overlap, the first one listed wins. Segments
/// follow the wiring order, on a matrix too.
///
/// You can also render separate animations into separate
/// [`SegmentMut`](ledkit_layout::SegmentMut)s; use `Segments` when you want the
/// split-up strip to be one animation, for example a step in a playlist.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
/// use ledkit_layout::Segment;
///
/// let left = Comet::new();
/// let right = Sparkle::new();
/// let parts: [(Segment, &dyn Animation); 2] = [
///     (Segment::new(0, 50), &left),
///     (Segment::new(50, 50).reversed(), &right),
/// ];
/// let split = Segments::new(&parts);
///
/// let mut leds = [Rgb::BLACK; 100];
/// split.render(&mut leds, Time::from_millis(0));
/// assert_eq!(leds[0], Rgb::CYAN); // the comet's head
/// ```
#[derive(Copy, Clone)]
#[must_use]
pub struct Segments<'a> {
    parts: &'a [(Segment, &'a dyn Animation)],
}

impl<'a> Segments<'a> {
    /// Each animation drawn on its segment.
    pub const fn new(parts: &'a [(Segment, &'a dyn Animation)]) -> Self {
        Self { parts }
    }
}

impl core::fmt::Debug for Segments<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_list()
            .entries(self.parts.iter().map(|(segment, _)| segment))
            .finish()
    }
}

impl Animation for Segments<'_> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let index = usize::from(at.index);
        for (segment, animation) in self.parts {
            if index < segment.start() || index - segment.start() >= segment.len() {
                continue;
            }
            let mut local = index - segment.start();
            if segment.is_reversed() {
                local = segment.len() - 1 - local;
            }
            let local = u16::try_from(local).unwrap_or(u16::MAX);
            let len = u16::try_from(segment.len()).unwrap_or(u16::MAX);
            let position = Position {
                x: local,
                y: 0,
                index: local,
            };
            return animation.color_at(position, &Frame::new(frame.millis, len, 1));
        }
        Rgb::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};
    use crate::Solid;

    /// Shows its position and the length it was given.
    struct Where;

    impl Animation for Where {
        fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
            Rgb::new(at.x as u8, at.index as u8, frame.width as u8)
        }
    }

    #[test]
    fn each_part_sees_its_own_strip() {
        let red = Solid::new().color(Rgb::RED);
        let parts: [(Segment, &dyn Animation); 2] = [
            (Segment::new(0, 4), &red),
            (Segment::new(6, 4).reversed(), &Where),
        ];
        let split = Segments::new(&parts);
        for x in 0..4 {
            assert_eq!(strip_at(&split, x, 10, 0), Rgb::RED);
        }
        assert_eq!(strip_at(&split, 4, 10, 0), Rgb::BLACK);
        assert_eq!(strip_at(&split, 5, 10, 0), Rgb::BLACK);
        assert_eq!(strip_at(&split, 9, 10, 0), Rgb::new(0, 0, 4));
        assert_eq!(strip_at(&split, 6, 10, 0), Rgb::new(3, 3, 4));
        exercise(&split);
    }

    #[test]
    fn empty_and_out_of_range_segments() {
        let parts: [(Segment, &dyn Animation); 2] = [
            (Segment::new(3, 0), &Where),
            (Segment::new(1000, 5), &Where),
        ];
        exercise(&Segments::new(&parts));
        exercise(&Segments::new(&[]));
    }
}
