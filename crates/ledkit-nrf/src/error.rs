//! The error type of this crate.

use core::fmt;
use embassy_nrf::pwm;
use ledkit_core::BufferTooSmall;

/// What can go wrong while setting up or sending with a
/// [`PwmOutput`](crate::PwmOutput).
///
/// # Example
///
/// ```ignore
/// match strip.show() {
///     Ok(()) => {}
///     Err(ledkit_nrf::Error::BufferTooSmall(e)) => { /* size it with buffer_len */ }
///     Err(other) => panic!("{other}"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The buffer you gave is too small for the frame; nothing was sent.
    BufferTooSmall(BufferTooSmall),
    /// The frame needs more duty words than one PWM sequence can play; nothing was
    /// sent. Drive fewer LEDs from this output (about 1350 RGB LEDs fit).
    FrameTooLong {
        /// The words the frame needs.
        words: usize,
        /// The most one frame can have.
        max: usize,
    },
    /// `embassy-nrf` refused the PWM setup or sequence (for example, a buffer that is
    /// not in RAM).
    Pwm(pwm::Error),
    /// The PWM didn't report the end of the frame in time. Not expected; the PWM has
    /// been stopped.
    Timeout,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall(e) => fmt::Display::fmt(e, f),
            Self::FrameTooLong { words, max } => write!(
                f,
                "the frame needs {words} PWM words but one PWM sequence holds at most \
                 {max} (about 1350 RGB LEDs); drive fewer LEDs from this output"
            ),
            Self::Pwm(pwm::Error::BufferNotInRAM) => {
                f.write_str("the frame buffer must be in RAM (EasyDMA can't read flash)")
            }
            Self::Pwm(e) => write!(f, "the PWM refused the frame: {e:?}"),
            Self::Timeout => f.write_str("the PWM didn't finish the frame in time"),
        }
    }
}

impl core::error::Error for Error {}

impl From<BufferTooSmall> for Error {
    fn from(e: BufferTooSmall) -> Self {
        Self::BufferTooSmall(e)
    }
}

impl From<pwm::Error> for Error {
    fn from(e: pwm::Error) -> Self {
        Self::Pwm(e)
    }
}
