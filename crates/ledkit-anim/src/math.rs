//! Small integer helpers for writing effects: waves, angles, distances,
//! repeatable "random" numbers and smooth noise.
//!
//! They use whole numbers only, so they are fast on chips without a
//! floating-point unit. Angles and wave positions ("phases") go once round in
//! 256 steps (`u8`) or 65536 steps (`u16`).
//!
//! # Example
//!
//! ```
//! use ledkit_anim::math;
//!
//! // A smooth up-and-down level, once round every 256 steps.
//! assert_eq!(math::sine(64), 255);
//! // How far a pixel 3 across and 4 down is from the corner.
//! assert_eq!(math::isqrt(3 * 3 + 4 * 4), 5);
//! ```

/// A quarter of a sine wave, from the middle (128) up to the top (255).
const QUARTER_SINE: [u8; 65] = [
    128, 131, 134, 137, 140, 143, 146, 149, 152, 155, 158, 162, 165, 167, 170, 173, 176, 179, 182,
    185, 188, 190, 193, 196, 198, 201, 203, 206, 208, 211, 213, 215, 218, 220, 222, 224, 226, 228,
    230, 232, 234, 235, 237, 238, 240, 241, 243, 244, 245, 246, 248, 249, 250, 250, 251, 252, 253,
    253, 254, 254, 254, 255, 255, 255, 255,
];

/// A smooth wave: 128 at phase 0, up to 255 at 64, back to 128 at 128, down
/// to 0 at 192.
///
/// ```
/// use ledkit_anim::math::sine;
///
/// assert_eq!(sine(0), 128);
/// assert_eq!(sine(64), 255);
/// assert_eq!(sine(192), 0);
/// ```
pub const fn sine(phase: u8) -> u8 {
    let i = (phase & 63) as usize;
    match phase >> 6 {
        0 => QUARTER_SINE[i],
        1 => QUARTER_SINE[64 - i],
        2 => 255 - QUARTER_SINE[i],
        _ => 255 - QUARTER_SINE[64 - i],
    }
}

/// A straight up-and-down wave: 0 at phase 0, 255 at 128, back down to 1 at 255.
///
/// ```
/// use ledkit_anim::math::triangle;
///
/// assert_eq!(triangle(0), 0);
/// assert_eq!(triangle(128), 255);
/// assert_eq!(triangle(64), 128);
/// ```
pub const fn triangle(phase: u8) -> u8 {
    if phase < 128 {
        phase * 2
    } else {
        (255 - phase) * 2 + 1
    }
}

/// How far through a repeating cycle of `period` milliseconds the time
/// `millis` is, as a phase from 0 to 255. A `period` of 0 gives 0.
///
/// ```
/// use ledkit_anim::math::phase;
///
/// assert_eq!(phase(500, 1000), 128);
/// assert_eq!(phase(1500, 1000), 128);
/// ```
pub const fn phase(millis: u32, period: u32) -> u8 {
    if period == 0 {
        return 0;
    }
    ((millis % period) as u64 * 256 / period as u64) as u8
}

/// Like [`phase`], with 65536 steps per cycle.
///
/// ```
/// use ledkit_anim::math::phase16;
///
/// assert_eq!(phase16(250, 1000), 16384);
/// ```
pub const fn phase16(millis: u32, period: u32) -> u16 {
    if period == 0 {
        return 0;
    }
    ((millis % period) as u64 * 65536 / period as u64) as u16
}

/// Blends from `a` (at `t` = 0) to `b` (at `t` = 255).
///
/// ```
/// use ledkit_anim::math::lerp_u8;
///
/// assert_eq!(lerp_u8(100, 200, 0), 100);
/// assert_eq!(lerp_u8(100, 200, 255), 200);
/// assert_eq!(lerp_u8(200, 100, 128), 150);
/// ```
pub const fn lerp_u8(a: u8, b: u8, t: u8) -> u8 {
    ((a as u32 * (255 - t as u32) + b as u32 * t as u32 + 127) / 255) as u8
}

