//! Pulse words for RMT-style peripherals (the ESP32 family).

use super::pulse::{BitPulse, PulseTicks};
use crate::BufferTooSmall;

/// The longest pulse half an RMT word can hold, in ticks (15 bits).
const MAX_HALF: u32 = 0x7FFF;

/// Bit 15 of an RMT word: the level of its first half. The second half's
/// level bit is bit 31; ledkit only ever sends the second half low.
const FIRST_HALF_HIGH: u32 = 1 << 15;

/// The longest reset one word can carry: two of the longest halves.
const MAX_RESET_TICKS: u32 = 2 * MAX_HALF;

/// The word that ends a transmission: a zero-length half.
const END_MARKER: u32 = 0;

const fn clamp_half(ticks: u32) -> u32 {
    if ticks > MAX_HALF {
        MAX_HALF
    } else {
        ticks
    }
}

/// The RMT word for one LED bit: the line is high for `pulse.high` ticks,
/// then low for `pulse.low` ticks.
///
/// The layout is the one the ESP32 RMT hardware reads (and `esp-hal`'s
/// `PulseCode` wraps): bits 0 to 14 are the length of the first half and bit
/// 15 is its level (1 is high); bits 16 to 30 are the length of the second
/// half and bit 31 is its level. Lengths above 32 767 are cut to 32 767. A
/// length of 0 would end the transmission, so keep both non-zero; the
/// `max_ticks` of [`ClocklessTiming::pulse_ticks`](crate::ClocklessTiming::pulse_ticks)
/// takes care of the upper limit.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::{rmt_bit_word, BitPulse};
///
/// // High for 8 ticks, then low for 6 ticks.
/// assert_eq!(rmt_bit_word(BitPulse { high: 8, low: 6 }), 0x0006_8008);
/// ```
#[must_use]
pub const fn rmt_bit_word(pulse: BitPulse) -> u32 {
    let high = clamp_half(pulse.high as u32);
    let low = clamp_half(pulse.low as u32);
    FIRST_HALF_HIGH | high | (low << 16)
}

/// The RMT word that holds the line low for the reset gap: two low halves
/// that add up to `reset_ticks`.
///
/// One word holds at most 65 534 ticks (two halves of 32 767), and a half of
/// 0 would end the transmission early, so `reset_ticks` is brought into the
/// range 2 to 65 534. At 10 MHz that is 0.2 us to 6.5 ms, which covers every
/// chip in the catalogue.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::rmt_reset_word;
///
/// // 2 800 ticks (280 us at 10 MHz) as two low halves of 1 400.
/// assert_eq!(rmt_reset_word(2800), (1400 << 16) | 1400);
/// ```
#[must_use]
pub const fn rmt_reset_word(reset_ticks: u32) -> u32 {
    let total = if reset_ticks < 2 {
        2
    } else if reset_ticks > MAX_RESET_TICKS {
        MAX_RESET_TICKS
    } else {
        reset_ticks
    };
    let first = total / 2;
    let second = total - first;
    first | (second << 16)
}

/// How many words [`fill_rmt_words`] writes for `byte_count` wire bytes: one
/// per bit, plus the reset word and the end marker.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::rmt_word_count;
///
/// assert_eq!(rmt_word_count(3), 26);
/// ```
#[must_use]
pub const fn rmt_word_count(byte_count: usize) -> usize {
    byte_count * 8 + 2
}

/// Works out the RMT clock divider that turns the RMT's input clock
/// (`rmt_hz`) into ticks of `tick_hz`.
///
/// Returns `None` unless the division is exact and the divider fits the
/// hardware's range of 1 to 255. An inexact divider would make every pulse
/// the wrong length.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::rmt_clock_divider;
///
/// assert_eq!(rmt_clock_divider(80_000_000, 10_000_000), Some(8));
/// assert_eq!(rmt_clock_divider(32_000_000, 10_000_000), None); // not exact
/// assert_eq!(rmt_clock_divider(5_000_000, 10_000_000), None); // too slow
/// ```
#[must_use]
pub const fn rmt_clock_divider(rmt_hz: u32, tick_hz: u32) -> Option<u8> {
    if tick_hz == 0 || rmt_hz % tick_hz != 0 {
        return None;
    }
    let divider = rmt_hz / tick_hz;
    if divider == 0 || divider > 255 {
        None
    } else {
        Some(divider as u8)
    }
}

