//! Changing from one animation to the next: [`Transition`], [`Style`],
//! [`Easing`] and [`Edge`].

use core::time::Duration;

use ledkit_color::Rgb;

use crate::effects::geometry::{farthest, polar};
use crate::math::{hash, scale_u8};
use crate::time::millis;
use crate::{Frame, Position};

/// How the speed of a transition changes from start to end.
///
/// # Example
///
/// ```
/// use ledkit_anim::Easing;
///
/// assert_eq!(Easing::Linear.apply(100), 100);
/// assert!(Easing::EaseIn.apply(100) < 100); // starts slowly
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Easing {
    /// The same speed all the way.
    Linear,
    /// Starts slowly, then speeds up.
    EaseIn,
    /// Starts quickly, then slows down to finish.
    EaseOut,
    /// Starts slowly, is fastest in the middle, and slows down to finish.
    #[default]
    EaseInOut,
}

impl Easing {
    /// How far along the change is (0 to 255) when the time is `progress`
    /// (0 to 255) of the way through. 0 and 255 are always unchanged.
    ///
    /// ```
    /// use ledkit_anim::Easing;
    ///
    /// assert_eq!(Easing::EaseInOut.apply(0), 0);
    /// assert_eq!(Easing::EaseInOut.apply(255), 255);
    /// ```
    pub const fn apply(self, progress: u8) -> u8 {
        let p = progress as u32;
        let eased = match self {
            Self::Linear => p,
            Self::EaseIn => p * p / 255,
            Self::EaseOut => 255 - (255 - p) * (255 - p) / 255,
            Self::EaseInOut => p * p * (3 * 255 - 2 * p) / (255 * 255),
        };
        eased as u8
    }
}

/// The edge a wipe starts from.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::{Edge, Transition};
///
/// let from_the_top = Transition::wipe(Edge::Top, Duration::from_secs(1));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Edge {
    /// The left edge (the start of a strip).
    #[default]
    Left,
    /// The right edge (the end of a strip).
    Right,
    /// The top edge.
    Top,
    /// The bottom edge.
    Bottom,
}

/// The look of a [`Transition`].
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::{Style, Transition};
///
/// assert_eq!(Transition::iris(Duration::from_secs(1)).style(), Style::Iris);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Style {
    /// Switch at once.
    #[default]
    Cut,
    /// Blend smoothly from one to the other (a crossfade).
    Fade,
    /// Fade the first out to black, then the second in.
    FadeThroughBlack,
    /// The new animation sweeps in from an edge.
    Wipe(Edge),
    /// LEDs switch over one by one, in a random order.
    Dissolve,
    /// The new animation grows out from the middle as a circle.
    Iris,
}

/// How one animation changes into the next, and how long it takes.
///
/// Used by [`Playlist`](crate::Playlist) entries and by
/// [`AnimationExt::then`](crate::AnimationExt::then).
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::{Easing, Transition};
///
/// let gentle = Transition::fade(Duration::from_secs(2));
/// let quick_wipe = Transition::wipe(ledkit_anim::Edge::Left, Duration::from_millis(500))
///     .easing(Easing::EaseOut);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[must_use]
pub struct Transition {
    style: Style,
    millis: u32,
    easing: Easing,
}

impl Transition {
    /// Switch at once.
    pub const fn cut() -> Self {
        Self {
            style: Style::Cut,
            millis: 0,
            easing: Easing::Linear,
        }
    }

    /// Blend smoothly from one to the other over `length` (a crossfade).
    pub const fn fade(length: Duration) -> Self {
        Self::new(Style::Fade, length, Easing::EaseInOut)
    }

    /// Fade the first out to black and the second in, over `length` in all.
    pub const fn fade_through_black(length: Duration) -> Self {
        Self::new(Style::FadeThroughBlack, length, Easing::Linear)
    }

    /// The new animation sweeps in from `edge` over `length`.
    pub const fn wipe(edge: Edge, length: Duration) -> Self {
        Self::new(Style::Wipe(edge), length, Easing::Linear)
    }

    /// LEDs switch over one by one, in a random order, over `length`.
    pub const fn dissolve(length: Duration) -> Self {
        Self::new(Style::Dissolve, length, Easing::Linear)
    }

    /// The new animation grows out from the middle as a circle over `length`.
    pub const fn iris(length: Duration) -> Self {
        Self::new(Style::Iris, length, Easing::Linear)
    }

    const fn new(style: Style, length: Duration, easing: Easing) -> Self {
        Self {
            style,
            millis: millis(length),
            easing,
        }
    }

    /// Changes how the speed of the transition varies (see [`Easing`]).
    pub const fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// The look of this transition.
    ///
    /// ```
    /// use ledkit_anim::{Style, Transition};
    ///
    /// assert_eq!(Transition::cut().style(), Style::Cut);
    /// ```
    pub const fn style(&self) -> Style {
        self.style
    }

    /// How long the transition takes, in milliseconds.
    ///
    /// ```
    /// use core::time::Duration;
    /// use ledkit_anim::Transition;
    ///
    /// assert_eq!(Transition::fade(Duration::from_secs(2)).length_millis(), 2000);
    /// ```
    pub const fn length_millis(&self) -> u32 {
        self.millis
    }

