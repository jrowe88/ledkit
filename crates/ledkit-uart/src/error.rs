//! The error type for sending to LEDs over a serial port.

use core::fmt;
use ledkit_core::BufferTooSmall;

/// Something went wrong sending a frame over the serial port.
///
/// `E` is your serial port's error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error<E> {
    /// The serial port reported an error.
    Uart(E),
    /// The buffer you gave the output is too small for this many LEDs.
    BufferTooSmall(BufferTooSmall),
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uart(e) => write!(f, "serial port error: {e:?}"),
            Self::BufferTooSmall(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl<E: fmt::Debug> core::error::Error for Error<E> {}

impl<E> From<BufferTooSmall> for Error<E> {
    fn from(e: BufferTooSmall) -> Self {
        Self::BufferTooSmall(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_readable() {
        let uart: Error<u8> = Error::Uart(7);
        assert_eq!(format!("{uart}"), "serial port error: 7");
        let small: Error<u8> = BufferTooSmall { needed: 9 }.into();
        assert_eq!(
            format!("{small}"),
            format!("{}", BufferTooSmall { needed: 9 })
        );
    }

    #[test]
    fn is_a_std_style_error() {
        fn is_error<E: core::error::Error>(_: &E) {}
        is_error(&Error::<u8>::Uart(1));
    }
}
