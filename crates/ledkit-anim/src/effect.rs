//! [`Effect`]: every built-in effect, by name.

use ledkit_color::Rgb;

use crate::{
    Animation, Blink, Breathe, Chase, ColorCycle, ColorWipe, Comet, Flames, Frame, GradientScroll,
    Pinwheel, Plasma, Position, Rain, Rainbow, Rings, Solid, Sparkle, Swirl,
};

/// Any one of the built-in effects. Use it to pick an effect by name (from a
/// button, a serial command or a settings file), or to keep a list of
/// different effects in one array.
///
/// Every effect also converts into an `Effect` with `.into()`, keeping its
/// settings.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// let effect = Effect::from_name("Rotating X").unwrap();
/// assert_eq!(effect.name(), "pinwheel");
///
/// // A customised effect, stored as an `Effect`.
/// let red_comet: Effect = Comet::new().color(Rgb::RED).into();
///
/// // Every effect with its default settings.
/// for effect in Effect::ALL {
///     let mut leds = [Rgb::BLACK; 10];
///     effect.render(&mut leds, Time::from_millis(0));
/// }
/// ```
#[derive(Copy, Clone, Debug)]
pub enum Effect {
    /// [`Solid`]: one color everywhere.
    Solid(Solid),
    /// [`Blink`]: switching between two colors.
    Blink(Blink),
    /// [`Breathe`]: slowly brightening and dimming.
    Breathe(Breathe),
    /// [`Rainbow`]: a moving rainbow.
    Rainbow(Rainbow),
    /// [`ColorCycle`]: everything one color, changing through a palette.
    ColorCycle(ColorCycle),
    /// [`GradientScroll`]: a palette sliding along.
    GradientScroll(GradientScroll),
    /// [`ColorWipe`]: filling the strip LED by LED.
    ColorWipe(ColorWipe),
    /// [`Chase`]: theater-sign lights.
    Chase(Chase),
    /// [`Comet`]: a dot with a fading tail.
    Comet(Comet),
    /// [`Sparkle`]: random twinkles.
    Sparkle(Sparkle),
    /// [`Flames`]: flickering fire.
    Flames(Flames),
    /// [`Swirl`]: a spiral of color.
    Swirl(Swirl),
    /// [`Rings`]: rings spreading from the middle.
    Rings(Rings),
    /// [`Pinwheel`]: spinning arms.
    Pinwheel(Pinwheel),
    /// [`Plasma`]: flowing blobs of color.
    Plasma(Plasma),
    /// [`Rain`]: falling drops.
    Rain(Rain),
}

/// Other names people use for the effects, and the effect each one means.
const ALIASES: [(&str, &str); 27] = [
    ("static", "solid"),
    ("strobe", "blink"),
    ("flash", "blink"),
    ("pulse", "breathe"),
    ("colorloop", "rainbow"),
    ("fade", "color cycle"),
    ("palettescroll", "gradient scroll"),
    ("wipe", "color wipe"),
    ("fill", "color wipe"),
    ("loading", "color wipe"),
    ("theaterchase", "chase"),
    ("marquee", "chase"),
    ("runninglights", "chase"),
    ("meteor", "comet"),
    ("scanner", "comet"),
    ("larson", "comet"),
    ("cylon", "comet"),
    ("glitter", "sparkle"),
    ("twinkle", "sparkle"),
    ("fire", "flames"),
    ("candle", "flames"),
    ("spiral", "swirl"),
    ("concentric", "rings"),
    ("rotatingarms", "pinwheel"),
    ("rotatingx", "pinwheel"),
    ("matrixrain", "rain"),
    ("digitalrain", "rain"),
];

impl Effect {
    /// Every built-in effect, with its default settings.
    pub const ALL: [Effect; 16] = [
        Effect::Solid(Solid::new()),
        Effect::Blink(Blink::new()),
        Effect::Breathe(Breathe::new()),
        Effect::Rainbow(Rainbow::new()),
        Effect::ColorCycle(ColorCycle::new()),
        Effect::GradientScroll(GradientScroll::new()),
        Effect::ColorWipe(ColorWipe::new()),
        Effect::Chase(Chase::new()),
        Effect::Comet(Comet::new()),
        Effect::Sparkle(Sparkle::new()),
        Effect::Flames(Flames::new()),
        Effect::Swirl(Swirl::new()),
        Effect::Rings(Rings::new()),
        Effect::Pinwheel(Pinwheel::new()),
        Effect::Plasma(Plasma::new()),
        Effect::Rain(Rain::new()),
    ];