    /// The color of the LED at `at` when the transition from `from` to `to`
    /// is `progress` (0 to 255) of the way through. 0 gives `from` and 255
    /// gives `to`, whatever the style.
    ///
    /// ```
    /// use core::time::Duration;
    /// use ledkit_anim::{Frame, Position, Transition};
    /// use ledkit_color::Rgb;
    ///
    /// let fade = Transition::fade(Duration::from_secs(1));
    /// let at = Position::default();
    /// let frame = Frame::new(0, 1, 1);
    /// assert_eq!(fade.mix(Rgb::RED, Rgb::BLUE, 255, at, &frame), Rgb::BLUE);
    /// ```
    pub fn mix(&self, from: Rgb, to: Rgb, progress: u8, at: Position, frame: &Frame) -> Rgb {
        match progress {
            0 => return from,
            255 => return to,
            _ => {}
        }
        let done = self.easing.apply(progress);
        let switched = |place: u8| place < done;
        match self.style {
            Style::Cut => to,
            Style::Fade => from.lerp(to, done),
            Style::FadeThroughBlack => {
                if done <= 128 {
                    from.scale(255 - done.saturating_mul(2))
                } else {
                    to.scale(scale_u8(u32::from(done) - 128, 127))
                }
            }
            Style::Wipe(edge) => {
                let (place, size) = match edge {
                    Edge::Left => (at.x, frame.width),
                    Edge::Right => (
                        frame.width.saturating_sub(1).saturating_sub(at.x),
                        frame.width,
                    ),
                    Edge::Top => (at.y, frame.height),
                    Edge::Bottom => (
                        frame.height.saturating_sub(1).saturating_sub(at.y),
                        frame.height,
                    ),
                };
                let place = (u32::from(place) * 256 / u32::from(size.max(1))) as u8;
                if switched(place) {
                    to
                } else {
                    from
                }
            }
            Style::Dissolve => {
                if switched(hash(u32::from(at.index)) as u8) {
                    to
                } else {
                    from
                }
            }
            Style::Iris => {
                let place = scale_u8(polar(at, *frame, None).distance, farthest(*frame, None));
                if switched(place) {
                    to
                } else {
                    from
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn all_styles() -> [Transition; 9] {
        [
            Transition::cut(),
            Transition::fade(SECOND),
            Transition::fade_through_black(SECOND),
            Transition::wipe(Edge::Left, SECOND),
            Transition::wipe(Edge::Right, SECOND),
            Transition::wipe(Edge::Top, SECOND),
            Transition::wipe(Edge::Bottom, SECOND),
            Transition::dissolve(SECOND),
            Transition::iris(SECOND),
        ]
    }

    /// Every LED of an 8x8 at `progress`, wired row by row.
    fn grid(transition: Transition, progress: u8) -> [Rgb; 64] {
        let frame = Frame::new(0, 8, 8);
        core::array::from_fn(|i| {
            let at = Position {
                x: (i % 8) as u16,
                y: (i / 8) as u16,
                index: i as u16,
            };
            transition.mix(Rgb::RED, Rgb::BLUE, progress, at, &frame)
        })
    }

    #[test]
    fn every_style_starts_at_from_and_ends_at_to() {
        for transition in all_styles() {
            assert_eq!(grid(transition, 0), [Rgb::RED; 64], "{transition:?}");
            assert_eq!(grid(transition, 255), [Rgb::BLUE; 64], "{transition:?}");
        }
    }

    #[test]
    fn fades() {
        let half = grid(Transition::fade(SECOND).easing(Easing::Linear), 128)[0];
        assert_eq!(half, Rgb::RED.lerp(Rgb::BLUE, 128));
        assert_eq!(
            grid(Transition::fade_through_black(SECOND), 128)[0],
            Rgb::BLACK
        );
    }

    #[test]
    fn wipe_from_the_left_switches_the_left_half() {
        let leds = grid(Transition::wipe(Edge::Left, SECOND), 128);
        assert_eq!(leds[3], Rgb::BLUE);
        assert_eq!(leds[4], Rgb::RED);
        let leds = grid(Transition::wipe(Edge::Bottom, SECOND), 128);
        assert_eq!(leds[63], Rgb::BLUE);
        assert_eq!(leds[0], Rgb::RED);
    }

    #[test]
    fn dissolve_switches_about_half() {
        let switched = grid(Transition::dissolve(SECOND), 128)
            .iter()
            .filter(|&&c| c == Rgb::BLUE)
            .count();
        assert!((19..=45).contains(&switched), "{switched}");
    }

    #[test]
    fn iris_opens_from_the_middle() {
        let leds = grid(Transition::iris(SECOND), 128);
        assert_eq!(leds[3 * 8 + 3], Rgb::BLUE);
        assert_eq!(leds[0], Rgb::RED);
        assert_eq!(leds[63], Rgb::RED);
    }

    #[test]
    fn easing_endpoints() {
        for easing in [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
        ] {
            assert_eq!(easing.apply(0), 0);
            assert_eq!(easing.apply(255), 255);
        }
    }
}