/// `value` as a share of `max`, from 0 to 255. Values above `max` give 255,
/// and a `max` of 0 gives 0.
///
/// ```
/// use ledkit_anim::math::scale_u8;
///
/// assert_eq!(scale_u8(5, 10), 127);
/// assert_eq!(scale_u8(20, 10), 255);
/// ```
pub const fn scale_u8(value: u32, max: u32) -> u8 {
    if max == 0 {
        return 0;
    }
    if value >= max {
        return 255;
    }
    (value as u64 * 255 / max as u64) as u8
}

/// The angle of the point `(dx, dy)` seen from `(0, 0)`, with 65536 steps in
/// a full turn.
///
/// 0 points right (more `x`), and the angle grows clockwise as you look at a
/// panel whose `y` grows downwards: straight down is 16384. Accurate to about
/// a quarter of a degree. `(0, 0)` gives 0.
///
/// ```
/// use ledkit_anim::math::atan2;
///
/// assert_eq!(atan2(0, 5), 0);
/// assert!(atan2(5, 0).abs_diff(16384) < 50); // straight down
/// assert!(atan2(0, -5).abs_diff(32768) < 50); // left
/// ```
pub const fn atan2(dy: i32, dx: i32) -> u16 {
    let ax = dx.unsigned_abs() as u64;
    let ay = dy.unsigned_abs() as u64;
    if ax == 0 && ay == 0 {
        return 0;
    }
    // The angle within the first eighth of a turn, then unfolded.
    let base = if ay <= ax {
        atan_unit(ay * 65536 / ax)
    } else {
        16384 - atan_unit(ax * 65536 / ay)
    };
    let angle = match (dx >= 0, dy >= 0) {
        (true, true) => base,
        (false, true) => 32768 - base,
        (false, false) => 32768 + base,
        (true, false) => 65536 - base,
    };
    (angle & 0xFFFF) as u16
}

/// atan(z) for z from 0 to 1 (as 0 to 65536), in 1/65536 turns (0 to 8192).
/// Uses atan(z) ≈ (π/4)z + 0.273·z(1 − z), good to about 0.2 degrees.
const fn atan_unit(z: u64) -> u64 {
    (8192 * z + 2847 * z * (65536 - z) / 65536) / 65536
}

