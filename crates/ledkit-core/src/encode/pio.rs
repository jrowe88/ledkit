//! Wire bytes packed into 32-bit words, for shift-register peripherals such as
//! the RP2040 / RP2350 PIO.

use crate::BufferTooSmall;

/// How many 32-bit words [`fill_pio_words`] writes for `wire_bytes` bytes,
/// `bytes_per_word` to a word: `wire_bytes / bytes_per_word`, rounded up.
///
/// A `bytes_per_word` of 0 is treated as 1, and above 4 as 4.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::pio_word_count;
///
/// // 10 RGB LEDs, one LED (3 bytes) per word.
/// assert_eq!(pio_word_count(30, 3), 10);
/// assert_eq!(pio_word_count(31, 3), 11);
/// ```
#[must_use]
pub const fn pio_word_count(wire_bytes: usize, bytes_per_word: usize) -> usize {
    wire_bytes.div_ceil(clamp_bytes_per_word(bytes_per_word))
}

const fn clamp_bytes_per_word(bytes_per_word: usize) -> usize {
    if bytes_per_word == 0 {
        1
    } else if bytes_per_word > 4 {
        4
    } else {
        bytes_per_word
    }
}

/// Packs wire bytes into 32-bit words, `bytes_per_word` to a word, most
/// significant byte first, and returns how many words it wrote.
///
/// The bytes sit at the top of each word (`b0 << 24 | b1 << 16 | ...`), which is
/// the order a PIO state machine shifting **left** sends them in. With one LED
/// per word (`bytes_per_word` = the chip's channel count, and an autopull
/// threshold of 8 bits per channel), each word is exactly one LED. A last word
/// with fewer bytes is padded with zero bits on the right.
///
/// A `bytes_per_word` of 0 is treated as 1, and above 4 as 4.
///
/// # Errors
///
/// Returns [`BufferTooSmall`] if `out` can't hold [`pio_word_count`] words. The
/// bytes are still all counted, so the error says exactly how big `out` must be,
/// and `out` must not be sent.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::fill_pio_words;
///
/// let mut out = [0u32; 2];
/// // Two RGB LEDs, one per word.
/// let used = fill_pio_words([1, 2, 3, 4, 5, 6], 3, &mut out).unwrap();
/// assert_eq!(used, 2);
/// assert_eq!(out, [0x0102_0300, 0x0405_0600]);
/// ```
pub fn fill_pio_words(
    wire: impl IntoIterator<Item = u8>,
    bytes_per_word: usize,
    out: &mut [u32],
) -> Result<usize, BufferTooSmall> {
    let mut words = 0usize;
    for word in pio_words(wire, bytes_per_word) {
        if let Some(slot) = out.get_mut(words) {
            *slot = word;
        }
        words += 1;
    }
    if out.len() < words {
        return Err(BufferTooSmall { needed: words });
    }
    Ok(words)
}

/// The words of [`fill_pio_words`], one at a time, without a buffer.
///
/// A processor feeding a PIO state machine's FIFO directly uses this: it packs
/// each word just before pushing it. The words are the same as
/// [`fill_pio_words`] writes.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::pio_words;
///
/// let mut words = pio_words([1, 2, 3, 4, 5, 6, 7], 3);
/// assert_eq!(words.next(), Some(0x0102_0300));
/// assert_eq!(words.next(), Some(0x0405_0600));
/// assert_eq!(words.next(), Some(0x0700_0000));
/// assert_eq!(words.next(), None);
/// ```
pub fn pio_words(
    wire: impl IntoIterator<Item = u8>,
    bytes_per_word: usize,
) -> impl Iterator<Item = u32> {
    let per_word = clamp_bytes_per_word(bytes_per_word);
    let mut bytes = wire.into_iter();
    core::iter::from_fn(move || {
        let mut word = u32::from(bytes.next()?) << 24;
        for i in 1..per_word {
            match bytes.next() {
                Some(byte) => word |= u32::from(byte) << (24 - 8 * i),
                None => break,
            }
        }
        Some(word)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{wire, Rgb, Rgbw, Sk6812Rgbw, Ws2812b};

    #[test]
    fn rgb_pixels_pack_one_per_word_at_the_top() {
        let pixels = [Rgb::new(0x11, 0x22, 0x33), Rgb::new(0xAA, 0xBB, 0xCC)];
        let mut out = [0u32; 2];
        let used =
            fill_pio_words(wire::clockless_bytes::<Ws2812b, _>(pixels), 3, &mut out).unwrap();
        assert_eq!(used, 2);
        // The WS2812B wants green, red, blue.
        assert_eq!(out, [0x2211_3300, 0xBBAA_CC00]);
    }

    #[test]
    fn rgbw_pixels_fill_all_32_bits() {
        let pixels = [Rgbw::new(0x11, 0x22, 0x33, 0x44)];
        let mut out = [0u32; 1];
        let used =
            fill_pio_words(wire::clockless_bytes::<Sk6812Rgbw, _>(pixels), 4, &mut out).unwrap();
        assert_eq!(used, 1);
        assert_eq!(out, [0x2211_3344]);
    }

    #[test]
    fn a_short_last_word_is_padded_with_zero_bits() {
        let mut out = [0xFFFF_FFFF; 2];
        assert_eq!(fill_pio_words([1, 2, 3, 4, 5], 4, &mut out), Ok(2));
        assert_eq!(out, [0x0102_0304, 0x0500_0000]);
    }

    #[test]
    fn too_small_a_buffer_reports_the_size_needed() {
        let mut out = [0u32; 1];
        assert_eq!(
            fill_pio_words([0; 9], 3, &mut out),
            Err(BufferTooSmall { needed: 3 })
        );
        let mut empty: [u32; 0] = [];
        assert_eq!(
            fill_pio_words([7], 3, &mut empty),
            Err(BufferTooSmall { needed: 1 })
        );
    }

    #[test]
    fn no_bytes_means_no_words() {
        let mut out: [u32; 0] = [];
        assert_eq!(fill_pio_words([], 3, &mut out), Ok(0));
        assert_eq!(pio_word_count(0, 3), 0);
    }

    #[test]
    fn word_count_matches_what_is_written() {
        for per_word in 0..=6 {
            for bytes in 0..20 {
                let mut out = [0u32; 32];
                let used = fill_pio_words(core::iter::repeat_n(9, bytes), per_word, &mut out);
                assert_eq!(used, Ok(pio_word_count(bytes, per_word)));
            }
        }
    }

    #[test]
    fn the_iterator_matches_the_buffer() {
        let bytes = [9u8, 8, 7, 6, 5, 4, 3, 2, 1];
        for per_word in 1..=4 {
            let mut out = [0u32; 9];
            let used = fill_pio_words(bytes, per_word, &mut out).unwrap();
            let words: Vec<u32> = pio_words(bytes, per_word).collect();
            assert_eq!(words, &out[..used]);
        }
    }

    #[test]
    fn odd_bytes_per_word_are_clamped() {
        let mut out = [0u32; 3];
        assert_eq!(fill_pio_words([1, 2], 0, &mut out), Ok(2));
        assert_eq!(&out[..2], &[0x0100_0000, 0x0200_0000]);
        assert_eq!(fill_pio_words([1, 2, 3, 4, 5], 9, &mut out), Ok(2));
        assert_eq!(&out[..2], &[0x0102_0304, 0x0500_0000]);
    }
}
