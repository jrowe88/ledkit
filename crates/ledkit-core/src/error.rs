//! Errors reported when a signal can't be produced.

use core::fmt;

/// Which of the four pulse windows of a clockless LED a problem refers to.
///
/// # Example
///
/// ```
/// use ledkit_core::BitPhase;
///
/// assert_eq!(format!("{}", BitPhase::ZeroHigh), "high time of a 0 bit");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitPhase {
    /// The high time of a `0` bit.
    ZeroHigh,
    /// The low time of a `0` bit.
    ZeroLow,
    /// The high time of a `1` bit.
    OneHigh,
    /// The low time of a `1` bit.
    OneLow,
}

impl fmt::Display for BitPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ZeroHigh => "high time of a 0 bit",
            Self::ZeroLow => "low time of a 0 bit",
            Self::OneHigh => "high time of a 1 bit",
            Self::OneLow => "low time of a 1 bit",
        })
    }
}

/// Why a clock speed can't produce the timing an LED needs.
///
/// # Example
///
/// ```
/// use ledkit_core::TimingError;
///
/// let e = TimingError::TooManyTicks { ticks: 90, max_ticks: 50 };
/// assert!(format!("{e}").contains("90"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimingError {
    /// A pulse would be too short or too long for the chip to accept.
    PhaseOutOfRange {
        /// Which pulse is wrong.
        phase: BitPhase,
        /// How long the pulse would actually be, in nanoseconds.
        actual_ns: u32,
        /// The shortest length the chip accepts, in nanoseconds.
        min_ns: u32,
        /// The longest length the chip accepts, in nanoseconds.
        max_ns: u32,
    },
    /// A pulse needs more clock ticks than the hardware can count.
    TooManyTicks {
        /// The number of ticks needed.
        ticks: u32,
        /// The most ticks the hardware can count in one pulse.
        max_ticks: u32,
    },
    /// The SPI clock can't be used to fake this LED's signal.
    SpiClockUnusable {
        /// The SPI clock that was asked for, in hertz.
        spi_hz: u32,
        /// The closest usable clock below it, if there is one, in hertz.
        nearest_lower_hz: Option<u32>,
        /// The closest usable clock above it, if there is one, in hertz.
        nearest_higher_hz: Option<u32>,
    },
    /// The serial port's baud can't be used to fake this LED's signal.
    UartBaudUnusable {
        /// The baud that was asked for.
        baud: u32,
        /// The closest usable baud below it, if there is one.
        nearest_lower_baud: Option<u32>,
        /// The closest usable baud above it, if there is one.
        nearest_higher_baud: Option<u32>,
    },
}

/// Prints a rate in millions using integer math, never rounding it: one
/// decimal place for multiples of 100 000 ("3.0 MHz", "3.2 MHz"), otherwise
/// the exact value with trailing zeros trimmed ("3.35 MHz", "4.0005 MHz").
/// The unit is "MHz" for a clock or "Mbaud" for a serial port.
struct Mega(u32, &'static str);

/// A frequency in MHz (see [`Mega`]).
#[allow(non_snake_case)]
const fn Mhz(hz: u32) -> Mega {
    Mega(hz, "MHz")
}

/// A serial rate in Mbaud (see [`Mega`]).
#[allow(non_snake_case)]
const fn Mbaud(baud: u32) -> Mega {
    Mega(baud, "Mbaud")
}

impl fmt::Display for Mega {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unit = self.1;
        let whole = self.0 / 1_000_000;
        let mut rest = self.0 % 1_000_000;
        if rest % 100_000 == 0 {
            return write!(f, "{whole}.{} {unit}", rest / 100_000);
        }
        // Six decimal places is exact for whole hertz; drop trailing zeros.
        let mut digits = 6;
        while rest % 10 == 0 {
            rest /= 10;
            digits -= 1;
        }
        write!(f, "{whole}.{rest:0digits$} {unit}")
    }
}

