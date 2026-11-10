//! Duty words for PWM sequencers (the nRF52 family).
//!
//! The nRF52's PWM plays a list of 16-bit "duty" words from RAM, one per PWM period.
//! With the period set to one LED bit (see
//! [`ClocklessTiming::equal_period_ticks`](crate::ClocklessTiming::equal_period_ticks)),
//! each word is one LED bit.

use super::equal_period::EqualPeriodTicks;
use crate::BufferTooSmall;

/// Bit 15 of a duty word: the polarity. When set, the output is high from the start of
/// the period until the counter reaches the compare value, then low.
const FALLING_EDGE: u16 = 1 << 15;

/// The largest compare value a duty word holds (15 bits).
const MAX_COMPARE: u32 = 0x7FFF;

/// The duty word that keeps the line low for a whole period, used for the reset gap.
///
/// It has the polarity bit set and a compare value of 0: high for 0 ticks. (With the
/// polarity bit clear, a compare value of 0 would hold the line *high*.)
///
/// # Example
///
/// ```
/// use ledkit_core::encode::{pwm_bit_word, PWM_RESET_WORD};
///
/// assert_eq!(PWM_RESET_WORD, pwm_bit_word(0));
/// ```
pub const PWM_RESET_WORD: u16 = FALLING_EDGE;

/// The duty word for one LED bit that is high for `high_ticks` ticks, then low for the
/// rest of the period.
///
/// The layout is the nRF52 PWM's (what `embassy-nrf`'s `SequencePwm` plays): bits 0 to
/// 14 are the compare value and bit 15 is the polarity, which ledkit always sets so the
/// line starts each period high. Values above 32 767 are cut to 32 767.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::pwm_bit_word;
///
/// // High for 13 ticks of 62.5 ns (812.5 ns).
/// assert_eq!(pwm_bit_word(13), 0x800D);
/// ```
#[must_use]
pub const fn pwm_bit_word(high_ticks: u32) -> u16 {
    let compare = if high_ticks > MAX_COMPARE {
        MAX_COMPARE
    } else {
        high_ticks
    };
    FALLING_EDGE | compare as u16
}

/// How many [`PWM_RESET_WORD`]s make the reset gap: enough whole periods of `period`
/// ticks, at `ticks_per_second`, to cover `reset_us` microseconds. At least 1.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::pwm_reset_words;
///
/// // The WS2812B's 280 us at 22 ticks of 16 MHz (1.375 us) per bit.
/// assert_eq!(pwm_reset_words(280, 22, 16_000_000), 204);
/// ```
#[must_use]
pub const fn pwm_reset_words(reset_us: u32, period: u32, ticks_per_second: u32) -> usize {
    let reset_ticks = reset_us as u64 * ticks_per_second as u64;
    let period_ticks = period as u64 * 1_000_000;
    if period_ticks == 0 {
        return 1;
    }
    let words = reset_ticks.div_ceil(period_ticks);
    if words == 0 {
        1
    } else {
        words as usize
    }
}

/// How many words [`fill_pwm_words`] writes for `byte_count` wire bytes: one per bit,
/// plus `reset_words`.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::pwm_word_count;
///
/// // One RGB LED (3 bytes) and a 204-word reset gap.
/// assert_eq!(pwm_word_count(3, 204), 228);
/// ```
#[must_use]
pub const fn pwm_word_count(byte_count: usize, reset_words: usize) -> usize {
    byte_count * 8 + reset_words
}

