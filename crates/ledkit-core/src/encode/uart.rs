//! Serial port (UART) bit patterns that imitate a clockless LED signal.

use super::{miss_ps, window_ok};
use crate::{BufferTooSmall, ClocklessTiming, TimingError};

/// How to fake a clockless LED signal with a serial port (UART).
///
/// A serial byte on the wire is 10 equal slots: a start bit, 8 data bits
/// (least significant first) and a stop bit. With the port's transmit line
/// **inverted**, the line rests low, the start bit is high and the stop bit
/// is low, which suits a NeoPixel. Each byte then carries two LED bits of 5
/// slots each: the first starts with the start bit, the second ends with the
/// stop bit. An LED bit is a few high slots followed by low ones, and the
/// number of high slots is what tells a `0` from a `1`.
///
/// Get one from [`ClocklessTiming::uart_encoding`], which picks the pattern
/// that fits the baud your port actually runs at.
///
/// # Example
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// let enc = ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000).unwrap();
/// // A 0 is 1 slot high (250 ns), a 1 is 3 slots high (750 ns).
/// assert_eq!((enc.zero_high_slots, enc.one_high_slots), (1, 3));
///
/// let mut out = [0u8; 4];
/// assert_eq!(enc.encode([0b1010_0000u8], &mut out), Ok(4));
/// assert_eq!(out, [0xEC, 0xEC, 0xEF, 0xEF]);
/// ```
///
/// # Sizing a buffer at compile time
///
/// [`ClocklessTiming::uart_encoding`] and [`buffer_len`](Self::buffer_len)
/// are `const fn`s, so a fixed strip's buffer can be an ordinary array whose
/// size is worked out when your program is compiled. A baud that can't work
/// stops the build instead of failing on the device.
///
/// ```
/// use ledkit_core::{ClocklessTiming, Ws2812b};
///
/// const LED_COUNT: usize = 30;
/// const UART_BUFFER_LEN: usize =
///     match ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000) {
///         Ok(encoding) => encoding.buffer_len(LED_COUNT * 3), // 3 bytes per RGB LED
///         Err(_) => panic!("this baud can't drive a WS2812B"),
///     };
///
/// let buffer = [0u8; UART_BUFFER_LEN];
/// assert_eq!(buffer.len(), 360);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartEncoding {
    /// How many of an LED bit's 5 slots are high for a `0`.
    pub zero_high_slots: u8,
    /// How many of an LED bit's 5 slots are high for a `1`.
    pub one_high_slots: u8,
}

/// The bauds suggested when one doesn't work: 1.0 to 6.0 Mbaud in 100 kHz steps.
const GRID_STEP: u32 = 100_000;
const GRID_FIRST: u32 = 10;
const GRID_LAST: u32 = 60;

impl ClocklessTiming {
    /// Finds the best serial port pattern for a port running at `baud`.
    ///
    /// Each LED bit is 5 slots of the serial byte, some high then the rest
    /// low (see [`UartEncoding`]). All four pulses must fit the chip's
    /// allowed window exactly. Of the patterns that do, the one closest to
    /// the chip's typical timing wins.
    ///
    /// Pass the baud the port **really** runs at: many ports can only get
    /// close to the one you ask for. This is a `const fn`, so it also works
    /// at compile time.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::UartBaudUnusable`] if no pattern fits, along
    /// with the nearest bauds (in steps of 100 kHz, from 1.0 to 6.0 Mbaud)
    /// that do work.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ClocklessTiming, TimingError, Ws2812b};
    ///
    /// let timing = ClocklessTiming::of::<Ws2812b>();
    /// assert!(timing.uart_encoding(4_000_000).is_ok());
    /// // 1 Mbaud is far too slow for a WS2812B.
    /// assert!(matches!(
    ///     timing.uart_encoding(1_000_000),
    ///     Err(TimingError::UartBaudUnusable { .. })
    /// ));
    /// ```
    pub const fn uart_encoding(&self, baud: u32) -> Result<UartEncoding, TimingError> {
        if let Some(e) = self.uart_search(baud) {
            return Ok(e);
        }
        Err(TimingError::UartBaudUnusable {
            baud,
            nearest_lower_baud: self.nearest_working_baud(baud, false),
            nearest_higher_baud: self.nearest_working_baud(baud, true),
        })
    }

    /// The closest baud on the 100 kHz grid, below (or above) `baud`, that works.
    const fn nearest_working_baud(&self, baud: u32, above: bool) -> Option<u32> {
        let mut step = 0;
        while step <= GRID_LAST - GRID_FIRST {
            let i = if above {
                GRID_FIRST + step
            } else {
                GRID_LAST - step
            };
            let b = i * GRID_STEP;
            let on_the_right_side = if above { b > baud } else { b < baud };
            if on_the_right_side && self.uart_search(b).is_some() {
                return Some(b);
            }
            step += 1;
        }
        None
    }

