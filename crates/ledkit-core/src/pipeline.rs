//! The per-pixel processing pipeline.

use crate::{Channel, Pixel};
use ledkit_color::{GammaTable, Rgb};

/// Offsets used to dither brightness scaling over eight frames.
///
/// Scaling a value down rarely gives a whole number. Adding a different
/// offset each frame makes the result round up some frames and down others,
/// so the average over time is closer to the true value. The offset 128 is
/// used when dithering is off (plain rounding).
///
/// # Example
///
/// ```
/// use ledkit_core::pipeline::DITHER_SEQUENCE;
///
/// assert_eq!(DITHER_SEQUENCE.len(), 8);
/// ```
pub const DITHER_SEQUENCE: [u8; 8] = [0, 128, 64, 192, 32, 160, 96, 224];

/// The offset used when dithering is off: plain rounding.
pub(crate) const NO_DITHER: u8 = 128;

/// Scales one channel by a brightness level, adding a dither offset.
pub(crate) const fn scale_channel(value: u8, brightness: u8, dither: u8) -> u8 {
    if brightness == 0 {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)] // result is at most 255
        let out = ((value as u16 * (brightness as u16 + 1) + dither as u16) >> 8) as u8;
        out
    }
}

/// The brightness below which HD brightness starts lowering the DotStar
/// global level. Above it, the channels alone have enough steps.
pub(crate) const HD_KNEE: u8 = 64;

/// Splits a brightness level between a DotStar's 5-bit global brightness and
/// its 8-bit channels, for [`Strip::set_hd_brightness`](crate::Strip::set_hd_brightness).
///
/// Returns `(global, channel)`, chosen so the channels come out as bright as
/// [`scale_channel`] makes them without HD: `scale_channel` multiplies by
/// `(brightness + 1) / 256`, so the split keeps `global / 31 * (channel + 1)`
/// close to `brightness + 1`. From [`HD_KNEE`] up the global level stays at 31, as
/// without HD, because a DotStar's global level doesn't scale its light
/// exactly in proportion (an APA102-2020 comes out dimmer at middle levels
/// than the same brightness made with the channels). Below the knee the
/// global level drops just enough to keep the channel scale at about
/// `HD_KNEE` or more, so dim colors keep their steps. 0 gives `(31, 0)`.
pub(crate) const fn hd_split(brightness: u8) -> (u8, u8) {
    if brightness == 0 {
        return (31, 0);
    }
    // The multiplier scale_channel uses, in 256ths, times 31.
    let wanted = (brightness as u16 + 1) * 31;
    let global = wanted.div_ceil(HD_KNEE as u16 + 1);
    let global = if global > 31 { 31 } else { global };
    // wanted >= 62 and global <= wanted / 2, so this is at least 1.
    let channel = (wanted + global / 2) / global - 1;
    #[allow(clippy::cast_possible_truncation)] // global <= 31, channel <= 255
    (
        global as u8,
        if channel > 255 { 255 } else { channel as u8 },
    )
}

/// Everything the pipeline needs to process one pixel.
pub(crate) struct PipelineSettings {
    /// White-balance correction for the R, G and B channels.
    pub correction: Rgb,
    /// Optional gamma table applied to every channel.
    pub gamma: Option<&'static GammaTable>,
    /// Raw brightness level (callers pass `Brightness::output_level()`).
    pub brightness: u8,
    /// Dither offset for this frame.
    pub dither: u8,
}

