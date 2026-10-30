//! The error type of this crate.

use core::fmt;
use esp_hal::rmt::{ConfigError, Error as RmtError};
use ledkit_core::{BufferTooSmall, TimingError};

/// What can go wrong while setting up or sending with an
/// [`RmtOutput`](crate::RmtOutput).
///
/// # Example
///
/// ```ignore
/// match RmtOutput::<Ws2812b, _, _>::new(rmt.channel0, peripherals.GPIO8, rmt_hz, buffer) {
///     Ok(output) => { /* use it */ }
///     Err(ledkit_esp::Error::ClockNotUsable { rmt_hz }) => { /* pick 80 MHz for Rmt::new */ }
///     Err(other) => panic!("{other}"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The LED's pulse lengths can't be made at the RMT tick rate. Not expected
    /// for the chips in the catalogue.
    Timing(TimingError),
    /// The RMT clock you passed can't be divided down to the 10 MHz tick ledkit
    /// uses: it must be 10 MHz times a whole number from 1 to 255 (for example
    /// 80 MHz). Pass the RMT's real frequency: the value you gave `Rmt::new`, or
    /// `rmt.frequency().as_hz()`.
    ClockNotUsable {
        /// The RMT clock that was given, in hertz.
        rmt_hz: u32,
    },
    /// The buffer you gave is too small for the frame; nothing was sent.
    BufferTooSmall(BufferTooSmall),
    /// `esp-hal` refused the channel setup (for example, no free RMT memory).
    Config(ConfigError),
    /// The RMT reported an error while sending.
    Rmt(RmtError),
    /// The RMT channel is missing. A failed send always hands the channel back, so
    /// this defensive state only appears if a panic unwound in the middle of a send.
    /// Create the output again.
    ChannelLost,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timing(e) => fmt::Display::fmt(e, f),
            Self::ClockNotUsable { rmt_hz } => write!(
                f,
                "the RMT clock of {rmt_hz} Hz can't be divided to 10 MHz ticks; \
                 pass the RMT's real frequency (the value you gave Rmt::new, for example                  80 MHz), which must be 10 MHz times a whole number from 1 to 255"
            ),
            Self::BufferTooSmall(e) => fmt::Display::fmt(e, f),
            Self::Config(e) => write!(f, "RMT channel setup failed: {e}"),
            Self::Rmt(e) => write!(f, "RMT transmit failed: {e}"),
            Self::ChannelLost => f.write_str("the RMT channel was lost; create the output again"),
        }
    }
}

impl core::error::Error for Error {}

impl From<TimingError> for Error {
    fn from(e: TimingError) -> Self {
        Self::Timing(e)
    }
}

impl From<BufferTooSmall> for Error {
    fn from(e: BufferTooSmall) -> Self {
        Self::BufferTooSmall(e)
    }
}

impl From<ConfigError> for Error {
    fn from(e: ConfigError) -> Self {
        Self::Config(e)
    }
}

impl From<RmtError> for Error {
    fn from(e: RmtError) -> Self {
        Self::Rmt(e)
    }
}
