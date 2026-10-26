//! Power estimation and current limiting.

use crate::pipeline::{process, PipelineSettings, DITHER_SEQUENCE};
use crate::{Channel, Pixel};

pub use crate::timing::PowerModel;

/// A current in milliamps (thousandths of an amp).
///
/// Used to say how much current your power supply can safely give.
///
/// # Example
///
/// ```
/// use ledkit_core::power::Milliamps;
///
/// let supply = Milliamps(2000); // a 2 A supply
/// assert!(supply > Milliamps(500));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Milliamps(pub u32);

const CHANNELS: [Channel; 4] = [Channel::Red, Channel::Green, Channel::Blue, Channel::White];

/// Estimates how much current a set of pixels draws, in microamps.
///
/// A typical NeoPixel draws about 20 mA per color channel at full
/// brightness, and about 1 mA even when off. This adds that up for every
/// pixel, using the numbers in the [`PowerModel`]. The result is an
/// estimate, not a measurement.
///
/// # Example
///
/// ```
/// use ledkit_core::power::{estimate_microamps, PowerModel};
/// use ledkit_core::Rgb;
///
/// // Ten white LEDs: 10 * (3 * 20 mA + 1 mA) = 610 mA.
/// let ua = estimate_microamps(&[Rgb::WHITE; 10], PowerModel::TYPICAL_5V);
/// assert_eq!(ua, 610_000);
/// ```
#[must_use]
pub fn estimate_microamps<P: Pixel>(pixels: &[P], model: PowerModel) -> u32 {
    let total = total_microamps(pixels.iter().copied(), model);
    u32::try_from(total).unwrap_or(u32::MAX)
}

/// Adds up the estimated current of some pixels, in microamps.
///
/// This is the one place the power model is applied; both
/// [`estimate_microamps`] and [`limited_brightness`] use it.
fn total_microamps<P: Pixel>(pixels: impl Iterator<Item = P>, model: PowerModel) -> u64 {
    let mut count: u64 = 0;
    let mut sum: u64 = 0;
    for pixel in pixels {
        count += 1;
        for channel in CHANNELS {
            sum += u64::from(pixel.channel(channel));
        }
    }
    count * u64::from(model.idle_microamps) + sum * u64::from(model.channel_full_microamps) / 255
}

/// The largest offset in [`DITHER_SEQUENCE`] (224), which gives the
/// brightest frame. Estimating with it makes the limit hold on every
/// dithered frame.
pub(crate) const WORST_CASE_DITHER: u8 = largest_offset(&DITHER_SEQUENCE);

/// The largest value in `offsets`.
const fn largest_offset(offsets: &[u8]) -> u8 {
    let mut largest = 0;
    let mut i = 0;
    while i < offsets.len() {
        if offsets[i] > largest {
            largest = offsets[i];
        }
        i += 1;
    }
    largest
}

/// The estimated current, in microamps, of `pixels` after the pipeline runs
/// with `settings` at brightness `brightness` and dither offset `dither`.
pub(crate) fn processed_microamps<P: Pixel>(
    pixels: &[P],
    settings: &PipelineSettings,
    brightness: u8,
    model: PowerModel,
    dither: u8,
) -> u64 {
    let trial = PipelineSettings {
        correction: settings.correction,
        gamma: settings.gamma,
        brightness,
        dither,
    };
    total_microamps(pixels.iter().map(|&pixel| process(pixel, &trial)), model)
}

