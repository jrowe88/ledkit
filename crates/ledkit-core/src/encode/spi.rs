//! SPI bit patterns that imitate a clockless LED signal.

use super::{miss_ps, saturate_u32, window_ok};
use crate::{BufferTooSmall, ClocklessTiming, TimingError};

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
        self.spi_encoding_checked(spi_hz, false)
    }

    /// Like [`spi_encoding`](Self::spi_encoding), but only patterns whose
    /// length is a whole half byte or byte (4 or 8 SPI bits per LED bit) qualify.
    ///
    /// Why this matters: some SPI peripherals, such as the one in many
    /// microcontrollers when driven a byte at a time, leave a small gap
    /// between bytes. A gap in the middle of an LED bit would stretch its
    /// pulse and corrupt the color. With 4 or 8 SPI bits per LED bit, every LED
    /// bit starts at a byte or half-byte boundary, and each pattern ends in
    /// low bits, so the gap only ever lands where the line is already low
    /// (between LED bits). The cost is that fewer SPI clocks work.
    ///
    /// Everything else matches [`spi_encoding`](Self::spi_encoding): the
    /// pulses must fit the chip's window exactly, the closest to typical
    /// timing wins, and this is a `const fn`.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::SpiClockUnusable`] if no byte-aligned pattern
    /// fits. The suggested nearest clocks are byte-aligned ones too.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let timing = ClocklessTiming::of::<Ws2812b>();
    /// // 3.2 MHz gives 4 SPI bits per LED bit, which is byte-aligned.
    /// let enc = timing.spi_encoding_byte_aligned(3_200_000)?;
    /// assert_eq!(enc.bits_per_led_bit, 4);
    /// // At 4 MHz the plain search packs 5 bits per LED bit, which isn't
    /// // byte-aligned, so this one uses 8 instead.
    /// assert_eq!(timing.spi_encoding(4_000_000)?.bits_per_led_bit, 5);
    /// assert_eq!(timing.spi_encoding_byte_aligned(4_000_000)?.bits_per_led_bit, 8);
    /// # Ok::<(), ledkit_core::TimingError>(())
    /// ```
    pub const fn spi_encoding_byte_aligned(&self, spi_hz: u32) -> Result<SpiEncoding, TimingError> {
        self.spi_encoding_checked(spi_hz, true)
    }

    /// The shared body of both public searches.
    const fn spi_encoding_checked(
        &self,
        spi_hz: u32,
        byte_aligned: bool,
    ) -> Result<SpiEncoding, TimingError> {
        if let Some(e) = self.search(spi_hz, byte_aligned) {
            return Ok(e);
        }
        Err(TimingError::SpiClockUnusable {
            spi_hz,
            nearest_lower_hz: self.nearest_working_clock(spi_hz, false, byte_aligned),
            nearest_higher_hz: self.nearest_working_clock(spi_hz, true, byte_aligned),
        })
    }

    /// The closest clock on the 100 kHz grid, below (or above) `spi_hz`, that works.
    const fn nearest_working_clock(
        &self,
        spi_hz: u32,
        above: bool,
        byte_aligned: bool,
    ) -> Option<u32> {
        let mut step = 0;
        while step <= GRID_LAST - GRID_FIRST {
            let i = if above {
                GRID_FIRST + step
            } else {
                GRID_LAST - step
            };
            let hz = i * GRID_STEP_HZ;
            let on_the_right_side = if above { hz > spi_hz } else { hz < spi_hz };
            if on_the_right_side && self.search(hz, byte_aligned).is_some() {
                return Some(hz);
            }
            step += 1;
        }
        None
    }

    /// The best pattern for `spi_hz`, if any fits. With `byte_aligned`, only 4 or 8 bits per LED bit count.
    const fn search(&self, spi_hz: u32, byte_aligned: bool) -> Option<SpiEncoding> {
        if spi_hz == 0 {
            return None;
        }
        let hz = spi_hz as u64;
        // (score, n, k0, k1) of the best pattern so far; ties go to the
        // smaller n, then k0, then k1.
        let mut best: Option<(u64, u8, u8, u8)> = None;
        let mut n: u8 = 3;
        while n <= 8 {
            let allowed = !byte_aligned || n == 4 || n == 8;
            let mut k0: u8 = 1;
            while allowed && k0 < n {
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
            reset_bytes: saturate_u32(reset_bytes),
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
            self.pack_byte(byte, &mut pos, out);
        }
        Ok(needed)
    }

    /// Like [`encode`](Self::encode), but for any iterator, including one that
    /// can't say how long it is (such as the one a `smart-leds` strip hands over).
    ///
    /// The bytes written and the length returned are exactly what
    /// [`encode`](Self::encode) gives for the same data. The input is read once, so
    /// the size isn't known until the end. If `out` fills up first, the rest
    /// of the input is only counted, and the error says how big `out` should
    /// have been. What `out` holds after an error is unspecified.
    ///
    /// # Errors
    ///
    /// Returns [`BufferTooSmall`] if `out` is shorter than
    /// [`buffer_len`](Self::buffer_len) of the number of bytes the iterator yielded.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, Ws2812b};
    ///
    /// let enc = ClocklessTiming::of::<Ws2812b>().spi_encoding(3_200_000).unwrap();
    /// let mut out = [0u8; 117];
    /// // `filter` hides the length, which `encode` would not accept.
    /// let used = enc.encode_from_iter([0xFFu8].into_iter().filter(|_| true), &mut out);
    /// assert_eq!(used, Ok(117));
    /// ```
    pub fn encode_from_iter(
        &self,
        wire: impl IntoIterator<Item = u8>,
        out: &mut [u8],
    ) -> Result<usize, BufferTooSmall> {
        if let Some(lead_in) = out.first_mut() {
            *lead_in = 0;
        }
        let mut pos: usize = 8;
        let mut wire_bytes: usize = 0;
        for byte in wire {
            self.pack_byte(byte, &mut pos, out);
            wire_bytes += 1;
        }
        let needed = self.buffer_len(wire_bytes);
        if out.len() < needed {
            return Err(BufferTooSmall { needed });
        }
        // 8 pattern bits per data bit always end on a byte boundary; the rest is reset.
        out[pos / 8..needed].fill(0);
        Ok(needed)
    }

    /// Writes the pattern for one LED data byte at bit position `pos`, then moves `pos` on.
    ///
    /// A byte of `out` is cleared the moment the first bit lands in it, so `out`
    /// needn't be zeroed beforehand. Bits that fall past the end of `out` are
    /// skipped, but `pos` still moves, so the caller can learn the full size.
    fn pack_byte(self, byte: u8, pos: &mut usize, out: &mut [u8]) {
        for i in (0..8).rev() {
            let ones = if (byte >> i) & 1 == 1 {
                self.one_high_bits
            } else {
                self.zero_high_bits
            };
            for j in 0..self.bits_per_led_bit {
                if let Some(slot) = out.get_mut(*pos / 8) {
                    if *pos % 8 == 0 {
                        *slot = 0;
                    }
                    if j < ones {
                        *slot |= 0x80 >> (*pos % 8);
                    }
                }
                *pos += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Apa106, Chip, Sk6812, Sk6812Rgbw, Ws2811, Ws2811Slow, Ws2812, Ws2812b, Ws2813, Ws2815,
    };

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
                nearest_higher_hz: Some(2_300_000)
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

    /// Reads bit `pos` of `out`, most significant bit first.
    fn bit(out: &[u8], pos: usize) -> bool {
        out[pos / 8] & (0x80 >> (pos % 8)) != 0
    }

    #[test]
    fn every_pattern_length_encodes_odd_length_data() {
        let cases: [(SpiEncoding, u8); 6] = [
            (enc::<Ws2813>(2_400_000).unwrap(), 3),
            (enc::<Ws2812b>(3_200_000).unwrap(), 4),
            (enc::<Ws2812b>(4_000_000).unwrap(), 5),
            (enc::<Ws2812b>(4_500_000).unwrap(), 6),
            (enc::<Ws2812b>(4_800_000).unwrap(), 7),
            (enc::<Ws2812b>(6_400_000).unwrap(), 8),
        ];
        let data = [0xA5u8, 0x3C, 0xFF, 0x00, 0x81];
        for (e, n) in cases {
            assert_eq!(e.bits_per_led_bit, n);
            for len in [1, 3, 5] {
                let wire = &data[..len];
                let mut out = [0xAAu8; 400];
                let used = e.encode(wire.iter().copied(), &mut out).unwrap();
                assert_eq!(used, e.buffer_len(len), "n {n} len {len}");
                assert_eq!(out[0], 0, "lead-in byte, n {n}");
                // Each LED bit is `ones` high SPI bits, then low ones.
                let mut pos = 8;
                for &byte in wire {
                    for i in (0..8).rev() {
                        let ones = if (byte >> i) & 1 == 1 {
                            e.one_high_bits
                        } else {
                            e.zero_high_bits
                        };
                        for j in 0..n {
                            assert_eq!(bit(&out, pos), j < ones, "n {n} len {len} pos {pos}");
                            pos += 1;
                        }
                    }
                }
                // 8 * n SPI bits per data byte always fill whole bytes, so
                // the data ends on a byte boundary; everything after it
                // (the reset bytes) must be zero, up to the returned length.
                assert_eq!(pos % 8, 0);
                let data_end = pos / 8;
                assert_eq!(used - data_end, e.reset_bytes as usize);
                assert!(
                    out[data_end..used].iter().all(|&b| b == 0),
                    "n {n} len {len}"
                );
                assert!(out[used..].iter().all(|&b| b == 0xAA), "n {n} len {len}");
            }
        }
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

    fn aligned<C: Chip>(hz: u32) -> Result<SpiEncoding, TimingError> {
        ClocklessTiming::of::<C>().spi_encoding_byte_aligned(hz)
    }

    #[test]
    fn byte_aligned_matches_packed_where_packed_is_already_aligned() {
        for hz in [2_400_000, 3_200_000, 6_400_000] {
            assert_eq!(aligned::<Ws2812b>(hz), enc::<Ws2812b>(hz));
        }
    }

    #[test]
    fn byte_aligned_tolerates_a_rounded_clock() {
        let at_3_2 = aligned::<Ws2812b>(3_200_000).unwrap();
        for hz in [3_150_000, 3_250_000] {
            let e = aligned::<Ws2812b>(hz).unwrap();
            assert_eq!(shape(e), shape(at_3_2));
        }
    }

    #[test]
    fn byte_aligned_too_slow_suggests_an_aligned_clock() {
        let Err(TimingError::SpiClockUnusable {
            nearest_lower_hz,
            nearest_higher_hz: Some(up),
            ..
        }) = aligned::<Ws2812b>(1_000_000)
        else {
            panic!("1 MHz must fail with a suggestion")
        };
        assert_eq!(nearest_lower_hz, None);
        let e = aligned::<Ws2812b>(up).unwrap();
        assert!(matches!(e.bits_per_led_bit, 4 | 8));
    }

    /// Over the whole grid, for every clockless chip in the catalogue, a
    /// byte-aligned encoding has 4 or 8 bits per LED bit, a packed encoding
    /// also exists at that clock, and every encoded data byte ends in a low bit.
    fn check_bytes_end_low<C: Chip>() {
        for step in GRID_FIRST..=GRID_LAST {
            let hz = step * GRID_STEP_HZ;
            if let Ok(e) = aligned::<C>(hz) {
                assert!(matches!(e.bits_per_led_bit, 4 | 8), "{hz} Hz");
                assert!(enc::<C>(hz).is_ok(), "{hz} Hz");
                let data = [0x00u8, 0xFF, 0xA5];
                let mut out = [0u8; 600];
                let used = e.encode(data, &mut out).unwrap();
                let data_bytes = data.len() * e.bits_per_led_bit as usize;
                assert!(used > 1 + data_bytes);
                for (i, byte) in out.iter().enumerate().take(1 + data_bytes).skip(1) {
                    assert_eq!(byte & 1, 0, "{hz} Hz byte {i}");
                }
            }
        }
    }

    #[test]
    fn byte_aligned_bytes_always_end_low() {
        check_bytes_end_low::<Ws2812>();
        check_bytes_end_low::<Ws2812b>();
        check_bytes_end_low::<Ws2813>();
        check_bytes_end_low::<Ws2815>();
        check_bytes_end_low::<Ws2811>();
        check_bytes_end_low::<Ws2811Slow>();
        check_bytes_end_low::<Sk6812>();
        check_bytes_end_low::<Sk6812Rgbw>();
        check_bytes_end_low::<Apa106>();
    }

    #[test]
    fn streaming_matches_encode() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        for data in [&[][..], &[0xFF][..], &[1, 2, 3][..], &[0xA5; 7][..]] {
            let (mut a, mut b) = ([0u8; 256], [0xAAu8; 256]);
            let n = e.encode(data.iter().copied(), &mut a).unwrap();
            // `filter` hides the length, as a smart-leds iterator would.
            assert_eq!(
                e.encode_from_iter(data.iter().copied().filter(|_| true), &mut b),
                Ok(n)
            );
            assert_eq!(a[..n], b[..n]);
        }
    }

    #[test]
    fn streaming_reports_the_full_size_when_out_is_short() {
        let e = enc::<Ws2812b>(3_200_000).unwrap();
        let mut out = [0u8; 20];
        assert_eq!(
            e.encode_from_iter([7u8; 9], &mut out),
            Err(BufferTooSmall {
                needed: e.buffer_len(9)
            })
        );
    }
}
