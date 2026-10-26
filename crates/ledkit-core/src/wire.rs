//! Turning pixels into the bytes an LED chip expects on the wire.
//!
//! "Wire order" is the order in which a chip wants its color values. A
//! WS2812B LED, for example, expects green first, then red, then blue, so the
//! pixel `Rgb::new(1, 2, 3)` goes out as `2, 1, 3`. Both functions here hand
//! back lazy iterators: nothing is allocated, and bytes are made on demand.

use crate::Chip;
use crate::Pixel;

/// Yields the bytes for a clockless chip (WS2812B, SK6812 and friends).
///
/// Each pixel becomes its channels in the chip's wire order, one byte per
/// channel. Use this for clockless chips; [`clocked_frame`] is for clocked
/// ones. Using the wrong one does not panic, it just gives bytes the chip
/// will not understand.
///
/// # Example
///
/// ```
/// use ledkit_core::{wire::clockless_bytes, Rgb, Ws2812b};
///
/// let bytes: Vec<u8> = clockless_bytes::<Ws2812b>([Rgb::new(1, 2, 3)]).collect();
/// assert_eq!(bytes, [2, 1, 3]); // green, red, blue
/// ```
pub fn clockless_bytes<C: Chip>(
    pixels: impl IntoIterator<Item = C::Pixel>,
) -> impl Iterator<Item = u8> {
    pixels
        .into_iter()
        .flat_map(|pixel| (0..C::ORDER.len()).map(move |i| pixel.channel(C::ORDER.channels()[i])))
}

/// How many bytes [`clocked_frame`] produces for `led_count` LEDs.
///
/// That is a 4-byte start frame, 4 bytes per LED, a 4-byte end frame and one
/// extra byte for every 16 LEDs (rounded up) so the last LEDs get clocked.
///
/// # Example
///
/// ```
/// use ledkit_core::wire::clocked_frame_len;
///
/// assert_eq!(clocked_frame_len(2), 17);
/// ```
pub const fn clocked_frame_len(led_count: usize) -> usize {
    4 + 4 * led_count + 4 + led_count.div_ceil(16)
}

/// Yields a complete DotStar (APA102/SK9822) frame for a clocked chip.
///
/// The frame is a start frame of four `0x00` bytes, then for each LED a
/// `0xFF` byte (full global brightness) followed by its channels in the
/// chip's wire order, then an end frame of zeros. Use this for clocked
/// chips; [`clockless_bytes`] is for clockless ones. Using the wrong one does
/// not panic. The iterator must know its length up front, because the end
/// frame depends on the LED count.
///
/// # Example
///
/// ```
/// use ledkit_core::{wire::clocked_frame, Apa102, Rgb};
///
/// let frame: Vec<u8> = clocked_frame::<Apa102, _>([Rgb::new(1, 2, 3)]).collect();
/// assert_eq!(frame, [0, 0, 0, 0, 0xFF, 3, 2, 1, 0, 0, 0, 0, 0]);
/// ```
pub fn clocked_frame<C: Chip, I>(pixels: I) -> impl Iterator<Item = u8>
where
    I: IntoIterator<Item = C::Pixel>,
    I::IntoIter: ExactSizeIterator,
{
    let pixels = pixels.into_iter();
    let led_count = pixels.len();
    let start = core::iter::repeat_n(0x00, 4);
    let body = pixels.flat_map(|pixel| {
        core::iter::once(0xFF)
            .chain((0..C::ORDER.len()).map(move |i| pixel.channel(C::ORDER.channels()[i])))
    });
    let end = core::iter::repeat_n(0x00, 4 + led_count.div_ceil(16));
    start.chain(body).chain(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Apa102, Rgb, Rgbw, Sk6812Rgbw, Ws2812b};

    #[test]
    fn grb_wire_order() {
        let bytes =
            clockless_bytes::<Ws2812b>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]).collect::<Vec<_>>();
        assert_eq!(bytes, [2, 1, 3, 5, 4, 6]);
    }

    #[test]
    fn rgbw_wire_order() {
        let mut it = clockless_bytes::<Sk6812Rgbw>([Rgbw::new(1, 2, 3, 4)]);
        assert_eq!(
            [it.next(), it.next(), it.next(), it.next(), it.next()],
            [Some(2), Some(1), Some(3), Some(4), None]
        );
    }

    #[test]
    fn dotstar_frame() {
        let frame =
            clocked_frame::<Apa102, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]).collect::<Vec<_>>();
        assert_eq!(
            frame,
            [0, 0, 0, 0, 0xFF, 3, 2, 1, 0xFF, 6, 5, 4, 0, 0, 0, 0, 0]
        );
        assert_eq!(clocked_frame_len(2), 17);
    }

    #[test]
    fn dotstar_frame_lengths() {
        assert_eq!(clocked_frame_len(0), 8);
        assert_eq!(clocked_frame_len(16), 4 + 64 + 4 + 1);
        assert_eq!(clocked_frame_len(17), 4 + 68 + 4 + 2);
    }
}
