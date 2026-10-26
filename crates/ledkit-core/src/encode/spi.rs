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
///
/// # Sizing a buffer at compile time
///
/// [`ClocklessTiming::spi_encoding`] and [`buffer_len`](Self::buffer_len)
/// are `const fn`s, so the buffer for a fixed strip and SPI clock can be an
/// ordinary array whose size is worked out when your program is compiled. A
/// clock that can't work stops the build instead of failing on the device.
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// const LED_COUNT: usize = 30;
/// const SPI_BUFFER_LEN: usize =
///     match ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000) {
///         Ok(encoding) => encoding.buffer_len(LED_COUNT * 3), // 3 bytes per RGB LED
///         Err(_) => panic!("this SPI clock can't drive a WS2812B"),
///     };
///
/// let spi_buffer = [0u8; SPI_BUFFER_LEN];
/// assert_eq!(spi_buffer.len(), 473);
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
const fn window_ok(phase: Phase, k: u64, spi_hz: u64) -> bool {
    let t = k * 1_000_000_000;
    phase.min_ns as u64 * spi_hz <= t && t <= phase.max_ns as u64 * spi_hz
}

/// How far `k` SPI bits are from the phase's typical length, in picoseconds.
const fn miss_ps(phase: Phase, k: u64, spi_hz: u64) -> u64 {
    (k * 1_000_000_000_000 / spi_hz).abs_diff(phase.typical_ns as u64 * 1000)
}

/// The SPI clocks suggested when one doesn't work: 1 MHz to 40 MHz in 100 kHz steps.
const GRID_STEP_HZ: u32 = 100_000;
const GRID_FIRST: u32 = 10;
const GRID_LAST: u32 = 400;

impl ClocklessTiming {
    /// Finds the best SPI bit pattern for an SPI clock of `spi_hz` hertz.
    ///
    /// Every LED bit is sent as `N` SPI bits (3 to 8), starting with some
    /// ones and finishing with zeros. All four pulses must fit the chip's
    /// allowed window exactly. Of the patterns that do, the one closest to
    /// the chip's typical timing wins.
    ///
    /// This is a `const fn`, so it also works at compile time (see
    /// [`SpiEncoding`] for sizing a buffer that way).
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
    pub const fn spi_encoding(&self, spi_hz: u32) -> Result<SpiEncoding, TimingError> {
        if let Some(e) = self.search(spi_hz) {
            return Ok(e);
        }
        Err(TimingError::SpiClockUnusable {
            spi_hz,
            nearest_lower_hz: self.nearest_working_clock(spi_hz, false),
            nearest_higher_hz: self.nearest_working_clock(spi_hz, true),
        })
    }

    /// The closest clock on the 100 kHz grid, below (or above) `spi_hz`, that works.
    const fn nearest_working_clock(&self, spi_hz: u32, above: bool) -> Option<u32> {
        let mut step = 0;
        while step <= GRID_LAST - GRID_FIRST {
            let i = if above {
                GRID_FIRST + step
            } else {
                GRID_LAST - step
            };
            let hz = i * GRID_STEP_HZ;
            let on_the_right_side = if above { hz > spi_hz } else { hz < spi_hz };
            if on_the_right_side && self.search(hz).is_some() {
                return Some(hz);
            }
            step += 1;
        }
        None
    }