    /// The best pattern for `baud`, if any fits.
    const fn uart_search(&self, baud: u32) -> Option<UartEncoding> {
        if baud == 0 {
            return None;
        }
        let baud_hz = baud as u64;
        let slots = UartEncoding::SLOTS_PER_LED_BIT as u64;
        // (score, k0, k1) of the best pattern so far; ties keep the smaller k0, then k1.
        let mut best: Option<(u64, u8, u8)> = None;
        let mut k0: u8 = 1;
        while k0 < UartEncoding::SLOTS_PER_LED_BIT {
            let mut k1: u8 = k0 + 1;
            while k1 < UartEncoding::SLOTS_PER_LED_BIT {
                let (zero_high, one_high) = (k0 as u64, k1 as u64);
                let phases = [
                    (self.zero_high, zero_high),
                    (self.zero_low, slots - zero_high),
                    (self.one_high, one_high),
                    (self.one_low, slots - one_high),
                ];
                let mut fits = true;
                let mut score: u64 = 0;
                let mut i = 0;
                while i < phases.len() {
                    let (phase, k) = phases[i];
                    if !window_ok(phase, k, baud_hz) {
                        fits = false;
                    }
                    score += miss_ps(phase, k, baud_hz);
                    i += 1;
                }
                let better = match best {
                    None => true,
                    Some((best_score, _, _)) => score < best_score,
                };
                if fits && better {
                    best = Some((score, k0, k1));
                }
                k1 += 1;
            }
            k0 += 1;
        }
        match best {
            Some((_, zero_high_slots, one_high_slots)) => Some(UartEncoding {
                zero_high_slots,
                one_high_slots,
            }),
            None => None,
        }
    }
}

impl UartEncoding {
    /// Each LED bit is 5 slots of a 10-slot serial byte.
    pub const SLOTS_PER_LED_BIT: u8 = 5;

    /// The data byte to hand the (inverted) port for two LED bits, `first`
    /// then `second`.
    ///
    /// On the wire, slot 0 is the start bit, slots 1 to 8 are data bits 0
    /// to 7, and slot 9 is the stop bit. Because the port inverts its output,
    /// data bit `i` is the opposite of the level wanted in slot `i + 1`.
    #[must_use]
    pub const fn byte_for(&self, first: bool, second: bool) -> u8 {
        let n = Self::SLOTS_PER_LED_BIT;
        let a = if first {
            self.one_high_slots
        } else {
            self.zero_high_slots
        };
        let b = if second {
            self.one_high_slots
        } else {
            self.zero_high_slots
        };
        let mut byte = 0u8;
        let mut i = 0u8;
        while i < 8 {
            let slot = i + 1;
            let high = if slot < n { slot < a } else { slot - n < b };
            if !high {
                byte |= 1 << i;
            }
            i += 1;
        }
        byte
    }

    /// How many serial bytes [`encode`](Self::encode) needs for `wire_bytes`
    /// bytes of LED data: 4 per byte (8 LED bits, two per serial byte).
    ///
    /// There are no reset bytes: a serial byte always has a high start bit,
    /// so the latch is a pause after the frame instead. It is a `const fn`,
    /// so it can size an array at compile time.
    #[must_use]
    pub const fn buffer_len(&self, wire_bytes: usize) -> usize {
        wire_bytes * 4
    }

