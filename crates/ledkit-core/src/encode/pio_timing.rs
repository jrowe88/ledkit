//! The step timing of the RP2040 / RP2350 PIO: an equal-period bit split into three
//! steps, with a whole clock divider.

use super::{miss_ps, EqualPeriodTicks};
use crate::{ClocklessTiming, TimingError};

/// How a PIO program like the Raspberry Pi `ws2812.pio` sends every bit: the state
/// machine runs at the system clock divided by `divider`, and each bit is three
/// steps of whole state machine cycles:
///
/// 1. high for `high` cycles (`T1`);
/// 2. high for a `1`, low for a `0`, for `data` cycles (`T2`);
/// 3. low for `low` cycles (`T3`).
///
/// Get one from [`ClocklessTiming::pio_timing`].
///
/// # Example
///
/// ```
/// use ledkit_core::encode::PioTiming;
///
/// let t = PioTiming { divider: 5, high: 10, data: 10, low: 15, reset_us: 280 };
/// assert_eq!(t.period(), 35);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PioTiming {
    /// The whole-number clock divider: one state machine cycle is this many system
    /// clock cycles.
    pub divider: u32,
    /// Cycles every bit starts high.
    pub high: u32,
    /// Cycles the line then stays high for a `1`, or is low for a `0`.
    pub data: u32,
    /// Cycles every bit ends low.
    pub low: u32,
    /// The chip's reset time, in microseconds.
    pub reset_us: u32,
}

impl PioTiming {
    /// The length of one bit, in state machine cycles: `high + data + low`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::encode::PioTiming;
    ///
    /// let t = PioTiming { divider: 1, high: 4, data: 4, low: 6, reset_us: 280 };
    /// assert_eq!(t.period(), 14);
    /// ```
    #[must_use]
    pub const fn period(&self) -> u32 {
        self.high + self.data + self.low
    }
}

impl ClocklessTiming {
    /// Works out the PIO step timing for a system clock of `sys_hz`: a whole clock
    /// divider from 1 to `max_divider`, and three steps of 1 to `max_step` cycles
    /// each.
    ///
    /// For every divider that divides `sys_hz` exactly, it asks
    /// [`equal_period_ticks`](Self::equal_period_ticks) for a bit both a `0` and a
    /// `1` can share, splits it into the three steps, and keeps the one whose four
    /// pulses are closest to the chip's typical lengths (ties go to the smaller
    /// divider). Fractional dividers are never used: they make pulse widths wobble
    /// by a cycle. A PIO instruction with one side-set bit waits at most 16 cycles,
    /// so `max_step` is 16 there.
    ///
    /// # Errors
    ///
    /// [`TimingError::EqualPeriodUnusable`] if no divider works (for example a
    /// system clock far too slow for the chip).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// // An RP2040 at 125 MHz.
    /// let t = ClocklessTiming::of::<Ws2812b>().pio_timing(125_000_000, 16, 65_535)?;
    /// assert!(t.high <= 16 && t.data <= 16 && t.low <= 16);
    /// # Ok::<(), ledkit_core::TimingError>(())
    /// ```
    pub const fn pio_timing(
        &self,
        sys_hz: u32,
        max_step: u32,
        max_divider: u32,
    ) -> Result<PioTiming, TimingError> {
        let mut best: Option<(u64, PioTiming)> = None;
        let mut divider = 1;
        while divider <= max_divider && divider <= sys_hz {
            let hz = sys_hz / divider;
            // Once one tick is longer than the longest allowed `0` high, every
            // larger divider fails too.
            if (self.zero_high.max_ns as u64) * (hz as u64) < 1_000_000_000 {
                break;
            }
            if sys_hz % divider == 0 {
                if let Ok(t) = self.equal_period_ticks(hz, 3 * max_step) {
                    let candidate = split(t, divider, self.reset_us);
                    if fits(&candidate, max_step) {
                        let h = hz as u64;
                        let score = miss_ps(self.zero_high, t.zero_high as u64, h)
                            + miss_ps(self.zero_low, (t.period - t.zero_high) as u64, h)
                            + miss_ps(self.one_high, t.one_high as u64, h)
                            + miss_ps(self.one_low, (t.period - t.one_high) as u64, h);
                        let better = match best {
                            None => true,
                            Some((b, _)) => score < b,
                        };
                        if better {
                            best = Some((score, candidate));
                        }
                    }
                }
            }
            divider += 1;
        }
        match best {
            Some((_, t)) => Ok(t),
            None => Err(TimingError::EqualPeriodUnusable {
                ticks_per_second: sys_hz,
                max_period: 3 * max_step,
            }),
        }
    }
}