/// Runs a pixel through correction, gamma and brightness, in that order.
pub(crate) fn process<P: Pixel>(pixel: P, s: &PipelineSettings) -> P {
    let rgb = Rgb::new(
        pixel.channel(Channel::Red),
        pixel.channel(Channel::Green),
        pixel.channel(Channel::Blue),
    )
    .corrected(s.correction);
    pixel.map_channels(|channel, value| {
        let mut v = match channel {
            Channel::Red => rgb.r,
            Channel::Green => rgb.g,
            Channel::Blue => rgb.b,
            Channel::White => value,
        };
        if let Some(table) = s.gamma {
            v = table.apply(v);
        }
        scale_channel(v, s.brightness, s.dither)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rgbw;
    use ledkit_color::Rgb;

    #[test]
    fn full_brightness_is_lossless() {
        for v in 0..=255u8 {
            for d in DITHER_SEQUENCE {
                assert_eq!(scale_channel(v, 255, d), v);
            }
            assert_eq!(scale_channel(v, 255, 128), v);
        }
    }
    #[test]
    fn zero_brightness_is_black() {
        for d in DITHER_SEQUENCE {
            assert_eq!(scale_channel(255, 0, d), 0);
        }
    }
    #[test]
    fn rounding_without_dither() {
        assert_eq!(scale_channel(200, 128, 128), 101);
        assert_eq!(scale_channel(10, 20, 128), 1);
    }
    #[test]
    fn dither_sequences() {
        assert_eq!(
            DITHER_SEQUENCE.map(|d| scale_channel(200, 128, d)),
            [100, 101, 101, 101, 100, 101, 101, 101]
        );
        assert_eq!(
            DITHER_SEQUENCE.map(|d| scale_channel(10, 20, d)),
            [0, 1, 1, 1, 0, 1, 1, 1]
        );
    }
    #[test]
    fn hd_split_keeps_the_light_and_gains_steps() {
        assert_eq!(hd_split(0), (31, 0));
        assert_eq!(hd_split(255), (31, 255));
        assert_eq!(hd_split(3), (2, 61));
        assert_eq!(hd_split(1), (1, 61));
        assert_eq!(hd_split(HD_KNEE - 1), (31, HD_KNEE - 1));
        for b in 1..=255u8 {
            let (global, channel) = hd_split(b);
            assert!((1..=31).contains(&global));
            // What the LEDs show, weighted by the global level, stays close
            // to the exact light the pipeline aims for: within the rounding
            // of one channel step at that global level.
            let weight = f32::from(global) / 31.0;
            for v in 0..=255u8 {
                let exact = f32::from(v) * (f32::from(b) + 1.0) / 256.0;
                let hd = f32::from(scale_channel(v, channel, NO_DITHER)) * weight;
                assert!(
                    (hd - exact).abs() <= weight + 0.001,
                    "{b} {v}: {global} {channel}"
                );
            }
            // And the channels get at least as many steps as before.
            assert!(channel >= b, "{b}: {channel}");
            // At and above the knee nothing changes; below it the channels
            // keep at least about half the knee's steps.
            if b >= HD_KNEE {
                assert_eq!((global, channel), (31, b));
            } else {
                assert!(channel >= HD_KNEE / 2 - 1, "{b}: {channel}");
            }
        }
    }
    #[test]
    fn pipeline_defaults_are_identity() {
        let s = PipelineSettings {
            correction: Rgb::WHITE,
            gamma: None,
            brightness: 255,
            dither: 128,
        };
        assert_eq!(process(Rgb::new(12, 34, 56), &s), Rgb::new(12, 34, 56));
    }
    #[test]
    fn pipeline_applies_gamma_then_brightness() {
        let s = PipelineSettings {
            correction: Rgb::WHITE,
            gamma: Some(&ledkit_color::GAMMA_2_6),
            brightness: 255,
            dither: 128,
        };
        assert_eq!(process(Rgb::new(128, 255, 0), &s), Rgb::new(42, 255, 0));
    }
    #[test]
    fn correction_skips_white() {
        let s = PipelineSettings {
            correction: Rgb::new(255, 176, 240),
            gamma: None,
            brightness: 255,
            dither: 128,
        };
        assert_eq!(
            process(Rgbw::new(255, 255, 255, 255), &s),
            Rgbw::new(255, 176, 240, 255)
        );
    }
}
