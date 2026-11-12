//! The scenes of the show. Each one is an [`Animation`]: a recipe that gives the
//! color of every LED at every moment from just two things, where the LED is and
//! what time it is. Nothing is remembered between frames, which is what lets the
//! show fade smoothly from one scene into the next.
//!
//! Two tricks make them look good on a dense strip:
//!
//! - **Positions in 1/256ths of an LED.** A firefly or a sheen of light sits
//!   *between* LEDs and glides from one to the next, instead of hopping.
//! - **Slow, overlapping waves.** Two or three waves with periods that don't line
//!   up (23 s and 9.7 s, say) add up to motion that never quite repeats.
//!
//! All the math is whole numbers (no floating point per LED). To tinker, change
//! the constants: the colors, the speeds (in milliseconds per cycle) and the sizes
//! (in LEDs).

use ledkit::anim::math::{hash, hash2, noise, phase16, sine};
use ledkit::color::Gradient;
use ledkit::prelude::*;

// ---------------------------------------------------------------------------
// Aurora
// ---------------------------------------------------------------------------

/// Northern lights: soft curtains of green, teal and violet drifting along the
/// strip, with stars twinkling in the dark gaps between them.
#[derive(Copy, Clone, Debug, Default)]
pub struct Aurora;

/// The aurora's colors, from green through teal and sky blue to violet and pink.
const AURORA_COLORS: Gradient<5> = Gradient::new([
    (0, Rgb::new(0, 255, 80)),
    (85, Rgb::new(0, 210, 160)),
    (150, Rgb::new(10, 110, 255)),
    (210, Rgb::new(110, 30, 255)),
    (255, Rgb::new(230, 20, 160)),
]);

/// The stars: a cool white.
const STAR: Rgb = Rgb::new(150, 170, 255);

impl Animation for Aurora {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let x = u32::from(at.x);
        let t = frame.millis;

        // The color: a slowly rolling landscape of noise with hills about 30
        // LEDs apart, drifting along the strip.
        let shade = noise(x * 256 / 30 + t / 40, t / 25);
        let color = AURORA_COLORS.color_at(stretch(shade, 60, 200));

        // The curtains: a finer noise drifting the other way. Low values are
        // the dark gaps; squaring keeps the edges soft and the gaps deep.
        let curtain = noise(x * 256 / 13 + (FAR_AWAY - t / 28), t / 18 + 7777);
        let lit = square(stretch(curtain, 90, 215));

        // Folds in the curtains: a gentle ripple running along them.
        let fold = 185 + u32::from(sine((x * 22).wrapping_sub(t / 5) as u8)) * 70 / 255;
        let glow = color.scale((u32::from(lit) * fold / 255) as u8);

        // Stars, only where the curtains leave the sky dark.
        let star = twinkle(x, t, 6, 1500);
        add(glow, STAR.scale(star).scale(255 - lit))
    }
}

// ---------------------------------------------------------------------------
// Fireflies
// ---------------------------------------------------------------------------

/// Fireflies on a summer evening: warm glows that fade in somewhere along the
/// strip, drift a little, and fade out again, each in its own rhythm, over a
/// dusky sky.
#[derive(Copy, Clone, Debug, Default)]
pub struct Fireflies;

/// One firefly: its color, and how often it glows, in milliseconds.
struct Firefly {
    color: Rgb,
    rhythm: u32,
}

impl Firefly {
    const fn new(color: Rgb, rhythm: u32) -> Self {
        Self { color, rhythm }
    }
}

