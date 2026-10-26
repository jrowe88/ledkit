//! Power estimation and current limiting.

use crate::pipeline::{process, PipelineSettings};
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

/// The dither offset that gives the brightest frame. Estimating with it
/// makes the limit hold on every dithered frame.
const WORST_CASE_DITHER: u8 = 224;

/// The estimated current, in microamps, of `pixels` after the pipeline runs
/// with `settings` at brightness `brightness` and the brightest dither offset.
pub(crate) fn processed_microamps<P: Pixel>(
    pixels: &[P],
    settings: &PipelineSettings,
    brightness: u8,
    model: PowerModel,
) -> u64 {
    let trial = PipelineSettings {
        correction: settings.correction,
        gamma: settings.gamma,
        brightness,
        dither: WORST_CASE_DITHER,
    };
    total_microamps(pixels.iter().map(|&pixel| process(pixel, &trial)), model)
}

/// Finds the largest brightness (up to the requested one) that keeps the
/// estimated current within `limit`. Returns 0 if even a black strip would
/// exceed the limit. Uses the largest dither offset so the limit holds on
/// every dithered frame.
pub(crate) fn limited_brightness<P: Pixel>(
    pixels: &[P],
    settings: &PipelineSettings,
    model: PowerModel,
    limit: Milliamps,
) -> u8 {
    let requested = settings.brightness;
    let limit_ua = u64::from(limit.0) * 1000;
    let estimate_at = |b: u8| processed_microamps(pixels, settings, b, model);

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
    use crate::pipeline::{scale_channel, DITHER_SEQUENCE};
    use crate::{Rgb, Rgbw};
    use ledkit_color::{GammaTable, GAMMA_2_6};

    fn settings(b: u8) -> PipelineSettings {
        PipelineSettings {
            correction: Rgb::WHITE,
            gamma: None,
            brightness: b,
            dither: 128,
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
                Milliamps(1000)
            ),
            255
        );
        assert_eq!(
            limited_brightness(
                &[Rgb::RED; 30],
                &settings(128),
                PowerModel::TYPICAL_5V,
                Milliamps(500)
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
                Milliamps(5)
            ),
            0
        );
    }
    #[test]
    fn empty_strip_keeps_requested_brightness() {
        let none: [Rgb; 0] = [];
        assert_eq!(
            limited_brightness(&none, &settings(200), PowerModel::TYPICAL_5V, Milliamps(0)),
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
        for limit in [50, 100, 250, 333, 499, 777] {
            for requested in [255, 200, 128, 30] {
                for gamma in gammas {
                    for color in colors_rgb {
                        check_limit(&[color; 24], requested, gamma, limit);
                    }
                    for color in colors_rgbw {
                        check_limit(&[color; 24], requested, gamma, limit);
                    }
                }
            }
        }
    }

    /// Checks the limit holds on every dither offset, and that one step
    /// brighter would break it.
    fn check_limit<P: Pixel>(
        pixels: &[P],
        requested: u8,
        gamma: Option<&'static GammaTable>,
        limit: u32,
    ) {
        let base = PipelineSettings {
            correction: Rgb::WHITE,
            gamma,
            brightness: requested,
            dither: 128,
        };
        let model = PowerModel::TYPICAL_5V;
        let b = limited_brightness(pixels, &base, model, Milliamps(limit));
        assert!(b <= requested);
        let idle_only = pixels.len() as u64 * 1000;
        let allowed = (u64::from(limit) * 1000).max(idle_only);
        for dither in DITHER_SEQUENCE {
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
                processed_microamps(pixels, &base, b + 1, model) > u64::from(limit) * 1000,
                "not maximal: limit {limit} requested {requested} b {b} pixels {:?}",
                pixels[0]
            );
        }
    }
}
