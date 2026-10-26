//! SPI bit patterns that imitate a clockless LED signal.

use crate::{BufferTooSmall, ClocklessTiming, Phase, TimingError};

/// How to fake a clockless LED signal with an SPI peripheral.
///
/// An SPI peripheral sends plain bits at a steady clock. If each LED bit is
/// sent as a few SPI bits, some high then some low, the line looks like the
/// pulses a NeoPixel expects. Get one from [`ClocklessTiming::spi_encoding`],
/// which picks the pattern that fits the SPI clock you actually have.
///
/// # Example
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let enc = ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000).unwrap();
/// assert_eq!(enc.bits_per_led_bit, 4);
///
/// let mut out = [0u8; 117];
/// let used = enc.encode([0b1010_0000u8], &mut out).unwrap();
/// assert_eq!(used, 117);
/// assert_eq!(&out[..5], &[0x00, 0xC8, 0xC8, 0x88, 0x88]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiEncoding {
    /// How many SPI bits are used to send one LED bit.
    pub bits_per_led_bit: u8,
    /// How many of those SPI bits are high (ones) for a `0` LED bit.
    pub zero_high_bits: u8,
    /// How many of those SPI bits are high (ones) for a `1` LED bit.
    pub one_high_bits: u8,
    /// How many zero bytes to send at the end so the LEDs latch the colors.
    pub reset_bytes: u32,
}

/// Whether `k` SPI bits last between the phase's minimum and maximum, exactly.
fn window_ok(phase: Phase, k: u64, spi_hz: u64) -> bool {
    let t = k * 1_000_000_000;
    u64::from(phase.min_ns) * spi_hz <= t && t <= u64::from(phase.max_ns) * spi_hz
}

/// How far `k` SPI bits are from the phase's typical length, in picoseconds.
fn miss_ps(phase: Phase, k: u64, spi_hz: u64) -> u64 {
    (k * 1_000_000_000_000 / spi_hz).abs_diff(u64::from(phase.typical_ns) * 1000)
}

impl ClocklessTiming {
    /// Finds the best SPI bit pattern for an SPI clock of `spi_hz` hertz.
    ///
    /// Every LED bit is sent as `N` SPI bits (3 to 8), starting with some
    /// ones and finishing with zeros. All four pulses must fit the chip's
    /// allowed window exactly. Of the patterns that do, the one closest to
    /// the chip's typical timing wins.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::SpiClockUnusable`] if no pattern fits, along
    /// with the nearest clocks (in steps of 100 kHz, from 1 MHz to 40 MHz)
    /// that do work.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let enc = ClocklessTiming::of::<Ws2812b>().spi_encoding(4_000_000)?;
    /// assert_eq!((enc.bits_per_led_bit, enc.zero_high_bits, enc.one_high_bits), (5, 1, 3));
    /// # Ok::<(), ledkit_core::TimingError>(())
    /// ```
    pub fn spi_encoding(&self, spi_hz: u32) -> Result<SpiEncoding, TimingError> {
        if let Some(e) = self.search(spi_hz) {
            return Ok(e);
        }
        let grid = (10..=400).map(|i| i * 100_000_u32);
        let nearest_lower_hz = grid
            .clone()
            .rev()
            .filter(|&hz| hz < spi_hz)
            .find(|&hz| self.search(hz).is_some());
        let nearest_higher_hz = grid
            .filter(|&hz| hz > spi_hz)
            .find(|&hz| self.search(hz).is_some());
        Err(TimingError::SpiClockUnusable {
            spi_hz,
            nearest_lower_hz,
            nearest_higher_hz,
        })
    }

    /// The best pattern for `spi_hz`, if any fits.
    fn search(&self, spi_hz: u32) -> Option<SpiEncoding> {
        if spi_hz == 0 {
            return None;
        }
        let hz = u64::from(spi_hz);
        let mut best: Option<(u64, u8, u8, u8)> = None;
        for n in 3_u8..=8 {
            for k0 in 1..n {
                for k1 in (k0 + 1)..n {
                    let (n64, k0_64, k1_64) = (u64::from(n), u64::from(k0), u64::from(k1));
                    let phases = [
                        (self.zero_high, k0_64),
                        (self.zero_low, n64 - k0_64),
                        (self.one_high, k1_64),
                        (self.one_low, n64 - k1_64),
                    ];
                    if !phases.iter().all(|&(p, k)| window_ok(p, k, hz)) {
                        continue;
                    }
                    let score: u64 = phases.iter().map(|&(p, k)| miss_ps(p, k, hz)).sum();
                    let cand = (score, n, k0, k1);
                    if best.is_none_or(|b| cand < b) {
                        best = Some(cand);
                    }
                }
            }
        }
        let (_, n, k0, k1) = best?;
        let reset_bytes = (u64::from(self.reset_us) * hz).div_ceil(8_000_000);
        Some(SpiEncoding {
            bits_per_led_bit: n,
            zero_high_bits: k0,
            one_high_bits: k1,
            reset_bytes: u32::try_from(reset_bytes).unwrap_or(u32::MAX),
        })
    }
}

impl SpiEncoding {
    /// How many bytes [`encode`](Self::encode) needs for `wire_bytes` bytes of LED data.
    ///
    /// That is one leading zero byte, the packed pattern bits, and the reset bytes.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let enc = ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000).unwrap();
    /// assert_eq!(enc.buffer_len(3), 1 + 12 + 112);
    /// ```
    #[must_use]
    pub fn buffer_len(&self, wire_bytes: usize) -> usize {
        let bits = wire_bytes * 8 * usize::from(self.bits_per_led_bit);
        1 + bits.div_ceil(8) + self.reset_bytes as usize
    }