/// The fireflies. Their rhythms are all different, so they never fall into step.
const FIREFLIES: [Firefly; 10] = [
    Firefly::new(Rgb::new(255, 170, 30), 4_300),
    Firefly::new(Rgb::new(190, 255, 40), 5_900),
    Firefly::new(Rgb::new(255, 120, 10), 3_700),
    Firefly::new(Rgb::new(255, 210, 80), 6_700),
    Firefly::new(Rgb::new(160, 255, 60), 5_100),
    Firefly::new(Rgb::new(255, 150, 40), 4_700),
    Firefly::new(Rgb::new(220, 255, 50), 6_100),
    Firefly::new(Rgb::new(255, 190, 60), 5_300),
    Firefly::new(Rgb::new(255, 140, 20), 7_300),
    Firefly::new(Rgb::new(200, 255, 70), 3_900),
];

/// How far a firefly's glow reaches to either side, in LEDs.
const FIREFLY_RADIUS: u32 = 4;

/// How far a firefly drifts while it glows, in LEDs.
const FIREFLY_DRIFT: u32 = 5;

/// The middle of a glowing firefly leans toward this warm white.
const WARM_WHITE: Rgb = Rgb::new(255, 230, 180);

/// The evening sky behind them: deep blue at one end, plum at the other.
const DUSK: Gradient<2> = Gradient::new([(0, Rgb::new(2, 3, 22)), (255, Rgb::new(16, 2, 14))]);

impl Animation for Fireflies {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let x = u32::from(at.x);
        let width = u32::from(frame.width.max(1));
        let t = frame.millis;

        // The sky: a dim, still gradient. (Moving it would show: colors this
        // dim have only a few steps, and the eye catches each one.)
        let mut color = DUSK.color_at((x * 255 / width) as u8);

        // This LED's place, in 1/256ths of an LED.
        let here = (x * 256) as i32;
        for (n, fly) in FIREFLIES.iter().enumerate() {
            let n = n as u32;
            // Each firefly starts at its own point in its rhythm. It glows for
            // the first half of each round, then rests.
            let local = t.wrapping_add(hash(n) % fly.rhythm);
            let beat = (local % fly.rhythm) * 65_536 / fly.rhythm;
            if beat >= 32_768 {
                continue;
            }
            // A new random number for each glow: where it is and how bright.
            let luck = hash2(n, local / fly.rhythm);

            // Somewhere new along the strip each time, drifting a little while
            // it glows (which way depends on its luck).
            let spot = ((luck >> 8) % (width * 256)) as i32;
            let mut drift = (beat as i32 - 16_384) * (FIREFLY_DRIFT * 256) as i32 / 32_768;
            if luck & 1 == 1 {
                drift = -drift;
            }
            let falloff = soft_spot((here - spot - drift).unsigned_abs(), FIREFLY_RADIUS);
            if falloff == 0 {
                continue;
            }

            // A soft rise and fall, some glows brighter than others, with a warm
            // white middle.
            let peak = 140 + luck % 116;
            let brightness = u32::from(bump((beat / 128) as u8)) * peak / 255;
            let hot = fly
                .color
                .lerp(WARM_WHITE, (falloff * falloff / 256 * 90 / 256) as u8);
            color = add(color, hot.scale((brightness * falloff / 256) as u8));
        }
        color
    }
}

// ---------------------------------------------------------------------------
// Silk
// ---------------------------------------------------------------------------

/// Flowing silk: a band of rainbow colors, slowly turning, folded back and forth
/// by two soft waves, with a sheen of light sliding along it.
#[derive(Copy, Clone, Debug, Default)]
pub struct Silk;

/// How much of the color wheel is on the strip at once (65536 is all of it).
const SILK_SPAN: u32 = 22_000;

/// How hard the two folds push the colors: the wave divided by this, so smaller
/// is stronger.
const SILK_PUSH_A: i32 = 20;
const SILK_PUSH_B: i32 = 60;

/// How far the sheen reaches to either side, in LEDs.
const SHEEN_RADIUS: u32 = 7;

impl Animation for Silk {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let x = u32::from(at.x);
        let width = u32::from(frame.width.max(1));
        let t = frame.millis;