/// The whole-number square root of `n`, rounded down.
///
/// ```
/// use ledkit_anim::math::isqrt;
///
/// assert_eq!(isqrt(16), 4);
/// assert_eq!(isqrt(15), 3);
/// ```
pub const fn isqrt(n: u32) -> u32 {
    let mut remainder = n;
    let mut root = 0u32;
    let mut bit = 1u32 << 30;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if remainder >= root + bit {
            remainder -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// Mixes the bits of `n` into a number that looks random. The same input
/// always gives the same output, which is what lets "random" effects like
/// sparkle work without remembering anything between frames.
///
/// ```
/// use ledkit_anim::math::hash;
///
/// assert_eq!(hash(7), hash(7));
/// assert_ne!(hash(7), hash(8));
/// ```
pub const fn hash(n: u32) -> u32 {
    // A widely used integer hash ("lowbias32", by Chris Wellons, public domain).
    let mut x = n;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// [`hash`] of two numbers at once.
///
/// ```
/// use ledkit_anim::math::hash2;
///
/// assert_ne!(hash2(1, 2), hash2(2, 1));
/// ```
pub const fn hash2(a: u32, b: u32) -> u32 {
    hash(a ^ hash(b).rotate_left(5))
}

/// Smooth noise: a gently rolling landscape of values from 0 to 255.
///
/// `x` and `y` count in 1/256ths of a "hill", so moving 256 steps goes from
/// one hill to the next, and moving 1 step changes the value only a little.
/// Slide `y` (or `x`) with time to make it flow.
///
/// ```
/// use ledkit_anim::math::noise;
///
/// let here = noise(1000, 2000);
/// let next_door = noise(1001, 2000);
/// assert!(here.abs_diff(next_door) <= 2);
/// ```
pub const fn noise(x: u32, y: u32) -> u8 {
    let (cx, cy) = (x >> 8, y >> 8);
    let (fx, fy) = (smooth(x & 255), smooth(y & 255));
    let top = mix(corner(cx, cy), corner(cx.wrapping_add(1), cy), fx);
    let bottom = mix(
        corner(cx, cy.wrapping_add(1)),
        corner(cx.wrapping_add(1), cy.wrapping_add(1)),
        fx,
    );
    mix(top, bottom, fy) as u8
}

/// The noise value at a hill top.
const fn corner(x: u32, y: u32) -> u32 {
    hash2(x, y) & 255
}

/// An S-shaped curve from 0 to 255 (smoothstep), so hills have round tops.
const fn smooth(f: u32) -> u32 {
    f * f * (3 * 256 - 2 * f) / 65536
}

/// Blends `a` to `b` by `t` out of 256.
const fn mix(a: u32, b: u32, t: u32) -> u32 {
    (a * (256 - t) + b * t) / 256
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_key_points() {
        assert_eq!(sine(0), 128);
        assert_eq!(sine(64), 255);
        assert!(sine(128).abs_diff(128) <= 1);
        assert_eq!(sine(192), 0);
    }

    #[test]
    fn sine_is_symmetric() {
        for p in 0..=255u8 {
            let sum = u16::from(sine(p)) + u16::from(sine(p.wrapping_add(128)));
            assert!((254..=256).contains(&sum), "phase {p}: {sum}");
        }
    }

    #[test]
    fn triangle_key_points() {
        assert_eq!(triangle(0), 0);
        assert_eq!(triangle(127), 254);
        assert_eq!(triangle(128), 255);
        assert_eq!(triangle(255), 1);
    }

    #[test]
    fn atan2_quadrants() {
        let close = |a: u16, b: u16| a.abs_diff(b) <= 182;
        assert_eq!(atan2(0, 10), 0);
        assert!(close(atan2(10, 0), 16384));
        assert!(close(atan2(0, -10), 32768));
        assert!(close(atan2(-10, 0), 49152));
        assert!(close(atan2(10, 10), 8192));
        assert!(close(atan2(-10, 10), 57344));
        assert_eq!(atan2(0, 0), 0);
    }

    #[test]
    fn atan2_is_accurate() {
        for degrees in 0..360 {
            let radians = f64::from(degrees).to_radians();
            let (dx, dy) = (
                (radians.cos() * 10_000.0) as i32,
                (radians.sin() * 10_000.0) as i32,
            );
            let expected = (f64::from(degrees) / 360.0 * 65536.0) as i64;
            let got = i64::from(atan2(dy, dx));
            let diff = (got - expected).rem_euclid(65536);
            let diff = diff.min(65536 - diff);
            assert!(
                diff <= 182,
                "{degrees} degrees: got {got}, expected {expected}"
            );
        }
    }

    #[test]
    fn atan2_large_inputs() {
        assert!(atan2(1 << 20, -(1 << 20)).abs_diff(24576) <= 182);
        let _ = atan2(i32::MAX, i32::MIN);
    }

    #[test]
    fn isqrt_exact_and_floor() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(15), 3);
        assert_eq!(isqrt(16), 4);
        assert_eq!(isqrt(u32::MAX), 65535);
    }

    #[test]
    fn phase_wraps_and_handles_zero() {
        assert_eq!(phase(0, 1000), 0);
        assert_eq!(phase(999, 1000), 255);
        assert_eq!(phase(5, 0), 0);
        assert_eq!(phase16(u32::MAX, 7), phase16(u32::MAX % 7, 7));
    }

    #[test]
    fn hash_spreads() {
        let mut seen = [false; 256];
        for n in 0..256 {
            seen[(hash(n) & 255) as usize] = true;
        }
        assert!(seen.iter().filter(|&&s| s).count() > 140);
    }

    #[test]
    fn noise_is_continuous() {
        for y in (0..2000).step_by(37) {
            for x in 0..1000 {
                assert!(noise(x, y).abs_diff(noise(x + 1, y)) <= 8);
            }
        }
    }

    #[test]
    fn noise_uses_full_range() {
        let (mut low, mut high) = (255, 0);
        for y in 0..64 {
            for x in 0..64 {
                let n = noise(x * 100, y * 100);
                low = low.min(n);
                high = high.max(n);
            }
        }
        assert!(low < 40 && high > 215, "{low}..{high}");
    }
}
