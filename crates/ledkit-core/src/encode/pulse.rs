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
    /// per-pulse limit; split it across several pulses if needed.
    pub reset: u32,
}

/// Converts nanoseconds to the nearest whole number of ticks.
fn ns_to_ticks(ns: u64, ticks_per_second: u32) -> u64 {
    (ns * u64::from(ticks_per_second) + 500_000_000) / 1_000_000_000
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
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let t = ClocklessTiming::of::<Ws2812b>();
    /// assert!(t.pulse_ticks(10_000_000, 32_767).is_ok());
    /// assert!(t.pulse_ticks(1_000_000, 32_767).is_err());
    /// ```
    pub fn pulse_ticks(
        &self,
        ticks_per_second: u32,
        max_ticks: u32,
    ) -> Result<PulseTicks, TimingError> {
        let phase = |p: Phase, which: BitPhase| -> Result<u32, TimingError> {
            let ticks = ns_to_ticks(u64::from(p.typical_ns), ticks_per_second);
            let actual = ticks * 1_000_000_000 / u64::from(ticks_per_second);
            let actual_ns = u32::try_from(actual).unwrap_or(u32::MAX);
            if ticks == 0 || !p.contains(actual_ns) {
                return Err(TimingError::PhaseOutOfRange {
                    phase: which,
                    actual_ns: if ticks == 0 { 0 } else { actual_ns },
                    min_ns: p.min_ns,
                    max_ns: p.max_ns,
                });
            }
            match u32::try_from(ticks) {
                Ok(t) if t <= max_ticks => Ok(t),
                _ => Err(TimingError::TooManyTicks {
                    ticks: u32::try_from(ticks).unwrap_or(u32::MAX),
                    max_ticks,
                }),
            }
        };
        let zero_high = phase(self.zero_high, BitPhase::ZeroHigh)?;
        let zero_low = phase(self.zero_low, BitPhase::ZeroLow)?;
        let one_high = phase(self.one_high, BitPhase::OneHigh)?;
        let one_low = phase(self.one_low, BitPhase::OneLow)?;
        let reset = ns_to_ticks(u64::from(self.reset_us) * 1000, ticks_per_second);
        Ok(PulseTicks {
            zero_high,
            zero_low,
            one_high,
            one_low,
            reset: u32::try_from(reset).unwrap_or(u32::MAX),
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
