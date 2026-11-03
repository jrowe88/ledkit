//! The error type of this crate.

use core::fmt;
use ledkit_core::{BufferTooSmall, TimingError};

/// What can go wrong while setting up or sending with a
/// [`PioOutput`](crate::PioOutput) or [`PioDmaOutput`](crate::PioDmaOutput).
///
/// # Example
///
/// ```ignore
/// match PioOutput::<Ws2812b, _, _, _, _>::new(&mut pio, sm0, pin, sys_hz, timer) {
///     Ok(output) => { /* use it */ }
///     Err(ledkit_rp::Error::NoProgramSpace) => { /* use the other PIO block */ }
///     Err(other) => panic!("{other}"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The LED's pulses can't be made from this system clock with whole clock
    /// dividers. Not expected for the chips in the catalogue at the usual
    /// 125 MHz (RP2040) or 150 MHz (RP2350); check the `sys_hz` you passed.
    Timing(TimingError),
    /// The PIO block has no room left for ledkit's 4-instruction program (it holds
    /// 32 instructions in all). Use the other PIO block, or fewer programs on this
    /// one.
    NoProgramSpace,
    /// The DMA buffer you gave is too small for the frame; nothing was sent.
    BufferTooSmall(BufferTooSmall),
    /// The DMA channel and state machine are missing. This only happens if a panic
    /// unwound in the middle of a send. Create the output again.
    ChannelLost,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timing(e) => fmt::Display::fmt(e, f),
            Self::NoProgramSpace => f.write_str(
                "the PIO block has no room for ledkit's 4-instruction program; \
                 use the other PIO block",
            ),
            Self::BufferTooSmall(e) => fmt::Display::fmt(e, f),
            Self::ChannelLost => f.write_str("the DMA channel was lost; create the output again"),
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
