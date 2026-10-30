//! Turning pixels and timings into signals.

mod pulse;
mod spi;

pub use pulse::{BitPulse, PulseTicks};
pub use spi::SpiEncoding;

/// Clamps a `u64` into a `u32`, saturating at `u32::MAX`.
pub(crate) const fn saturate_u32(value: u64) -> u32 {
    if value > u32::MAX as u64 {
        u32::MAX
    } else {
        value as u32
    }
}