    /// Turns LED data bytes into the SPI bytes to send. Returns how many bytes were written.
    ///
    /// The output starts with a zero byte (it keeps the line low if your SPI
    /// pin idles high), then every LED bit as its pattern, then the reset
    /// zeros. `out` is fully overwritten up to the returned length.
    ///
    /// # Errors
    ///
    /// Returns [`BufferTooSmall`] if `out` is shorter than [`buffer_len`](Self::buffer_len).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let enc = ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000).unwrap();
    /// let mut out = [0u8; 117];
    /// assert_eq!(enc.encode([0xFFu8], &mut out), Ok(117));
    /// ```
    pub fn encode(
        &self,
        wire: impl IntoIterator<Item = u8, IntoIter: ExactSizeIterator>,
        out: &mut [u8],
    ) -> Result<usize, BufferTooSmall> {
        let wire = wire.into_iter();
        let needed = self.buffer_len(wire.len());
        if out.len() < needed {
            return Err(BufferTooSmall { needed });
        }
        out[..needed].fill(0);
        // Bit position in `out`; byte 0 is the lead-in, so data starts at bit 8.
        let mut pos: usize = 8;
        for byte in wire {
            for i in (0..8).rev() {
                let ones = if (byte >> i) & 1 == 1 {
                    self.one_high_bits
                } else {
                    self.zero_high_bits
                };
                for j in 0..self.bits_per_led_bit {
                    if j < ones {
                        out[pos / 8] |= 0x80 >> (pos % 8);
                    }
                    pos += 1;
                }
            }
        }
        Ok(needed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Apa106, Chip, Sk6812, Ws2811Slow, Ws2812b, Ws2813};

    fn enc<C: Chip>(hz: u32) -> Result<SpiEncoding, TimingError> {
        ClocklessTiming::of::<C>().spi_encoding(hz)
    }

    fn shape(e: SpiEncoding) -> (u8, u8, u8) {
        (e.bits_per_led_bit, e.zero_high_bits, e.one_high_bits)
    }

    #[test]
    fn ws2812b_common_clocks() {
        assert_eq!(enc::<Ws2812b>(2_400_000).map(shape), Ok((4, 1, 2)));
        assert_eq!(enc::<Ws2812b>(3_200_000).map(shape), Ok((4, 1, 2)));
        assert_eq!(enc::<Ws2812b>(4_000_000).map(shape), Ok((5, 1, 3)));
        assert_eq!(enc::<Ws2812b>(6_400_000).map(shape), Ok((8, 2, 5)));
    }

    #[test]
    fn other_chips() {
        assert_eq!(enc::<Ws2813>(2_400_000).unwrap().bits_per_led_bit, 3);
        assert_eq!(enc::<Ws2811Slow>(2_400_000).map(shape), Ok((6, 1, 3)));
        assert_eq!(enc::<Apa106>(3_200_000).map(shape), Ok((5, 1, 4)));
    }

    #[test]
    fn too_slow_suggests_a_working_clock() {
        assert_eq!(
            enc::<Ws2812b>(1_000_000),
            Err(TimingError::SpiClockUnusable {
                spi_hz: 1_000_000,
                nearest_lower_hz: None,
                nearest_higher_hz: Some(2_000_000)
            })
        );
    }

    #[test]
    fn too_fast_suggests_a_working_clock() {
        assert_eq!(
            enc::<Ws2812b>(20_000_000),
            Err(TimingError::SpiClockUnusable {
                spi_hz: 20_000_000,
                nearest_lower_hz: Some(6_800_000),
                nearest_higher_hz: None
            })
        );
    }

    #[test]
    fn gap_in_the_middle_suggests_both_sides() {
        assert_eq!(
            enc::<Ws2813>(4_000_000),
            Err(TimingError::SpiClockUnusable {
                spi_hz: 4_000_000,
                nearest_lower_hz: Some(3_300_000),
                nearest_higher_hz: Some(4_500_000)
            })
        );
    }

    #[test]
    fn zero_clock_has_no_encoding() {
        assert!(matches!(
            enc::<Ws2812b>(0),
            Err(TimingError::SpiClockUnusable { spi_hz: 0, .. })
        ));
    }

    #[test]
    fn reset_bytes() {
        assert_eq!(enc::<Ws2812b>(3_200_000).unwrap().reset_bytes, 112);
        assert_eq!(enc::<Sk6812>(3_200_000).unwrap().reset_bytes, 32);
    }

    #[test]
    fn encodes_four_bit_patterns() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        let mut out = [0xAAu8; 117];
        assert_eq!(e.buffer_len(1), 117);
        assert_eq!(e.encode([0b1010_0000u8], &mut out), Ok(117));
        assert_eq!(&out[..5], &[0x00, 0xC8, 0xC8, 0x88, 0x88]);
        assert!(out[5..].iter().all(|&b| b == 0));
    }

    #[test]
    fn encodes_three_bit_patterns_across_byte_boundaries() {
        let e = enc::<Ws2813>(2_400_000).unwrap();
        let mut out = [0u8; 97];
        assert_eq!(e.encode([0xFF, 0x00], &mut out), Ok(97));
        assert_eq!(&out[..7], &[0x00, 0xDB, 0x6D, 0xB6, 0x92, 0x49, 0x24]);
    }

    #[test]
    fn slices_work_too() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        let data = [0b1010_0000u8];
        let mut out = [0u8; 117];
        assert_eq!(e.encode(data.iter().copied(), &mut out), Ok(117));
    }

    #[test]
    fn small_buffer_is_an_error() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        assert_eq!(
            e.encode([0u8; 3], &mut [0u8; 10]),
            Err(BufferTooSmall {
                needed: e.buffer_len(3)
            })
        );
    }
}
