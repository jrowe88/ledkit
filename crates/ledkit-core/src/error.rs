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
}

/// Prints a frequency in MHz with one decimal place, using integer math.
struct Mhz(u32);

impl fmt::Display for Mhz {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{} MHz",
            self.0 / 1_000_000,
            (self.0 % 1_000_000) / 100_000
        )
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
            } => write!(
                f,
                "the {phase} would be {actual_ns} ns, but this LED needs {min_ns} to {max_ns} ns; \
                 try a different clock speed"
            ),
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

impl fmt::Display for BufferTooSmall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the output buffer is too small; it must hold at least {} items",
            self.needed
        )
    }
}
