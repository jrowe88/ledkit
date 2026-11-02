//! Stacking one animation on another: [`Layer`] and [`Blend`].

use ledkit_color::Rgb;

use crate::math::lerp_u8;
use crate::{Animation, Frame, Position};

/// How a top layer combines with what is under it.
///
/// # Example
///
/// ```
/// use ledkit_anim::Blend;
/// use ledkit_color::Rgb;
///
/// // Black on top lets the base show through.
/// assert_eq!(Blend::Over.mix(Rgb::BLUE, Rgb::BLACK), Rgb::BLUE);
/// assert_eq!(Blend::Add.mix(Rgb::RED, Rgb::BLUE), Rgb::MAGENTA);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Blend {
    /// The top layer covers the base, as much as it is bright: black is
    /// see-through, full brightness covers completely, and a dim tail fades
    /// into what is under it. The usual choice.
    #[default]
    Over,
    /// Adds the light of both, like two lamps on one spot.
    Add,
    /// Multiplies them: white on top changes nothing, black on top hides the
    /// base. Use a pattern on top as a mask.
    Multiply,
    /// The opposite of multiply: always at least as bright as either.
    Screen,
    /// The brighter of the two, channel by channel.
    Lighten,
    /// The darker of the two, channel by channel.
    Darken,
    /// Halfway between the two.
    Average,
}

impl Blend {
    /// The color of `top` laid on `base` with this blend.
    ///
    /// ```
    /// use ledkit_anim::Blend;
    /// use ledkit_color::Rgb;
    ///
    /// assert_eq!(Blend::Multiply.mix(Rgb::RED, Rgb::WHITE), Rgb::RED);
    /// ```
    pub const fn mix(self, base: Rgb, top: Rgb) -> Rgb {
        match self {
            Self::Over => {
                let cover = max3(top.r, top.g, top.b);
                Rgb::new(
                    over(base.r, top.r, cover),
                    over(base.g, top.g, cover),
                    over(base.b, top.b, cover),
                )
            }
            Self::Add => Rgb::new(
                base.r.saturating_add(top.r),
                base.g.saturating_add(top.g),
                base.b.saturating_add(top.b),
            ),
            Self::Multiply => Rgb::new(
                multiply(base.r, top.r),
                multiply(base.g, top.g),
                multiply(base.b, top.b),
            ),
            Self::Screen => Rgb::new(
                screen(base.r, top.r),
                screen(base.g, top.g),
                screen(base.b, top.b),
            ),
            Self::Lighten => Rgb::new(max(base.r, top.r), max(base.g, top.g), max(base.b, top.b)),
            Self::Darken => Rgb::new(min(base.r, top.r), min(base.g, top.g), min(base.b, top.b)),
            Self::Average => Rgb::new(
                lerp_u8(base.r, top.r, 128),
                lerp_u8(base.g, top.g, 128),
                lerp_u8(base.b, top.b, 128),
            ),
        }
    }
}

const fn max(a: u8, b: u8) -> u8 {
    if a > b {
        a
    } else {
        b
    }
}

const fn min(a: u8, b: u8) -> u8 {
    if a < b {
        a
    } else {
        b
    }
}

const fn max3(a: u8, b: u8, c: u8) -> u8 {
    max(a, max(b, c))
}

/// `top` plus whatever of `base` its coverage lets through.
const fn over(base: u8, top: u8, cover: u8) -> u8 {
    let through = base as u32 * (255 - cover as u32) / 255;
    let sum = top as u32 + through;
    if sum > 255 {
        255
    } else {
        sum as u8
    }
}

const fn multiply(a: u8, b: u8) -> u8 {
    ((a as u32 * b as u32 + 127) / 255) as u8
}

const fn screen(a: u8, b: u8) -> u8 {
    255 - multiply(255 - a, 255 - b)
}

/// One animation laid on top of another. Made by
/// [`AnimationExt::layer`](crate::AnimationExt::layer).
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// // A white comet flying over a slow rainbow.
/// let show = Rainbow::new().dimmed(80).layer(Comet::new().color(Rgb::WHITE), Blend::Over);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Layer<B, T> {
    pub(crate) base: B,
    pub(crate) top: T,
    pub(crate) blend: Blend,
}

impl<B: Animation, T: Animation> Animation for Layer<B, T> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        self.blend
            .mix(self.base.color_at(at, frame), self.top.color_at(at, frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::{exercise, strip_at};
    use crate::{AnimationExt, Comet, Solid};

    #[test]
    fn over() {
        assert_eq!(Blend::Over.mix(Rgb::BLUE, Rgb::BLACK), Rgb::BLUE);
        assert_eq!(Blend::Over.mix(Rgb::BLUE, Rgb::WHITE), Rgb::WHITE);
        let mixed = Blend::Over.mix(Rgb::new(0, 0, 200), Rgb::new(128, 0, 0));
        assert_eq!(mixed.r, 128);
        assert!(mixed.b.abs_diff(100) <= 1, "{mixed:?}");
    }

    #[test]
    fn the_others() {
        assert_eq!(
            Blend::Add.mix(Rgb::new(200, 10, 0), Rgb::new(100, 10, 0)),
            Rgb::new(255, 20, 0)
        );
        assert_eq!(
            Blend::Multiply.mix(Rgb::new(10, 20, 30), Rgb::WHITE),
            Rgb::new(10, 20, 30)
        );
        assert_eq!(
            Blend::Multiply.mix(Rgb::new(10, 20, 30), Rgb::BLACK),
            Rgb::BLACK
        );
        assert_eq!(
            Blend::Screen.mix(Rgb::new(10, 20, 30), Rgb::BLACK),
            Rgb::new(10, 20, 30)
        );
        assert_eq!(
            Blend::Lighten.mix(Rgb::new(10, 200, 0), Rgb::new(100, 0, 5)),
            Rgb::new(100, 200, 5)
        );
        assert_eq!(
            Blend::Darken.mix(Rgb::new(10, 200, 0), Rgb::new(100, 0, 5)),
            Rgb::new(10, 0, 0)
        );
        let average = Blend::Average.mix(Rgb::BLACK, Rgb::WHITE);
        assert!((127..=128).contains(&average.r));
    }

    #[test]
    fn layer_shows_base_where_top_is_dark() {
        let show = Solid::new()
            .color(Rgb::BLUE)
            .layer(Comet::new().color(Rgb::RED), Blend::Over);
        assert_eq!(strip_at(&show, 0, 10, 0), Rgb::RED);
        assert_eq!(strip_at(&show, 5, 10, 0), Rgb::BLUE);
        exercise(&show);
    }
}