/// Fills `out` with the duty words of a clockless frame: one word per bit of `wire`
/// (most significant bit first), then `reset_words` [`PWM_RESET_WORD`]s. Returns how
/// many words it wrote.
///
/// The word layout is described at [`pwm_bit_word`]. If `out` is too short the function
/// still counts every byte, so the error says exactly how big `out` must be.
///
/// # Errors
///
/// Returns [`BufferTooSmall`] if `out` can't hold [`pwm_word_count`] words; `out` is
/// then partly written and must not be sent.
///
/// # Example
///
/// ```
/// use ledkit_core::encode::{fill_pwm_words, pwm_bit_word, PWM_RESET_WORD};
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let ticks = ClocklessTiming::of::<Ws2812b>().equal_period_ticks(16_000_000, 32_767)?;
/// let mut out = [0u16; 10];
/// let used = fill_pwm_words([0b1000_0000u8], ticks, 2, &mut out).unwrap();
///
/// assert_eq!(used, 10);
/// assert_eq!(out[0], pwm_bit_word(ticks.one_high));
/// assert_eq!(out[1], pwm_bit_word(ticks.zero_high));
/// assert_eq!(out[8..], [PWM_RESET_WORD; 2]);
/// # Ok::<(), ledkit_core::TimingError>(())
/// ```
pub fn fill_pwm_words(
    wire: impl IntoIterator<Item = u8>,
    ticks: EqualPeriodTicks,
    reset_words: usize,
    out: &mut [u16],
) -> Result<usize, BufferTooSmall> {
    let zero = pwm_bit_word(ticks.zero_high);
    let one = pwm_bit_word(ticks.one_high);
    let mut count = 0usize;
    for byte in wire {
        for bit in (0..8u8).rev() {
            if let Some(slot) = out.get_mut(count) {
                *slot = if (byte >> bit) & 1 == 1 { one } else { zero };
            }
            count += 1;
        }
    }
    let needed = count + reset_words;
    if out.len() < needed {
        return Err(BufferTooSmall { needed });
    }
    out[count..needed].fill(PWM_RESET_WORD);
    Ok(needed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ClocklessTiming, Sk6812, Ws2812b};

    const HZ: u32 = 16_000_000;

    fn ws_ticks() -> EqualPeriodTicks {
        ClocklessTiming::of::<Ws2812b>()
            .equal_period_ticks(HZ, 32_767)
            .unwrap()
    }

    #[test]
    fn bit_word_sets_polarity() {
        assert_eq!(pwm_bit_word(13), 0x800D);
        assert_eq!(pwm_bit_word(6), 0x8006);
        assert_eq!(pwm_bit_word(100_000), 0xFFFF);
    }

    #[test]
    fn reset_word_is_low_all_period() {
        assert_eq!(PWM_RESET_WORD, 0x8000);
    }

    #[test]
    fn ws2812b_reset_is_204_words() {
        let t = ws_ticks();
        assert_eq!(pwm_reset_words(280, t.period, HZ), 204);
    }

    #[test]
    fn sk6812_reset_is_68_words() {
        let t = ClocklessTiming::of::<Sk6812>()
            .equal_period_ticks(HZ, 32_767)
            .unwrap();
        // 80 us / 1.1875 us = 67.4, rounded up.
        assert_eq!(pwm_reset_words(80, t.period, HZ), 68);
    }

    #[test]
    fn reset_is_at_least_one_word() {
        assert_eq!(pwm_reset_words(0, 22, HZ), 1);
        assert_eq!(pwm_reset_words(280, 0, HZ), 1);
    }

    #[test]
    fn fill_one_byte() {
        let mut out = [0u16; 12];
        let used = fill_pwm_words([0b1000_0001u8], ws_ticks(), 4, &mut out).unwrap();
        assert_eq!(used, 12);
        assert_eq!(
            out,
            [
                0x800D, 0x8006, 0x8006, 0x8006, 0x8006, 0x8006, 0x8006, 0x800D, 0x8000, 0x8000,
                0x8000, 0x8000
            ]
        );
    }

    #[test]
    fn fill_pwm_words_with_no_wire_bytes_is_only_reset() {
        let mut out = [0u16; 3];
        assert_eq!(fill_pwm_words([], ws_ticks(), 3, &mut out), Ok(3));
        assert_eq!(out, [PWM_RESET_WORD; 3]);
    }

    #[test]
    fn buffer_too_small_reports_exact_need() {
        let mut out = [0u16; 10];
        assert_eq!(
            fill_pwm_words([0u8; 3], ws_ticks(), 204, &mut out),
            Err(BufferTooSmall { needed: 228 })
        );
        assert_eq!(pwm_word_count(3, 204), 228);
    }
}
