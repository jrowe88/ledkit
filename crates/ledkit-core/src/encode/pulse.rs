//! Pulse lengths in clock ticks, for RMT and PIO style peripherals.

use crate::{BitPhase, Chip, ClocklessTiming, Phase, Protocol, TimingError};

/// Pulse lengths of a clockless LED signal, counted in ticks of a hardware clock.
///
/// Peripherals such as the ESP32 RMT or the RP2040 PIO describe a signal as
/// "stay high for N ticks, then low for M ticks". Get one from
/// [`ClocklessTiming::pulse_ticks`].
///
/// # Example
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let ticks = ClocklessTiming::of::<Ws2812b>().pulse_ticks(10_000_000, 32_767)?;
/// assert_eq!(ticks.zero_high, 4);
/// assert_eq!(ticks.reset, 2800);
/// # Ok::<(), ledkit_core::TimingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseTicks {
    /// Ticks the line is high for a `0` bit.
    pub zero_high: u32,
    /// Ticks the line is low for a `0` bit.
    pub zero_low: u32,
    /// Ticks the line is high for a `1` bit.
    pub one_high: u32,
    /// Ticks the line is low for a `1` bit.
    pub one_low: u32,
    /// Ticks the line stays low to latch the colors. It may exceed the
    /// per-pulse limit; split it across several pulses if needed. Very long
    /// resets saturate at `u32::MAX`.
    pub reset: u32,
}

/// Converts nanoseconds to the nearest whole number of ticks.
const fn ns_to_ticks(ns: u64, ticks_per_second: u32) -> u64 {
    let ticks = (ns as u128 * ticks_per_second as u128 + 500_000_000) / 1_000_000_000;
    if ticks > u64::MAX as u128 {
        u64::MAX
    } else {
        ticks as u64
    }
}

/// Clamps a `u64` into a `u32`, saturating at `u32::MAX`.
const fn saturate_u32(value: u64) -> u32 {
    if value > u32::MAX as u64 {
        u32::MAX
    } else {
        value as u32
    }
}

/// Ticks for one pulse, checked against its window and the tick limit.
const fn phase_ticks(
    p: Phase,
    which: BitPhase,
    ticks_per_second: u32,
    max_ticks: u32,
) -> Result<u32, TimingError> {
    let ticks = ns_to_ticks(p.typical_ns as u64, ticks_per_second);
    let tps = ticks_per_second as u128;
    // The pulse lasts `ticks / ticks_per_second` seconds. Compare it with
    // the window exactly, with both sides multiplied by `ticks_per_second`,
    // so a pulse of 450.45 ns never passes a 450 ns maximum.
    let pulse = ticks as u128 * 1_000_000_000;
    let in_window = p.min_ns as u128 * tps <= pulse && pulse <= p.max_ns as u128 * tps;
    // A zero-tick phase (including a zero clock rate) is never valid;
    // return before dividing by `ticks_per_second`.
    if ticks == 0 || !in_window {
        let actual_ns = if ticks == 0 {
            0
        } else {
            let ns = pulse / tps;
            if ns > u32::MAX as u128 {
                u32::MAX
            } else {
                ns as u32
            }
        };
        return Err(TimingError::PhaseOutOfRange {
            phase: which,
            actual_ns,
            min_ns: p.min_ns,
            max_ns: p.max_ns,
        });
    }
    if ticks > max_ticks as u64 {
        return Err(TimingError::TooManyTicks {
            ticks: saturate_u32(ticks),
            max_ticks,
        });
    }
    Ok(saturate_u32(ticks))
}

impl ClocklessTiming {
    /// Gets the timing of a clockless chip, for example `ClocklessTiming::of::<Ws2812b>()`.
    ///
    /// # Panics
    ///
    /// Panics with "this chip is clocked, not clockless" if `C` is a clocked
    /// chip such as the APA102.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// assert_eq!(ClocklessTiming::of::<Ws2812b>().reset_us, 280);
    /// ```
    #[must_use]
    pub const fn of<C: Chip>() -> ClocklessTiming {
        match C::PROTOCOL {
            Protocol::Clockless(t) => t,
            Protocol::Clocked(_) => panic!("this chip is clocked, not clockless"),
        }
    }

