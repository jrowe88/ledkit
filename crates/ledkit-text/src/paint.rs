//! [`Paint`]: what colors the letters.

use ledkit_anim::{
    Animation, Blink, Breathe, Chase, ColorCycle, ColorWipe, Comet, Effect, Flames, Frame,
    GradientScroll, Pinwheel, Plasma, Position, Rain, Rainbow, Rings, Solid, Sparkle, Swirl,
};
use ledkit_color::Rgb;

/// What colors the letters: one color, one of the built-in effects, or your
/// own animation.
///
/// The text decides where the letters are; the paint decides what color
/// each lit LED gets. Anything that converts into a `Paint` works wherever a
/// paint is asked for, so you can pass an `Rgb` or an effect directly.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// let plain: Paint = Rgb::RED.into();
/// let fancy: Paint = Rainbow::new().into();
/// ```
///
/// To use your own [`Animation`], keep it in a `static`:
///
/// ```
/// use ledkit_anim::{Animation, Frame, Position};
/// use ledkit_text::prelude::*;
///
/// struct AllGreen;
/// impl Animation for AllGreen {
///     fn color_at(&self, _at: Position, _frame: &Frame) -> Rgb {
///         Rgb::GREEN
///     }
/// }
///
/// static GREEN: AllGreen = AllGreen;
/// let paint = Paint::Custom(&GREEN);
/// ```
#[derive(Copy, Clone)]
pub enum Paint {
    /// One color for every letter.
    Color(Rgb),
    /// One of the built-in effects.
    Effect(Effect),
    /// Your own animation, kept in a `static`.
    Custom(&'static (dyn Animation + Sync)),
}

impl Paint {
    /// The color for the LED at `at`.
    // Used by `TextArea` and `Ticker`, which arrive in the next tasks.
    #[allow(dead_code, clippy::trivially_copy_pass_by_ref)] // `&Frame` matches `Animation`.
    pub(crate) fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        match self {
            Paint::Color(color) => *color,
            Paint::Effect(effect) => effect.color_at(at, frame),
            Paint::Custom(animation) => animation.color_at(at, frame),
        }
    }
}

impl From<Rgb> for Paint {
    fn from(color: Rgb) -> Self {
        Paint::Color(color)
    }
}

impl From<Effect> for Paint {
    fn from(effect: Effect) -> Self {
        Paint::Effect(effect)
    }
}

/// Lets each effect type be used as a `Paint` directly.
macro_rules! paint_from_effects {
    ($($effect:ident),* $(,)?) => {
        $(
            impl From<$effect> for Paint {
                fn from(effect: $effect) -> Self {
                    Paint::Effect(Effect::from(effect))
                }
            }
        )*
    };
}

paint_from_effects!(
    Solid,
    Blink,
    Breathe,
    Rainbow,
    ColorCycle,
    GradientScroll,
    ColorWipe,
    Chase,
    Comet,
    Sparkle,
    Flames,
    Swirl,
    Rings,
    Pinwheel,
    Plasma,
    Rain,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_color_and_effect() {
        let frame = Frame::new(500, 8, 8);
        let at = Position {
            x: 3,
            y: 2,
            index: 19,
        };
        assert_eq!(Paint::from(Rgb::RED).color_at(at, &frame), Rgb::RED);
        assert_eq!(
            Paint::from(Plasma::new()).color_at(at, &frame),
            Plasma::new().color_at(at, &frame)
        );
    }
}