impl fmt::Display for TimingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::PhaseOutOfRange {
                phase,
                actual_ns,
                min_ns,
                max_ns,
            } => {
                write!(
                    f,
                    "the {phase} would be {actual_ns} ns, but this LED needs {min_ns} to {max_ns} ns"
                )?;
                // The pulse is the typical length rounded to whole ticks, so
                // it misses the window only when a tick is too long. A finer
                // (faster) clock fixes both directions; a slower one never does.
                if actual_ns < min_ns {
                    f.write_str("; try a faster clock")
                } else {
                    f.write_str("; the clock is too coarse to time this pulse, try a faster clock")
                }
            }
            Self::TooManyTicks { ticks, max_ticks } => write!(
                f,
                "a pulse needs {ticks} clock ticks but the hardware can count only {max_ticks}; \
                 try a slower clock"
            ),
            Self::SpiClockUnusable {
                spi_hz,
                nearest_lower_hz,
                nearest_higher_hz,
            } => {
                write!(
                    f,
                    "an SPI clock of {} can't produce valid timing for this LED",
                    Mhz(spi_hz)
                )?;
                match (nearest_lower_hz, nearest_higher_hz) {
                    (Some(lo), Some(hi)) => write!(f, "; try {} or {}", Mhz(lo), Mhz(hi)),
                    (Some(hz), None) | (None, Some(hz)) => write!(f, "; try {}", Mhz(hz)),
                    (None, None) => f.write_str("; no SPI clock works for this LED"),
                }
            }
            Self::UartBaudUnusable {
                baud,
                nearest_lower_baud,
                nearest_higher_baud,
            } => {
                write!(
                    f,
                    "a serial port at {} can't produce valid timing for this LED",
                    Mbaud(baud)
                )?;
                match (nearest_lower_baud, nearest_higher_baud) {
                    (Some(lo), Some(hi)) => write!(f, "; try {} or {}", Mbaud(lo), Mbaud(hi)),
                    (Some(b), None) | (None, Some(b)) => write!(f, "; try {}", Mbaud(b)),
                    (None, None) => f.write_str("; no serial baud works for this LED"),
                }
            }
        }
    }
}

/// The output buffer you gave is too small for the signal.
///
/// # Example
///
/// ```
/// use ledkit_core::BufferTooSmall;
///
/// let e = BufferTooSmall { needed: 96 };
/// assert!(format!("{e}").contains("96"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferTooSmall {
    /// How many items the buffer must hold.
    pub needed: usize,
}

impl core::error::Error for TimingError {}

impl fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the output buffer is too small; it must hold at least {} items",
            self.needed
        )
    }
}

impl core::error::Error for BufferTooSmall {}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn uart_message_with_a_neighbour() {
        let e = TimingError::UartBaudUnusable {
            baud: 4_100_000,
            nearest_lower_baud: Some(4_000_000),
            nearest_higher_baud: None,
        };
        let msg = format!("{e}");
        assert!(
            msg.contains("4.1 Mbaud") && msg.contains("4.0 Mbaud"),
            "{msg}"
        );
    }
    #[test]
    fn uart_message_with_no_neighbours() {
        let e = TimingError::UartBaudUnusable {
            baud: 4_000_000,
            nearest_lower_baud: None,
            nearest_higher_baud: None,
        };
        let msg = format!("{e}");
        assert!(msg.contains("no serial baud works for this LED"), "{msg}");
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
    fn mhz_pads_and_never_truncates() {
        let e = TimingError::SpiClockUnusable {
            spi_hz: 2_050_000,
            nearest_lower_hz: Some(4_512_345),
            nearest_higher_hz: None,
        };
        let msg = format!("{e}");
        assert!(
            msg.contains("2.05 MHz") && msg.contains("4.512345 MHz"),
            "{msg}"
        );
    }
    #[test]
    fn mhz_prints_the_exact_clock() {
        for (hz, want) in [
            (4_000_500, "4.0005 MHz"),
            (1_950_000, "1.95 MHz"),
            (3_350_000, "3.35 MHz"),
            (4_000_000, "4.0 MHz"),
            (3_300_000, "3.3 MHz"),
            (1, "0.000001 MHz"),
            (12_345_670, "12.34567 MHz"),
        ] {
            assert_eq!(format!("{}", Mhz(hz)), want, "{hz} Hz");
        }
    }
    #[test]
    fn errors_implement_core_error() {
        fn is_error<E: core::error::Error>(_: &E) {}
        is_error(&TimingError::TooManyTicks {
            ticks: 2,
            max_ticks: 1,
        });
        is_error(&BufferTooSmall { needed: 1 });
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