    /// Works out how many ticks each pulse lasts on a clock of `ticks_per_second`.
    ///
    /// Each pulse is rounded to the nearest whole tick. It is an error if a
    /// pulse rounds to zero ticks or lands outside the chip's allowed window
    /// ([`TimingError::PhaseOutOfRange`]), or needs more than `max_ticks`
    /// ([`TimingError::TooManyTicks`]). The reset is not checked against
    /// either limit.
    ///
    /// # Errors
    ///
    /// Returns a [`TimingError`] if the clock is too slow, too fast or
    /// too coarse for this chip.
    ///
    /// This is a `const fn`, so it also works at compile time.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let t = ClocklessTiming::of::<Ws2812b>();
    /// assert!(t.pulse_ticks(10_000_000, 32_767).is_ok());
    /// assert!(t.pulse_ticks(1_000_000, 32_767).is_err());
    /// ```
    pub const fn pulse_ticks(
        &self,
        ticks_per_second: u32,
        max_ticks: u32,
    ) -> Result<PulseTicks, TimingError> {
        let zero_high = match phase_ticks(
            self.zero_high,
            BitPhase::ZeroHigh,
            ticks_per_second,
            max_ticks,
        ) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };
        let zero_low = match phase_ticks(
            self.zero_low,
            BitPhase::ZeroLow,
            ticks_per_second,
            max_ticks,
        ) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };
        let one_high = match phase_ticks(
            self.one_high,
            BitPhase::OneHigh,
            ticks_per_second,
            max_ticks,
        ) {
            Ok(t) => t,
            Err(e) => return Err(e),
        };
        let one_low = match phase_ticks(self.one_low, BitPhase::OneLow, ticks_per_second, max_ticks)
        {
            Ok(t) => t,
            Err(e) => return Err(e),
        };
        let reset = ns_to_ticks(self.reset_us as u64 * 1000, ticks_per_second);
        Ok(PulseTicks {
            zero_high,
            zero_low,
            one_high,
            one_low,
            reset: saturate_u32(reset),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Sk6812, Ws2812b};

    #[test]
    fn ws2812b_at_10mhz() {
        let t = ClocklessTiming::of::<Ws2812b>();
        assert_eq!(
            t.pulse_ticks(10_000_000, 32_767),
            Ok(PulseTicks {
                zero_high: 4,
                zero_low: 9,
                one_high: 8,
                one_low: 6,
                reset: 2800
            })
        );
    }
    #[test]
    fn ws2812b_at_40mhz() {
        let p = ClocklessTiming::of::<Ws2812b>()
            .pulse_ticks(40_000_000, 32_767)
            .unwrap();
        assert_eq!(
            (p.zero_high, p.zero_low, p.one_high, p.one_low, p.reset),
            (16, 36, 32, 24, 11200)
        );
    }
    #[test]
    fn sk6812_at_10mhz() {
        let p = ClocklessTiming::of::<Sk6812>()
            .pulse_ticks(10_000_000, 32_767)
            .unwrap();
        assert_eq!(
            (p.zero_high, p.zero_low, p.one_high, p.one_low, p.reset),
            (3, 9, 6, 6, 800)
        );
    }
    #[test]
    fn too_coarse_a_clock_is_rejected() {
        assert!(matches!(
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(1_000_000, 32_767),
            Err(TimingError::PhaseOutOfRange {
                phase: BitPhase::ZeroHigh,
                ..
            })
        ));
    }
    #[test]
    fn tick_limit_is_enforced() {
        assert!(matches!(
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(80_000_000, 50),
            Err(TimingError::TooManyTicks { .. })
        ));
    }
    #[test]
    fn zero_clock_rate_is_an_error_not_a_panic() {
        assert!(matches!(
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(0, 32_767),
            Err(TimingError::PhaseOutOfRange {
                phase: BitPhase::ZeroHigh,
                actual_ns: 0,
                ..
            })
        ));
    }
    #[test]
    fn huge_reset_saturates() {
        let mut t = ClocklessTiming::of::<Ws2812b>();
        t.reset_us = u32::MAX;
        let p = t.pulse_ticks(u32::MAX, u32::MAX).unwrap();
        assert_eq!(p.reset, u32::MAX);
    }
    #[test]
    fn works_at_compile_time() {
        const TICKS: Result<PulseTicks, TimingError> =
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(10_000_000, 32_767);
        assert_eq!(
            TICKS,
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(10_000_000, 32_767)
        );
        assert!(TICKS.is_ok());
    }
    #[test]
    fn spi_message_with_both_neighbours() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 4_000_000,
            nearest_lower_hz: Some(3_300_000),
            nearest_higher_hz: Some(4_500_000),
        };
        let msg = format!("{e}");
        assert!(msg.contains("3.3 MHz") && msg.contains("4.5 MHz"), "{msg}");
    }
    #[test]
    fn spi_message_with_no_neighbours() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 4_000_000,
            nearest_lower_hz: None,
            nearest_higher_hz: None,
        };
        let msg = format!("{e}");
        assert!(msg.contains("no SPI clock works for this LED"), "{msg}");
    }
    #[test]
    fn mhz_prints_two_decimals_when_needed() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 1_950_000,
            nearest_lower_hz: Some(3_350_000),
            nearest_higher_hz: Some(3_200_000),
        };
        let msg = format!("{e}");
        assert!(msg.contains("1.95 MHz"), "{msg}");
        assert!(msg.contains("3.35 MHz"), "{msg}");
        assert!(
            msg.contains("3.2 MHz") && !msg.contains("3.20 MHz"),
            "{msg}"
        );
    }
    #[test]
    fn mhz_two_decimals_pad_and_truncate() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 2_050_000,
            nearest_lower_hz: Some(4_512_345),
            nearest_higher_hz: None,
        };
        let msg = format!("{e}");
        assert!(
            msg.contains("2.05 MHz") && msg.contains("4.51 MHz"),
            "{msg}"
        );
    }
    #[test]
    fn too_short_pulse_suggests_a_faster_clock() {
        // At 1 MHz the 400 ns zero-high rounds to 0 ticks.
        let e = ClocklessTiming::of::<Ws2812b>()
            .pulse_ticks(1_000_000, 32_767)
            .unwrap_err();
        let msg = format!("{e}");
        assert!(
            msg.contains("0 ns") && msg.contains("try a faster clock"),
            "{msg}"
        );
    }
    #[test]
    fn too_long_pulse_message() {
        // At 1.8 MHz the 400 ns zero-high rounds to 1 tick = 555 ns (> 450).
        let e = ClocklessTiming::of::<Ws2812b>()
            .pulse_ticks(1_800_000, 32_767)
            .unwrap_err();
        assert_eq!(
            e,
            TimingError::PhaseOutOfRange {
                phase: BitPhase::ZeroHigh,
                actual_ns: 555,
                min_ns: 220,
                max_ns: 450,
            }
        );
        let msg = format!("{e}");
        assert!(
            msg.contains("555 ns") && msg.contains("too coarse"),
            "{msg}"
        );
    }
    #[test]
    fn errors_implement_core_error() {
        fn is_error<E: core::error::Error>(_: &E) {}
        is_error(&TimingError::TooManyTicks {
            ticks: 2,
            max_ticks: 1,
        });
        is_error(&crate::BufferTooSmall { needed: 1 });
    }
    #[test]
    fn window_is_checked_exactly_not_after_rounding() {
        // At 2.22 MHz one tick is 450.45 ns: just over the 450 ns maximum.
        assert_eq!(
            ClocklessTiming::of::<Ws2812b>().pulse_ticks(2_220_000, 32_767),
            Err(TimingError::PhaseOutOfRange {
                phase: BitPhase::ZeroHigh,
                actual_ns: 450,
                min_ns: 220,
                max_ns: 450,
            })
        );
    }
    #[test]
    fn window_error_beats_tick_limit() {
        let mut t = ClocklessTiming::of::<Ws2812b>();
        t.zero_high = Phase::new(400, 300, 350);
        assert!(matches!(
            t.pulse_ticks(80_000_000, 10),
            Err(TimingError::PhaseOutOfRange {
                phase: BitPhase::ZeroHigh,
                ..
            })
        ));
    }
    #[test]
    fn error_message_is_readable() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 1_000_000,
            nearest_lower_hz: None,
            nearest_higher_hz: Some(2_000_000),
        };
        let msg = format!("{e}");
        assert!(msg.contains("1.0 MHz") && msg.contains("2.0 MHz"), "{msg}");
    }
}
