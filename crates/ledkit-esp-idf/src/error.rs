//! The error type of this crate.

use core::fmt;
use esp_idf_hal::sys::EspError;
use ledkit_core::TimingError;

/// What can go wrong while setting up an [`RmtOutput`](crate::RmtOutput).
///
/// Sending a frame can only fail with the ESP-IDF's own [`EspError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The LED's pulse lengths can't be made at the RMT tick rate. Not expected for
    /// the chips in the catalogue.
    Timing(TimingError),
    /// ESP-IDF refused: for example, every RMT channel is already in use.
    Esp(EspError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timing(e) => fmt::Display::fmt(e, f),
            Self::Esp(e) => write!(f, "ESP-IDF error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<TimingError> for Error {
    fn from(e: TimingError) -> Self {
        Self::Timing(e)
    }
}

impl From<EspError> for Error {
    fn from(e: EspError) -> Self {
        Self::Esp(e)
    }
}
