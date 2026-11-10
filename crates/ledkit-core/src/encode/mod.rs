//! Turning pixels and timings into signals.

mod equal_period;
mod pio;
mod pio_timing;
mod pulse;
mod pwm;
mod rmt;
mod spi;
mod uart;

pub use equal_period::EqualPeriodTicks;
pub use pio::{fill_pio_words, pio_word_count, pio_words};
pub use pio_timing::PioTiming;
pub use pulse::{BitPulse, PulseTicks};
pub use pwm::{fill_pwm_words, pwm_bit_word, pwm_reset_words, pwm_word_count, PWM_RESET_WORD};
pub use rmt::{fill_rmt_words, rmt_bit_word, rmt_clock_divider, rmt_reset_word, rmt_word_count};
pub use spi::SpiEncoding;
pub use uart::UartEncoding;

/// Clamps a `u64` into a `u32`, saturating at `u32::MAX`.
pub(crate) const fn saturate_u32(value: u64) -> u32 {
    if value > u32::MAX as u64 {
        u32::MAX
    } else {
        value as u32
    }
}

/// Whether `k` units of a `hz` clock last between the phase's minimum and maximum, exactly.
pub(crate) const fn window_ok(phase: crate::Phase, k: u64, hz: u64) -> bool {
    let t = k * 1_000_000_000;
    phase.min_ns as u64 * hz <= t && t <= phase.max_ns as u64 * hz
}

/// How far `k` units of a `hz` clock are from the phase's typical length, in picoseconds.
pub(crate) const fn miss_ps(phase: crate::Phase, k: u64, hz: u64) -> u64 {
    (k * 1_000_000_000_000 / hz).abs_diff(phase.typical_ns as u64 * 1000)
}
