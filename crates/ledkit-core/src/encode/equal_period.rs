//! Bit timings where a `0` and a `1` last equally long, for PWM and PIO peripherals.

use super::{miss_ps, window_ok};
use crate::{ClocklessTiming, Phase, TimingError};

/// One LED bit as a fixed number of clock ticks (`period`) of which the line is high
/// for `zero_high` ticks to send a `0` or `one_high` ticks to send a `1`.
///
/// Some peripherals can't change the length of a bit, only how long it stays high:
/// the nRF52's PWM (its counter top is the same for every bit) and the standard
/// RP2040 PIO program. Get one from [`ClocklessTiming::equal_period_ticks`].
///
/// # Example
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let ticks = ClocklessTiming::of::<Ws2812b>().equal_period_ticks(16_000_000, 255)?;
/// assert_eq!((ticks.period, ticks.zero_high, ticks.one_high), (22, 6, 13));
/// # Ok::<(), ledkit_core::TimingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualPeriodTicks {
    /// Ticks in one LED bit, `0` or `1`.
    pub period: u32,
    /// Ticks the line is high for a `0` bit; it is low for the rest of the period.
    pub zero_high: u32,
    /// Ticks the line is high for a `1` bit; it is low for the rest of the period.
    pub one_high: u32,
}

impl ClocklessTiming {
    /// Works out a bit length, in ticks of a `ticks_per_second` clock, that both a `0`
    /// and a `1` can use, and how long each stays high.
    ///
    /// Every pulse (both highs and both lows) is inside the chip's datasheet window.
    /// Of the timings that fit, it picks the one whose high times are closest to the
    /// chip's typical ones (the LED tells `0` from `1` by how long the line is high),
    /// then the one whose period is closest to the chip's typical bit length (which
    /// keeps the lows away from their limits), then the shorter period.
    ///
    /// This is a `const fn`, so it also works at compile time.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::EqualPeriodUnusable`] if no period of at most
    /// `max_period` ticks fits, usually because the clock is too slow.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let timing = ClocklessTiming::of::<Ws2812b>();
    /// // The nRF52's 16 MHz PWM clock: 22 ticks (1.375 us) per bit.
    /// assert_eq!(timing.equal_period_ticks(16_000_000, 32_767)?.period, 22);
    /// // 1 MHz is far too slow.
    /// assert!(timing.equal_period_ticks(1_000_000, 32_767).is_err());
    /// # Ok::<(), ledkit_core::TimingError>(())
    /// ```
    pub const fn equal_period_ticks(
        &self,
        ticks_per_second: u32,
        max_period: u32,
    ) -> Result<EqualPeriodTicks, TimingError> {
        let hz = ticks_per_second as u64;
        let unusable = TimingError::EqualPeriodUnusable {
            ticks_per_second,
            max_period,
        };
        if hz == 0 {
            return Err(unusable);
        }
        // The chip's typical bit length: the average of its typical 0 and 1 bits.
        let typical_period_ps = (self.zero_high.typical_ns as u64
            + self.zero_low.typical_ns as u64
            + self.one_high.typical_ns as u64
            + self.one_low.typical_ns as u64)
            * 500;
        let (zero_high_lo, zero_high_hi) = tick_range(self.zero_high, hz);
        let (one_high_lo, one_high_hi) = tick_range(self.one_high, hz);
        let (zero_low_lo, zero_low_hi) = tick_range(self.zero_low, hz);
        let (one_low_lo, one_low_hi) = tick_range(self.one_low, hz);

        // (ticks, miss of the highs, miss of the period), best so far.
        let mut best: Option<(EqualPeriodTicks, u64, u64)> = None;
        let mut zero_high = zero_high_lo;
        while zero_high <= zero_high_hi {
            let mut one_high = one_high_lo;
            while one_high <= one_high_hi {
                let high_miss =
                    miss_ps(self.zero_high, zero_high, hz) + miss_ps(self.one_high, one_high, hz);
                // Periods where both lows are inside their windows.
                let first = max(max(zero_high + zero_low_lo, one_high + one_low_lo), 2);
                let last = min(
                    min(zero_high + zero_low_hi, one_high + one_low_hi),
                    max_period as u64,
                );
                let mut period = first;
                while period <= last {
                    if window_ok(self.zero_high, zero_high, hz)
                        && window_ok(self.one_high, one_high, hz)
                        && window_ok(self.zero_low, period - zero_high, hz)
                        && window_ok(self.one_low, period - one_high, hz)
                    {
                        let period_miss =
                            (period * 1_000_000_000_000 / hz).abs_diff(typical_period_ps);
                        let better = match best {
                            None => true,
                            Some((b, b_high, b_period)) => {
                                high_miss < b_high
                                    || (high_miss == b_high
                                        && (period_miss < b_period
                                            || (period_miss == b_period
                                                && (period as u32) < b.period)))
                            }
                        };
                        if better {
                            let ticks = EqualPeriodTicks {
                                period: period as u32,
                                zero_high: zero_high as u32,
                                one_high: one_high as u32,
                            };
                            best = Some((ticks, high_miss, period_miss));
                        }
                    }
                    period += 1;
                }
                one_high += 1;
            }
            zero_high += 1;
        }
        match best {
            Some((ticks, _, _)) => Ok(ticks),
            None => Err(unusable),
        }
    }
}