    /// Encodes LED wire bytes into `out`, returning how many bytes were used.
    ///
    /// # Errors
    ///
    /// Returns [`BufferTooSmall`] if `out` is shorter than
    /// [`buffer_len`](Self::buffer_len); nothing useful is written then.
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
        self.encode_from_iter(wire, out)
    }

    /// Like [`encode`](Self::encode), for an iterator whose length isn't
    /// known up front.
    ///
    /// # Errors
    ///
    /// Returns [`BufferTooSmall`] (with the size that would have been
    /// needed) if the bytes don't fit in `out`.
    pub fn encode_from_iter(
        &self,
        wire: impl IntoIterator<Item = u8>,
        out: &mut [u8],
    ) -> Result<usize, BufferTooSmall> {
        let mut wire_bytes = 0;
        for byte in wire {
            for (k, pair) in (0..4).rev().enumerate() {
                let first = (byte >> (2 * pair + 1)) & 1 == 1;
                let second = (byte >> (2 * pair)) & 1 == 1;
                if let Some(slot) = out.get_mut(wire_bytes * 4 + k) {
                    *slot = self.byte_for(first, second);
                }
            }
            wire_bytes += 1;
        }
        let needed = self.buffer_len(wire_bytes);
        if out.len() < needed {
            return Err(BufferTooSmall { needed });
        }
        Ok(needed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Apa106, BufferTooSmall, Chip, ClocklessTiming, Sk6812, Sk6812Rgbw, TimingError, Ws2811,
        Ws2811Slow, Ws2812, Ws2812b, Ws2813, Ws2815,
    };

    /// The bauds the error suggestions come from: 1.0 to 6.0 Mbaud in 100 kHz steps.
    fn grid() -> impl Iterator<Item = u32> {
        (10..=60).map(|i| i * 100_000)
    }

    fn ws2812b() -> UartEncoding {
        ClocklessTiming::of::<Ws2812b>()
            .uart_encoding(4_000_000)
            .unwrap()
    }

    #[test]
    fn ws2812b_at_4_mbaud() {
        let e = ws2812b();
        assert_eq!((e.zero_high_slots, e.one_high_slots), (1, 3));
    }

    #[test]
    fn byte_table_for_ws2812b() {
        let e = ws2812b();
        assert_eq!(e.byte_for(false, false), 0xEF);
        assert_eq!(e.byte_for(false, true), 0x8F);
        assert_eq!(e.byte_for(true, false), 0xEC); // the spec's worked example
        assert_eq!(e.byte_for(true, true), 0x8C);
    }

    #[test]
    fn encodes_msb_first_two_bits_per_byte() {
        let mut out = [0u8; 4];
        assert_eq!(ws2812b().encode([0b1010_0000u8], &mut out), Ok(4));
        assert_eq!(out, [0xEC, 0xEC, 0xEF, 0xEF]);
    }

    #[test]
    fn rgbw_is_16_bytes_per_led() {
        let e = ClocklessTiming::of::<Sk6812Rgbw>()
            .uart_encoding(4_000_000)
            .unwrap();
        assert_eq!(e.buffer_len(4), 16);
    }

    #[test]
    fn encode_reports_a_small_buffer() {
        let e = ws2812b();
        let mut out = [0u8; 7];
        assert_eq!(
            e.encode([1u8, 2], &mut out),
            Err(BufferTooSmall { needed: 8 })
        );
        assert_eq!(
            e.encode_from_iter([1u8, 2], &mut out),
            Err(BufferTooSmall { needed: 8 })
        );
    }

    #[test]
    fn encode_and_encode_from_iter_agree() {
        let e = ws2812b();
        let wire = [0x00u8, 0xFF, 0x5A];
        let (mut a, mut b) = ([0u8; 12], [0u8; 12]);
        assert_eq!(e.encode(wire, &mut a), Ok(12));
        assert_eq!(e.encode_from_iter(wire, &mut b), Ok(12));
        assert_eq!(a, b);
    }

    /// Whether `slots` slots at `baud` last within the phase's window, exactly.
    fn fits(phase: crate::Phase, slots: u8, baud: u32) -> bool {
        let t = u64::from(slots) * 1_000_000_000;
        let hz = u64::from(baud);
        u64::from(phase.min_ns) * hz <= t && t <= u64::from(phase.max_ns) * hz
    }

    fn check_windows<C: Chip>() {
        let t = ClocklessTiming::of::<C>();
        for baud in grid() {
            if let Ok(e) = t.uart_encoding(baud) {
                let (k0, k1) = (e.zero_high_slots, e.one_high_slots);
                assert!(fits(t.zero_high, k0, baud), "{} {baud}", C::NAME);
                assert!(fits(t.zero_low, 5 - k0, baud), "{} {baud}", C::NAME);
                assert!(fits(t.one_high, k1, baud), "{} {baud}", C::NAME);
                assert!(fits(t.one_low, 5 - k1, baud), "{} {baud}", C::NAME);
            }
        }
    }

    #[test]
    fn every_pattern_fits_the_chip_windows() {
        check_windows::<Ws2812>();
        check_windows::<Ws2812b>();
        check_windows::<Ws2813>();
        check_windows::<Ws2815>();
        check_windows::<Ws2811>();
        check_windows::<Ws2811Slow>();
        check_windows::<Sk6812>();
        check_windows::<Sk6812Rgbw>();
        check_windows::<Apa106>();
    }

    fn has_a_working_baud<C: Chip>() -> bool {
        let t = ClocklessTiming::of::<C>();
        grid().any(|baud| t.uart_encoding(baud).is_ok())
    }

    #[test]
    fn every_clockless_chip_has_a_working_baud() {
        assert!(has_a_working_baud::<Ws2812>());
        assert!(has_a_working_baud::<Ws2812b>());
        assert!(has_a_working_baud::<Ws2813>());
        assert!(has_a_working_baud::<Ws2815>());
        assert!(has_a_working_baud::<Ws2811>());
        assert!(has_a_working_baud::<Ws2811Slow>());
        assert!(has_a_working_baud::<Sk6812>());
        assert!(has_a_working_baud::<Sk6812Rgbw>());
        assert!(has_a_working_baud::<Apa106>());
    }

    #[test]
    fn error_names_working_neighbours() {
        let t = ClocklessTiming::of::<Ws2812b>();
        match t.uart_encoding(1_000_000) {
            Err(TimingError::UartBaudUnusable {
                baud: 1_000_000,
                nearest_higher_baud: Some(hi),
                ..
            }) => {
                assert!(t.uart_encoding(hi).is_ok());
                assert!(t.uart_encoding(hi - 100_000).is_err());
            }
            other => panic!("{other:?}"),
        }
        assert!(t.uart_encoding(0).is_err());
    }

    #[test]
    fn works_at_compile_time() {
        const LEN: usize = match ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000) {
            Ok(e) => e.buffer_len(30 * 3),
            Err(_) => panic!(),
        };
        assert_eq!(LEN, 360);
    }
}