        // Two folds, each a slow wave: one 40 LEDs long moving forward over 9 s,
        // one 23 LEDs long moving back over 6.5 s.
        let fold_a = (x * 65_536 / 40).wrapping_sub(u32::from(phase16(t, 9_000)));
        let fold_b = (x * 65_536 / 23).wrapping_add(u32::from(phase16(t, 6_500)));

        // A third of the color wheel along the strip, turning all the way round
        // every 45 s. The folds push the colors gently back and forth.
        let push = wave(fold_a) / SILK_PUSH_A + wave(fold_b) / SILK_PUSH_B;
        let hue = (x * SILK_SPAN / width + u32::from(phase16(t, 45_000))) as i32 + push;
        let color = Rgb::from_hue(Hue(hue as u16));

        // Light catches the slopes of the first fold. (It stays a little below
        // full: silk lights the whole strip, the other scenes only parts of it.)
        let facing = (wave(fold_a.wrapping_add(16_384)) + 32_768) as u32;
        let color = color.scale((120 + facing * 100 / 65_536) as u8);

        // A sheen sliding along the strip every 10 seconds, starting and ending
        // just off the ends.
        let sheen = (u32::from(phase16(t, 10_000)) * (width + 24) / 256) as i32 - 12 * 256;
        let falloff = soft_spot(((x * 256) as i32 - sheen).unsigned_abs(), SHEEN_RADIUS);
        color.lerp(Rgb::WHITE, (falloff * 110 / 256) as u8)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A start for coordinates that count down with time, so they never go below 0
/// (it takes about a year).
const FAR_AWAY: u32 = 1 << 30;

/// A smooth wave with fine steps, for things that move slowly. `phase` goes once
/// round every 65536 (only its low 16 bits count); the result runs from -32768
/// to about 32512.
fn wave(phase: u32) -> i32 {
    let step = (phase >> 8) as u8;
    let fine = (phase & 255) as i32;
    let here = i32::from(sine(step));
    let next = i32::from(sine(step.wrapping_add(1)));
    here * 256 + (next - here) * fine - 32_768
}

/// A single soft pulse: 0 at phase 0, rising to 255 at 128 and back to 0.
fn bump(phase: u8) -> u8 {
    255 - sine(phase.wrapping_add(64))
}

/// How much of a soft round glow reaches `distance` (in 1/256ths of an LED) from
/// its middle: 256 in the middle, fading smoothly to 0 at `radius` LEDs.
fn soft_spot(distance: u32, radius: u32) -> u32 {
    let reach = radius * 256;
    if distance >= reach {
        return 0;
    }
    let near = 256 - distance * distance / (reach * reach / 256);
    near * near / 256
}

/// Stretches `low..=high` to the full 0 to 255; values outside are clamped.
fn stretch(value: u8, low: u8, high: u8) -> u8 {
    let above = u32::from(value.clamp(low, high) - low);
    (above * 255 / u32::from(high - low)) as u8
}

/// `value` times itself, so the middle values drop and the ends stay.
fn square(value: u8) -> u8 {
    (u32::from(value) * u32::from(value) / 255) as u8
}

/// Two colors added together, as light adds.
fn add(a: Rgb, b: Rgb) -> Rgb {
    Rgb::new(
        a.r.saturating_add(b.r),
        a.g.saturating_add(b.g),
        a.b.saturating_add(b.b),
    )
}

/// A twinkle for the LED at `x`: each LED has its own cycle of 5 to 8 seconds,
/// and lights in about one in `one_in` of them for `length` milliseconds.
fn twinkle(x: u32, t: u32, one_in: u32, length: u32) -> u8 {
    let cycle = 5_000 + hash(x) % 3_000;
    let local = t + hash(x ^ 0x5bd1) % cycle;
    if !hash2(x, local / cycle).is_multiple_of(one_in) {
        return 0;
    }
    let into = local % cycle;
    if into >= length {
        return 0;
    }
    bump((into * 256 / length) as u8)
}