/// The whole tick counts (at least 1) that might fall inside the phase's window. The
/// ends are rounded outwards; the caller checks each candidate exactly.
const fn tick_range(phase: Phase, hz: u64) -> (u64, u64) {
    let lo = phase.min_ns as u64 * hz / 1_000_000_000;
    let hi = (phase.max_ns as u64 * hz).div_ceil(1_000_000_000);
    (if lo == 0 { 1 } else { lo }, hi)
}

const fn max(a: u64, b: u64) -> u64 {
    if a > b {
        a
    } else {
        b
    }
}

const fn min(a: u64, b: u64) -> u64 {
    if a < b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Apa106, Chip, Sk6812, Sk6812Rgbw, Ws2811, Ws2811Slow, Ws2812, Ws2812b, Ws2813};

    const NRF_PWM_HZ: u32 = 16_000_000;

    fn at<C: Chip>(hz: u32) -> (u32, u32, u32) {
        let t = ClocklessTiming::of::<C>()
            .equal_period_ticks(hz, 32_767)
            .unwrap();
        (t.period, t.zero_high, t.one_high)
    }

    #[test]
    fn ws2812b_at_16mhz() {
        // Not the familiar 20 ticks: there the 1 bit's low would be 437.5 ns,
        // under the WS2812B's 450 ns minimum.
        assert_eq!(at::<Ws2812b>(NRF_PWM_HZ), (22, 6, 13));
    }

    #[test]
    fn ws2812_ws2813_ws2811_at_16mhz() {
        assert_eq!(at::<Ws2812>(NRF_PWM_HZ), (20, 6, 11));
        assert_eq!(at::<Ws2813>(NRF_PWM_HZ), (20, 6, 14));
        assert_eq!(at::<Ws2811>(NRF_PWM_HZ), (20, 4, 10));
    }

    #[test]
    fn sk6812_at_16mhz() {
        assert_eq!(at::<Sk6812>(NRF_PWM_HZ), (19, 5, 10));
        assert_eq!(at::<Sk6812Rgbw>(NRF_PWM_HZ), (19, 5, 10));
    }

    #[test]
    fn slow_ws2811_and_apa106_at_16mhz() {
        assert_eq!(at::<Ws2811Slow>(NRF_PWM_HZ), (40, 8, 19));
        assert_eq!(at::<Apa106>(NRF_PWM_HZ), (27, 6, 22));
    }

    #[test]
    fn ws2812b_at_8mhz() {
        assert_eq!(at::<Ws2812b>(8_000_000), (11, 3, 6));
    }

    #[test]
    fn every_pulse_is_inside_its_window() {
        let timing = ClocklessTiming::of::<Ws2812b>();
        for hz in [8_000_000, 10_000_000, 16_000_000, 20_000_000, 40_000_000] {
            let t = timing.equal_period_ticks(hz, 32_767).unwrap();
            let ns = |ticks: u32| (u64::from(ticks) * 1_000_000_000 / u64::from(hz)) as u32;
            assert!(timing.zero_high.contains(ns(t.zero_high)), "{hz}");
            assert!(timing.one_high.contains(ns(t.one_high)), "{hz}");
            assert!(timing.zero_low.contains(ns(t.period - t.zero_high)), "{hz}");
            assert!(timing.one_low.contains(ns(t.period - t.one_high)), "{hz}");
        }
    }

    #[test]
    fn too_slow_a_clock_has_no_equal_period() {
        assert_eq!(
            ClocklessTiming::of::<Ws2812b>().equal_period_ticks(1_000_000, 32_767),
            Err(TimingError::EqualPeriodUnusable {
                ticks_per_second: 1_000_000,
                max_period: 32_767
            })
        );
    }

    #[test]
    fn zero_clock_is_an_error_not_a_panic() {
        assert!(ClocklessTiming::of::<Ws2812b>()
            .equal_period_ticks(0, 32_767)
            .is_err());
    }

    #[test]
    fn max_period_is_respected() {
        let timing = ClocklessTiming::of::<Ws2812b>();
        // 21 ticks still fits (6 and 13 high, lows of 937.5 and 500 ns).
        assert_eq!(
            timing.equal_period_ticks(NRF_PWM_HZ, 21).unwrap().period,
            21
        );
        // At 20 the 1 bit's high drops to 12 ticks (750 ns) so its low stays 450 ns or more.
        let t = timing.equal_period_ticks(NRF_PWM_HZ, 20).unwrap();
        assert_eq!((t.period, t.zero_high, t.one_high), (20, 6, 12));
        // Nothing fits in 10 ticks (625 ns).
        assert!(timing.equal_period_ticks(NRF_PWM_HZ, 10).is_err());
    }

    #[test]
    fn works_at_compile_time() {
        const TICKS: Result<EqualPeriodTicks, TimingError> =
            ClocklessTiming::of::<Ws2812b>().equal_period_ticks(16_000_000, 32_767);
        assert_eq!(TICKS.unwrap().period, 22);
    }

    #[test]
    fn a_phase_window_can_be_checked_directly() {
        // The search relies on the same exact window test as `pulse_ticks`.
        let p = Phase::new(400, 220, 450);
        assert!(window_ok(p, 7, 16_000_000)); // 437.5 ns
        assert!(!window_ok(p, 8, 16_000_000)); // 500 ns
        assert_eq!(miss_ps(p, 6, 16_000_000), 25_000);
    }
}