    /// The effect's name, in lowercase with spaces, such as `"color wipe"`.
    ///
    /// ```
    /// use ledkit_anim::prelude::*;
    ///
    /// assert_eq!(Effect::from(Swirl::new()).name(), "swirl");
    /// ```
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Solid(_) => "solid",
            Self::Blink(_) => "blink",
            Self::Breathe(_) => "breathe",
            Self::Rainbow(_) => "rainbow",
            Self::ColorCycle(_) => "color cycle",
            Self::GradientScroll(_) => "gradient scroll",
            Self::ColorWipe(_) => "color wipe",
            Self::Chase(_) => "chase",
            Self::Comet(_) => "comet",
            Self::Sparkle(_) => "sparkle",
            Self::Flames(_) => "flames",
            Self::Swirl(_) => "swirl",
            Self::Rings(_) => "rings",
            Self::Pinwheel(_) => "pinwheel",
            Self::Plasma(_) => "plasma",
            Self::Rain(_) => "rain",
        }
    }

    /// The effect called `name`, with default settings, or `None` if there is
    /// no such effect.
    ///
    /// Capitals, spaces, `-` and `_` don't matter, and common other names
    /// work too: "marquee" and "theater chase" give a [`Chase`], "fire" gives
    /// [`Flames`], "concentric" gives [`Rings`], "rotating X" gives a
    /// [`Pinwheel`], "scanner" gives a [`Comet`] and so on.
    ///
    /// ```
    /// use ledkit_anim::prelude::*;
    ///
    /// assert_eq!(Effect::from_name("Theater-Chase").unwrap().name(), "chase");
    /// assert!(Effect::from_name("no such thing").is_none());
    /// ```
    pub fn from_name(name: &str) -> Option<Effect> {
        let wanted = |candidate: &str| same_name(name, candidate);
        let name = ALIASES
            .iter()
            .find(|(alias, _)| wanted(alias))
            .map_or(name, |(_, effect)| *effect);
        Self::ALL
            .into_iter()
            .find(|effect| same_name(name, effect.name()))
    }

    /// The effect inside, as an [`Animation`].
    fn animation(&self) -> &dyn Animation {
        match self {
            Self::Solid(e) => e,
            Self::Blink(e) => e,
            Self::Breathe(e) => e,
            Self::Rainbow(e) => e,
            Self::ColorCycle(e) => e,
            Self::GradientScroll(e) => e,
            Self::ColorWipe(e) => e,
            Self::Chase(e) => e,
            Self::Comet(e) => e,
            Self::Sparkle(e) => e,
            Self::Flames(e) => e,
            Self::Swirl(e) => e,
            Self::Rings(e) => e,
            Self::Pinwheel(e) => e,
            Self::Plasma(e) => e,
            Self::Rain(e) => e,
        }
    }
}

/// True if `a` and `b` are the same name, ignoring case, spaces, `-` and `_`.
fn same_name(a: &str, b: &str) -> bool {
    fn squash(s: &str) -> impl Iterator<Item = char> + '_ {
        s.chars()
            .filter(|c| !matches!(c, ' ' | '-' | '_'))
            .map(|c| c.to_ascii_lowercase())
    }
    squash(a).eq(squash(b))
}

impl Animation for Effect {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        self.animation().color_at(at, frame)
    }
}

macro_rules! into_effect {
    ($($name:ident),*) => {
        $(
            impl From<$name> for Effect {
                fn from(effect: $name) -> Self {
                    Self::$name(effect)
                }
            }
        )*
    };
}

into_effect!(
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
    Rain
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::test_support::exercise;

    #[test]
    fn names_and_aliases() {
        assert_eq!(Effect::from_name("Rotating-X").unwrap().name(), "pinwheel");
        assert_eq!(Effect::from_name("MARQUEE").unwrap().name(), "chase");
        assert_eq!(
            Effect::from_name("color_wipe").unwrap().name(),
            "color wipe"
        );
        assert_eq!(Effect::from_name("Fire").unwrap().name(), "flames");
        assert!(Effect::from_name("nope").is_none());
        assert!(Effect::from_name("").is_none());
    }

    #[test]
    fn every_name_round_trips_and_is_unique() {
        for (i, effect) in Effect::ALL.iter().enumerate() {
            assert_eq!(
                Effect::from_name(effect.name()).unwrap().name(),
                effect.name()
            );
            for other in &Effect::ALL[i + 1..] {
                assert_ne!(effect.name(), other.name());
            }
        }
        for (alias, target) in ALIASES {
            assert!(Effect::from_name(alias).is_some(), "{alias}");
            assert!(Effect::from_name(target).is_some(), "{target}");
        }
    }

    #[test]
    fn every_effect_renders() {
        for effect in Effect::ALL {
            exercise(&effect);
        }
    }
}