/// Fills `out` with the RMT words of a clockless frame: one word per bit of
/// `wire` (most significant bit first), then the reset as a low word, then an
/// end marker. Returns how many words it wrote.
///
/// `T` is whatever the peripheral takes a word as. It only needs
/// `From<u32>`, so it is `u32` for plain words and `esp-hal`'s `PulseCode`
/// for `ledkit-esp`. The word layout is described at [`rmt_bit_word`].
///
/// The wire bytes can be of any length. If `out` is too short the function
/// still counts them, so the error says exactly how big `out` must be.
///
/// # Errors
///
/// Returns [`BufferTooSmall`] if `out` can't hold [`rmt_word_count`] words;
/// `out` is then partly written and must not be sent.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::{fill_rmt_words, rmt_bit_word, rmt_reset_word};
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let ticks = ClocklessTiming::of::<Ws2812b>().pulse_ticks(10_000_000, 32_767)?;
/// let mut out = [0u32; 10];
/// let used = fill_rmt_words(&ticks, [0b1000_0000u8], &mut out).unwrap();
///
/// assert_eq!(used, 10);
/// assert_eq!(out[0], rmt_bit_word(ticks.one_pulse()));
/// assert_eq!(out[1], rmt_bit_word(ticks.zero_pulse()));
/// assert_eq!(out[8], rmt_reset_word(ticks.reset));
/// assert_eq!(out[9], 0); // the end marker
/// # Ok::<(), ledkit_core::TimingError>(())
/// ```
pub fn fill_rmt_words<T: From<u32>>(
    ticks: &PulseTicks,
    wire: impl IntoIterator<Item = u8>,
    out: &mut [T],
) -> Result<usize, BufferTooSmall> {
    let mut count = 0usize;
    for pulse in ticks.bit_pulses(wire) {
        if let Some(slot) = out.get_mut(count) {
            *slot = T::from(rmt_bit_word(pulse));
        }
        count += 1;
    }
    let needed = count + 2;
    if out.len() < needed {
        return Err(BufferTooSmall { needed });
    }
    out[count] = T::from(rmt_reset_word(ticks.reset));
    out[count + 1] = T::from(END_MARKER);
    Ok(needed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Apa106, Chip, ClocklessTiming, Sk6812, Sk6812Rgbw, Ws2811, Ws2811Slow, Ws2812, Ws2812b,
        Ws2813, Ws2815,
    };

    // The esp-hal layout, written out independently of the implementation.
    fn word(level1: bool, len1: u16, level2: bool, len2: u16) -> u32 {
        u32::from(len1)
            | (u32::from(level1) << 15)
            | (u32::from(len2) << 16)
            | (u32::from(level2) << 31)
    }

    fn ws_ticks() -> PulseTicks {
        ClocklessTiming::of::<Ws2812b>()
            .pulse_ticks(10_000_000, 32_767)
            .unwrap()
    }

    #[test]
    fn fills_one_word_per_bit_then_reset_then_end() {
        let t = ws_ticks();
        let mut out = [0u32; 26];
        let n = fill_rmt_words(&t, [0xFFu8, 0x00, 0x80], &mut out).unwrap();
        assert_eq!(n, 26);
        assert_eq!(out[0], word(true, 8, false, 6)); // first `1` bit
        assert_eq!(out[7], word(true, 8, false, 6));
        assert_eq!(out[8], word(true, 4, false, 9)); // first `0` bit
        assert_eq!(out[16], word(true, 8, false, 6)); // 0x80 starts with a 1
        assert_eq!(out[17], word(true, 4, false, 9));
        assert_eq!(out[24], rmt_reset_word(t.reset));
        assert_eq!(out[25], 0); // end marker
    }

    fn check_reset(reset: u32) {
        let w = rmt_reset_word(reset);
        let (a, b) = (w & 0x7FFF, (w >> 16) & 0x7FFF);
        assert!(a > 0 && b > 0, "reset {reset}: halves {a} and {b}");
        assert_eq!(w & (1 << 15), 0, "first half must be low");
        assert_eq!(w >> 31, 0, "second half must be low");
        assert_eq!(a + b, reset.clamp(2, 65_534), "reset {reset}");
    }

    fn check_chip<C: Chip>() {
        let t = ClocklessTiming::of::<C>()
            .pulse_ticks(10_000_000, 32_767)
            .unwrap();
        check_reset(t.reset);
    }

    #[test]
    fn reset_word_never_has_a_zero_half() {
        check_chip::<Ws2812>();
        check_chip::<Ws2812b>();
        check_chip::<Ws2813>();
        check_chip::<Ws2815>();
        check_chip::<Ws2811>();
        check_chip::<Ws2811Slow>();
        check_chip::<Sk6812>();
        check_chip::<Sk6812Rgbw>();
        check_chip::<Apa106>();
        for r in [0, 1, 2, 3, 32_767, 32_768, 65_534, 65_535, u32::MAX] {
            check_reset(r);
        }
    }

    #[test]
    fn too_small_buffer_reports_needed() {
        let t = ws_ticks();
        let mut out = [0u32; 10];
        let err = fill_rmt_words(&t, [1u8, 2, 3], &mut out).unwrap_err();
        assert_eq!(
            err,
            BufferTooSmall {
                needed: rmt_word_count(3)
            }
        );
        let mut out = [0u32; 25];
        assert_eq!(
            fill_rmt_words(&t, [1u8, 2, 3], &mut out),
            Err(BufferTooSmall { needed: 26 })
        );
        let mut out = [0u32; 26];
        assert_eq!(fill_rmt_words(&t, [1u8, 2, 3], &mut out), Ok(26));
    }

    #[test]
    fn zero_leds_is_reset_and_end_only() {
        let t = ws_ticks();
        let mut out = [7u32; 4];
        let n = fill_rmt_words(&t, core::iter::empty(), &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0], rmt_reset_word(t.reset));
        assert_eq!(out[1], 0);
    }

    #[test]
    fn works_with_any_from_u32_type() {
        #[derive(Clone, Copy, Default, PartialEq, Debug)]
        struct Code(u32);
        impl From<u32> for Code {
            fn from(v: u32) -> Self {
                Self(v)
            }
        }
        let t = ws_ticks();
        let mut out = [Code::default(); 10];
        fill_rmt_words(&t, [0u8], &mut out).unwrap();
        assert_eq!(out[0], Code(word(true, 4, false, 9)));
    }

    #[test]
    fn divider_must_be_exact() {
        let hz = 10_000_000;
        assert_eq!(rmt_clock_divider(80_000_000, hz), Some(8));
        assert_eq!(rmt_clock_divider(10_000_000, hz), Some(1));
        assert_eq!(rmt_clock_divider(32_000_000, hz), None);
        assert_eq!(rmt_clock_divider(0, hz), None);
        assert_eq!(rmt_clock_divider(15_000_000, hz), None);
        assert_eq!(rmt_clock_divider(2_550_000_000, hz), Some(255));
        assert_eq!(rmt_clock_divider(2_560_000_000, hz), None);
        assert_eq!(rmt_clock_divider(80_000_000, 0), None);
    }

    #[test]
    fn bit_word_clamps_to_fifteen_bits() {
        let w = rmt_bit_word(BitPulse {
            high: 40_000,
            low: 1,
        });
        assert_eq!(w, word(true, 32_767, false, 1));
    }
}