    /// The best pattern for `spi_hz`, if any fits.
    const fn search(&self, spi_hz: u32) -> Option<SpiEncoding> {
        if spi_hz == 0 {
            return None;
        }
        let hz = spi_hz as u64;
        // (score, n, k0, k1) of the best pattern so far; ties go to the
        // smaller n, then k0, then k1.
        let mut best: Option<(u64, u8, u8, u8)> = None;
        let mut n: u8 = 3;
        while n <= 8 {
            let mut k0: u8 = 1;
            while k0 < n {
                let mut k1: u8 = k0 + 1;
                while k1 < n {
                    let (n64, k0_64, k1_64) = (n as u64, k0 as u64, k1 as u64);
                    let phases = [
                        (self.zero_high, k0_64),
                        (self.zero_low, n64 - k0_64),
                        (self.one_high, k1_64),
                        (self.one_low, n64 - k1_64),
                    ];
                    let mut fits = true;
                    let mut score: u64 = 0;
                    let mut i = 0;
                    while i < phases.len() {
                        let (phase, k) = phases[i];
                        if !window_ok(phase, k, hz) {
                            fits = false;
                        }
                        score += miss_ps(phase, k, hz);
                        i += 1;
                    }
                    let better = match best {
                        None => true,
                        // Scanning n, k0, k1 in increasing order means a
                        // tie on score keeps the earlier (smaller) pattern.
                        Some((best_score, _, _, _)) => score < best_score,
                    };
                    if fits && better {
                        best = Some((score, n, k0, k1));
                    }
                    k1 += 1;
                }
                k0 += 1;
            }
            n += 1;
        }
        let Some((_, n, k0, k1)) = best else {
            return None;
        };
        let reset_bytes = (self.reset_us as u64 * hz).div_ceil(8_000_000);
        Some(SpiEncoding {
            bits_per_led_bit: n,
            zero_high_bits: k0,
            one_high_bits: k1,
            reset_bytes: if reset_bytes > u32::MAX as u64 {
                u32::MAX
            } else {
                reset_bytes as u32
            },
        })
    }
}

impl SpiEncoding {
    /// How many bytes [`encode`](Self::encode) needs for `wire_bytes` bytes of LED data.
    ///
    /// That is one leading zero byte, the packed pattern bits, and the reset
    /// bytes. It is a `const fn`, so it can size an array at compile time.
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
    pub const fn buffer_len(&self, wire_bytes: usize) -> usize {
        let bits = wire_bytes * 8 * self.bits_per_led_bit as usize;
        1 + bits.div_ceil(8) + self.reset_bytes as usize
    }

    /// Turns LED data bytes into the SPI bytes to send. Returns how many bytes were written.
    ///
    /// The output starts with a zero byte (it keeps the line low if your SPI
    /// pin idles high), then every LED bit as its pattern, then the reset
    /// zeros. `out` is fully overwritten up to the returned length.
    ///
    /// The buffer size is worked out from the length `wire` reports
    /// ([`ExactSizeIterator::len`]), and at most that many bytes are taken
    /// from it, so even an iterator that reports the wrong length can never
    /// write past [`buffer_len`](Self::buffer_len). If it yields fewer bytes
    /// than it reported, the rest of the data is left as zeros.
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
        let len = wire.len();
        let needed = self.buffer_len(len);
        if out.len() < needed {
            return Err(BufferTooSmall { needed });
        }
        out[..needed].fill(0);
        // Bit position in `out`; byte 0 is the lead-in, so data starts at bit 8.
        let mut pos: usize = 8;
        for byte in wire.take(len) {
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
    fn buffer_size_at_compile_time() {
        const BUF: usize = match ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000) {
            Ok(e) => e.buffer_len(30 * 3),
            Err(_) => panic!("bad clock"),
        };
        let buf = [0u8; BUF];
        assert_eq!(buf.len(), 1 + 360 + 112);
    }

    #[test]
    fn const_and_runtime_results_agree() {
        const AT_4_MHZ: Result<SpiEncoding, TimingError> =
            ClocklessTiming::of::<Ws2812b>().spi_encoding(4_000_000);
        const AT_20_MHZ: Result<SpiEncoding, TimingError> =
            ClocklessTiming::of::<Ws2812b>().spi_encoding(20_000_000);
        assert_eq!(AT_4_MHZ, enc::<Ws2812b>(4_000_000));
        assert_eq!(AT_20_MHZ, enc::<Ws2812b>(20_000_000));
    }

    /// An iterator that claims fewer bytes than it really yields.
    struct Liar(u8);

    impl Iterator for Liar {
        type Item = u8;
        fn next(&mut self) -> Option<u8> {
            self.0 = self.0.wrapping_add(1);
            Some(0xFF)
        }
    }

    impl ExactSizeIterator for Liar {
        fn len(&self) -> usize {
            1
        }
    }

    #[test]
    fn a_dishonest_length_never_writes_past_the_buffer_length() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        let mut out = [0xAAu8; 200];
        assert_eq!(e.encode(Liar(0), &mut out), Ok(117));
        let mut honest = [0u8; 117];
        assert_eq!(e.encode([0xFFu8], &mut honest), Ok(117));
        assert_eq!(out[..117], honest);
        assert!(out[117..].iter().all(|&b| b == 0xAA));
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