/// Finds the largest brightness (up to the requested one) that keeps the
/// estimated current within `limit`. Returns 0 if even a black strip would
/// exceed the limit.
///
/// `dither` is the offset to assume: pass [`WORST_CASE_DITHER`] when
/// dithering is on, so the limit holds on every dithered frame, and
/// [`NO_DITHER`](crate::pipeline::NO_DITHER) when it is off.
pub(crate) fn limited_brightness<P: Pixel>(
    pixels: &[P],
    settings: &PipelineSettings,
    model: PowerModel,
    limit: Milliamps,
    dither: u8,
) -> u8 {
    let requested = settings.brightness;
    let limit_ua = u64::from(limit.0) * 1000;
    let estimate_at = |b: u8| processed_microamps(pixels, settings, b, model, dither);

    if pixels.is_empty() {
        return requested;
    }
    let full = estimate_at(requested);
    if full <= limit_ua {
        return requested;
    }
    let idle = pixels.len() as u64 * u64::from(model.idle_microamps);
    if limit_ua < idle {
        return 0;
    }
    // Here full > limit_ua >= idle, so the channel part is non-zero.
    let channel_part = full - idle;
    let available = limit_ua - idle;
    let guess = u64::from(requested) * available / channel_part;
    #[allow(clippy::cast_possible_truncation)] // clamped to a u8 first
    let mut b = guess.min(u64::from(requested)) as u8;
    // The guess is close but not exact (rounding, gamma and correction), so
    // step down until it fits, then up while the next level still fits.
    while b > 0 && estimate_at(b) > limit_ua {
        b -= 1;
    }
    while b < requested && estimate_at(b + 1) <= limit_ua {
        b += 1;
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{scale_channel, DITHER_SEQUENCE, NO_DITHER};
    use crate::{Rgb, Rgbw};
    use ledkit_color::{GammaTable, GAMMA_2_6};

    fn settings(b: u8) -> PipelineSettings {
        PipelineSettings {
            correction: Rgb::WHITE,
            gamma: None,
            brightness: b,
            dither: NO_DITHER,
        }
    }
    #[test]
    fn estimate() {
        assert_eq!(
            estimate_microamps(&[Rgb::WHITE; 10], PowerModel::TYPICAL_5V),
            610_000
        );
        assert_eq!(
            estimate_microamps(&[Rgbw::new(0, 0, 0, 255); 1], PowerModel::TYPICAL_5V),
            21_000
        );
    }
    #[test]
    fn limit_scales_down_white() {
        let b = limited_brightness(
            &[Rgb::WHITE; 10],
            &settings(255),
            PowerModel::TYPICAL_5V,
            Milliamps(300),
            WORST_CASE_DITHER,
        );
        assert_eq!(b, 122);
        assert_eq!(scale_channel(255, b, 128), 123);
        let processed = [process(Rgb::WHITE, &settings(b)); 10];
        assert!(estimate_microamps(&processed, PowerModel::TYPICAL_5V) <= 300_000);
    }
    #[test]
    fn under_the_limit_is_untouched() {
        assert_eq!(
            limited_brightness(
                &[Rgb::WHITE; 10],
                &settings(255),
                PowerModel::TYPICAL_5V,
                Milliamps(1000),
                WORST_CASE_DITHER
            ),
            255
        );
        assert_eq!(
            limited_brightness(
                &[Rgb::RED; 30],
                &settings(128),
                PowerModel::TYPICAL_5V,
                Milliamps(500),
                NO_DITHER
            ),
            128
        );
    }
    #[test]
    fn limit_below_idle_current_is_black() {
        assert_eq!(
            limited_brightness(
                &[Rgb::WHITE; 10],
                &settings(255),
                PowerModel::TYPICAL_5V,
                Milliamps(5),
                WORST_CASE_DITHER
            ),
            0
        );
    }
    #[test]
    fn empty_strip_keeps_requested_brightness() {
        let none: [Rgb; 0] = [];
        assert_eq!(
            limited_brightness(
                &none,
                &settings(200),
                PowerModel::TYPICAL_5V,
                Milliamps(0),
                NO_DITHER
            ),
            200
        );
    }
    #[test]
    fn never_exceeds_limit_and_is_the_largest_that_fits() {
        let gammas: [Option<&'static GammaTable>; 2] = [None, Some(&GAMMA_2_6)];
        let colors_rgb = [Rgb::WHITE, Rgb::RED, Rgb::new(10, 200, 90)];
        let colors_rgbw = [
            Rgbw::new(255, 255, 255, 255),
            Rgbw::new(0, 0, 0, 255),
            Rgbw::new(10, 200, 90, 40),
        ];
        for dithering in [true, false] {
            for limit in [50, 100, 250, 333, 499, 777] {
                for requested in [255, 200, 128, 30] {
                    for gamma in gammas {
                        for color in colors_rgb {
                            check_limit(&[color; 24], requested, gamma, limit, dithering);
                        }
                        for color in colors_rgbw {
                            check_limit(&[color; 24], requested, gamma, limit, dithering);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn worst_case_dither_is_the_largest_offset() {
        assert_eq!(WORST_CASE_DITHER, 224);
        assert!(DITHER_SEQUENCE.iter().all(|&d| d <= WORST_CASE_DITHER));
    }
    #[test]
    fn dithering_off_allows_a_brighter_limit() {
        // Ten white LEDs and 363 mA: only 150 per channel fits. Plain
        // rounding reaches 150 at brightness 150, but the brightest dither
        // offset already rounds brightness 150 up to 151.
        let pixels = [Rgb::WHITE; 10];
        let model = PowerModel::TYPICAL_5V;
        let on = limited_brightness(
            &pixels,
            &settings(255),
            model,
            Milliamps(363),
            WORST_CASE_DITHER,
        );
        let off = limited_brightness(&pixels, &settings(255), model, Milliamps(363), NO_DITHER);
        assert_eq!((on, off), (149, 150));
        assert!(off > on);
    }

    /// Checks the limit holds on every frame (every dither offset when
    /// dithering, plain rounding when not), and that one step brighter
    /// would break it.
    fn check_limit<P: Pixel>(
        pixels: &[P],
        requested: u8,
        gamma: Option<&'static GammaTable>,
        limit: u32,
        dithering: bool,
    ) {
        let base = PipelineSettings {
            correction: Rgb::WHITE,
            gamma,
            brightness: requested,
            dither: NO_DITHER,
        };
        let model = PowerModel::TYPICAL_5V;
        let assumed = if dithering {
            WORST_CASE_DITHER
        } else {
            NO_DITHER
        };
        let b = limited_brightness(pixels, &base, model, Milliamps(limit), assumed);
        assert!(b <= requested);
        let idle_only = pixels.len() as u64 * 1000;
        let allowed = (u64::from(limit) * 1000).max(idle_only);
        let offsets: &[u8] = if dithering {
            &DITHER_SEQUENCE
        } else {
            &[NO_DITHER]
        };
        for &dither in offsets {
            let frame = PipelineSettings {
                correction: base.correction,
                gamma,
                brightness: b,
                dither,
            };
            let used = total_microamps(pixels.iter().map(|&p| process(p, &frame)), model);
            assert!(
                used <= allowed,
                "limit {limit} requested {requested} dither {dither} pixels {:?}",
                pixels[0]
            );
        }
        if b < requested {
            assert!(
                processed_microamps(pixels, &base, b + 1, model, assumed) > u64::from(limit) * 1000,
                "not maximal: limit {limit} requested {requested} b {b} dithering {dithering} pixels {:?}",
                pixels[0]
            );
        }
    }
}
