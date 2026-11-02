//! Distance and angle from a center point, shared by the round effects.

use crate::{math, Frame, Position};

/// Where an LED is, seen from the center.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Polar {
    /// Distance from the center, in 1/16ths of an LED.
    pub distance: u32,
    /// Angle around the center: 65536 steps a full turn, 0 pointing right,
    /// growing clockwise.
    pub angle: u16,
}

/// The center to measure from, in 1/16ths of an LED: `center` if given (in
/// LEDs), otherwise the middle of the surface (between LEDs on even sizes).
pub(crate) fn center16(frame: Frame, center: Option<(u16, u16)>) -> (i32, i32) {
    match center {
        Some((x, y)) => (i32::from(x) * 16, i32::from(y) * 16),
        None => (
            i32::from(frame.width.saturating_sub(1)) * 8,
            i32::from(frame.height.saturating_sub(1)) * 8,
        ),
    }
}

/// The distance and angle of `at` from the center.
pub(crate) fn polar(at: Position, frame: Frame, center: Option<(u16, u16)>) -> Polar {
    let (cx, cy) = center16(frame, center);
    let dx = i32::from(at.x) * 16 - cx;
    let dy = i32::from(at.y) * 16 - cy;
    let squared = i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy);
    Polar {
        distance: math::isqrt(u32::try_from(squared).unwrap_or(u32::MAX)),
        angle: math::atan2(dy, dx),
    }
}

/// The largest distance from the center to a corner, in 1/16ths of an LED (at least 1).
pub(crate) fn farthest(frame: Frame, center: Option<(u16, u16)>) -> u32 {
    let (cx, cy) = center16(frame, center);
    let w = i32::from(frame.width.saturating_sub(1)) * 16;
    let h = i32::from(frame.height.saturating_sub(1)) * 16;
    let dx = i64::from(cx.max(w - cx));
    let dy = i64::from(cy.max(h - cy));
    math::isqrt(u32::try_from(dx * dx + dy * dy).unwrap_or(u32::MAX)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middle_of_an_even_grid_is_between_leds() {
        let frame = Frame::new(0, 8, 8);
        let a = polar(
            Position {
                x: 3,
                y: 3,
                index: 0,
            },
            frame,
            None,
        );
        let b = polar(
            Position {
                x: 4,
                y: 4,
                index: 0,
            },
            frame,
            None,
        );
        assert_eq!(a.distance, b.distance);
        assert_eq!(a.distance, 11); // half an LED each way: about 0.7 LEDs
    }

    #[test]
    fn farthest_is_the_corner() {
        assert_eq!(farthest(Frame::new(0, 9, 9), None), 90); // 4 LEDs each way
        assert_eq!(farthest(Frame::new(0, 1, 1), None), 1);
    }
}
