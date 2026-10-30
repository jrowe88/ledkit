//! The error type of this crate.

use core::fmt;
use ledkit_core::BufferTooSmall;

/// What can go wrong while sending a frame over SPI.
///
/// `E` is the error type of your SPI bus.
///
/// # Example
///
/// ```
/// use ledkit_core::BufferTooSmall;
/// use ledkit_spi::Error;
///
/// let e: Error<()> = BufferTooSmall { needed: 96 }.into();
/// assert!(format!("{e}").contains("96"));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
    /// The SPI bus reported an error.
    Spi(E),
    /// The buffer you gave is too small for the frame; nothing was sent.
    BufferTooSmall(BufferTooSmall),
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spi(e) => write!(f, "SPI bus error: {e:?}"),
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
        let spi: Error<u8> = Error::Spi(7);
        assert_eq!(format!("{spi}"), "SPI bus error: 7");
        let small: Error<u8> = BufferTooSmall { needed: 9 }.into();
        assert_eq!(
            format!("{small}"),
            format!("{}", BufferTooSmall { needed: 9 })
        );
    }

    #[test]
    fn is_a_std_style_error() {
        fn is_error<E: core::error::Error>(_: &E) {}
        is_error(&Error::<u8>::Spi(1));
    }
}