/// Splits an equal-period bit into the three PIO steps.
const fn split(t: EqualPeriodTicks, divider: u32, reset_us: u32) -> PioTiming {
    PioTiming {
        divider,
        high: t.zero_high,
        data: t.one_high.saturating_sub(t.zero_high),
        low: t.period.saturating_sub(t.one_high),
        reset_us,
    }
}

/// Whether every step is 1 to `max_step` cycles.
const fn fits(t: &PioTiming, max_step: u32) -> bool {
    t.high >= 1
        && t.high <= max_step
        && t.data >= 1
        && t.data <= max_step
        && t.low >= 1
        && t.low <= max_step
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    /// Every pulse of `t` at `sys_hz` is inside the chip's window.
    fn assert_in_windows(timing: ClocklessTiming, t: PioTiming, sys_hz: u32) {
        let pulses = [
            (timing.zero_high, t.high),
            (timing.zero_low, t.data + t.low),
            (timing.one_high, t.high + t.data),
            (timing.one_low, t.low),
        ];
        for (phase, cycles) in pulses {
            let exact = cycles as u128 * t.divider as u128 * 1_000_000_000;
            assert!(
                phase.min_ns as u128 * sys_hz as u128 <= exact
                    && exact <= phase.max_ns as u128 * sys_hz as u128,
                "{t:?} misses {}..{} ns",
                phase.min_ns,
                phase.max_ns
            );
        }
    }

    #[test]
    fn every_clockless_chip_fits_the_rp_clocks() {
        fn check<C: Chip>() {
            let timing = ClocklessTiming::of::<C>();
            for hz in [125_000_000, 150_000_000, 133_000_000, 200_000_000] {
                let t = timing
                    .pio_timing(hz, 16, 65_535)
                    .unwrap_or_else(|e| panic!("{} at {hz}: {e}", C::NAME));
                assert!(t.high >= 1 && t.data >= 1 && t.low >= 1, "{t:?}");
                assert!(t.high <= 16 && t.data <= 16 && t.low <= 16, "{t:?}");
                assert_eq!(hz % t.divider, 0);
                assert_in_windows(timing, t, hz);
            }
        }
        check::<Ws2812>();
        check::<Ws2812b>();
        check::<Ws2813>();
        check::<Ws2815>();
        check::<Ws2811>();
        check::<Ws2811Slow>();
        check::<Sk6812>();
        check::<Sk6812Rgbw>();
        check::<Apa106>();
    }

    #[test]
    fn ws2812b_on_an_rp2040_and_an_rp2350() {
        let timing = ClocklessTiming::of::<Ws2812b>();
        let rp2040 = timing.pio_timing(125_000_000, 16, 65_535).unwrap();
        let rp2350 = timing.pio_timing(150_000_000, 16, 65_535).unwrap();
        assert_eq!(rp2040.reset_us, 280);
        assert!(rp2040.divider > 1 && rp2350.divider > 1);
        assert_in_windows(timing, rp2040, 125_000_000);
        assert_in_windows(timing, rp2350, 150_000_000);
    }

    #[test]
    fn too_slow_a_clock_is_refused() {
        let timing = ClocklessTiming::of::<Ws2812b>();
        assert_eq!(
            timing.pio_timing(1_000_000, 16, 65_535),
            Err(TimingError::EqualPeriodUnusable {
                ticks_per_second: 1_000_000,
                max_period: 48
            })
        );
        assert!(timing.pio_timing(0, 16, 65_535).is_err());
        // Too fast, with no divider allowed to slow it down.
        assert!(timing.pio_timing(150_000_000, 16, 1).is_err());
    }

    #[test]
    fn works_at_compile_time() {
        const T: Result<PioTiming, TimingError> =
            ClocklessTiming::of::<Ws2812b>().pio_timing(125_000_000, 16, 65_535);
        assert!(T.is_ok());
    }
}
