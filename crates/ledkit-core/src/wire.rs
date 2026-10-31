//! Turning pixels into the bytes an LED chip expects on the wire.
//!
//! "Wire order" is the order in which a chip wants its color values. A
//! WS2812B LED, for example, expects green first, then red, then blue, so the
//! pixel `Rgb::new(1, 2, 3)` goes out as `2, 1, 3`. Both functions here hand
//! back lazy iterators: nothing is allocated, and bytes are made on demand.
//! Both iterators know exactly how many bytes are left, so they can be passed
//! straight to [`SpiEncoding::encode`](crate::encode::SpiEncoding::encode).

use core::fmt;
use core::iter::FusedIterator;
use core::marker::PhantomData;

use crate::Pixel;
use crate::{Chip, ClockedFormat};

/// Yields the bytes for a clockless chip (WS2812B, SK6812 and friends).
///
/// Each pixel becomes its channels in the chip's wire order, one byte per
/// channel. Use this for clockless chips; [`clocked_frame`] is for clocked
/// ones. Using the wrong one does not panic, it just gives bytes the chip
/// will not understand.
///
/// The returned [`ClocklessBytes`] knows its exact length, so it can go
/// straight into [`SpiEncoding::encode`](crate::encode::SpiEncoding::encode).
///
/// # Example
///
/// ```
/// use ledkit_core::{wire::clockless_bytes, Rgb, Ws2812b};
///
/// let bytes: Vec<u8> = clockless_bytes::<Ws2812b, _>([Rgb::new(1, 2, 3)]).collect();
/// assert_eq!(bytes, [2, 1, 3]); // green, red, blue
/// ```
pub fn clockless_bytes<C, I>(pixels: I) -> ClocklessBytes<C, I::IntoIter>
where
    C: Chip,
    I: IntoIterator<Item = C::Pixel>,
    I::IntoIter: ExactSizeIterator,
{
    ClocklessBytes {
        pixels: pixels.into_iter(),
        current: None,
        next_channel: 0,
        chip: PhantomData,
    }
}

/// The bytes of a clockless chip, in wire order. Made by [`clockless_bytes`].
///
/// It is an [`ExactSizeIterator`]: `len()` is the number of bytes still to
/// come (whole pixels left times the channels per pixel, plus whatever is
/// left of the pixel in progress).
///
/// # Example
///
/// ```
/// use ledkit_core::{wire::clockless_bytes, Rgb, Ws2812b};
///
/// let mut bytes = clockless_bytes::<Ws2812b, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]);
/// assert_eq!(bytes.len(), 6);
/// assert_eq!(bytes.next(), Some(2));
/// assert_eq!(bytes.len(), 5);
/// ```
pub struct ClocklessBytes<C: Chip, I> {
    pixels: I,
    current: Option<C::Pixel>,
    next_channel: usize,
    chip: PhantomData<C>,
}

impl<C: Chip, I> ClocklessBytes<C, I> {
    /// Bytes left in the pixel currently being sent.
    fn left_in_current(&self) -> usize {
        match self.current {
            Some(_) => C::ORDER.len() - self.next_channel,
            None => 0,
        }
    }
}

impl<C: Chip, I: Clone> Clone for ClocklessBytes<C, I> {
    fn clone(&self) -> Self {
        Self {
            pixels: self.pixels.clone(),
            current: self.current,
            next_channel: self.next_channel,
            chip: PhantomData,
        }
    }
}

impl<C: Chip, I: fmt::Debug> fmt::Debug for ClocklessBytes<C, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClocklessBytes")
            .field("pixels", &self.pixels)
            .field("current", &self.current)
            .field("next_channel", &self.next_channel)
            .finish()
    }
}

impl<C: Chip, I: Iterator<Item = C::Pixel>> Iterator for ClocklessBytes<C, I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        loop {
            if let Some(pixel) = self.current {
                if let Some(&channel) = C::ORDER.channels().get(self.next_channel) {
                    self.next_channel += 1;
                    return Some(pixel.channel(channel));
                }
                self.current = None;
            }
            self.current = Some(self.pixels.next()?);
            self.next_channel = 0;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let per_pixel = C::ORDER.len();
        let left = self.left_in_current();
        let (low, high) = self.pixels.size_hint();
        let low = low.saturating_mul(per_pixel).saturating_add(left);
        let high = high
            .and_then(|h| h.checked_mul(per_pixel))
            .and_then(|h| h.checked_add(left));
        (low, high)
    }
}

impl<C: Chip, I: ExactSizeIterator<Item = C::Pixel>> ExactSizeIterator for ClocklessBytes<C, I> {}

impl<C: Chip, I: FusedIterator<Item = C::Pixel>> FusedIterator for ClocklessBytes<C, I> {}

/// How many bytes [`clocked_frame`] produces for `led_count` LEDs of chip `C`.
///
/// That is a 4-byte start frame, 4 bytes per LED and the chip's end frame:
///
/// - APA102: one byte for every 16 LEDs (rounded up).
/// - SK9822: 4 reset bytes, then one byte for every 16 LEDs (rounded up).
///
/// For zero LEDs the APA102 frame is just the 4-byte start frame.
///
/// The "4 bytes per LED" assumes 3-channel DotStar pixels (a brightness byte
/// plus blue, green and red), which is what every clocked chip in the
/// catalogue uses.
///
/// # Example
///
/// ```
/// use ledkit_core::wire::clocked_frame_len;
/// use ledkit_core::{Apa102, Sk9822};
///
/// assert_eq!(clocked_frame_len::<Apa102>(2), 13);
/// assert_eq!(clocked_frame_len::<Sk9822>(2), 17);
/// ```
pub const fn clocked_frame_len<C: Chip>(led_count: usize) -> usize {
    4 + 4 * led_count + ClockedFormat::of::<C>().end_frame_len(led_count)
}

/// Yields a complete DotStar (APA102/SK9822) frame for a clocked chip.
///
/// The frame is a start frame of four `0x00` bytes, then for each LED a
/// `0xFF` byte (full global brightness) followed by its channels in the
/// chip's wire order, then the chip's end frame (`n` is the LED count). Use
/// this for clocked chips; [`clockless_bytes`] is for clockless ones. Using the
/// wrong one does not panic. The iterator must know its length up front,
/// because the end frame depends on the LED count.
///
/// # The end frame, in plain words
///
/// The LEDs sit in a chain and each one passes the data on half a clock late,
/// so the last LEDs only show their colors if the strip gets about `n / 2` more
/// clock edges after the data. `ceil(n / 16)` bytes is `n / 2` bits or more.
/// What those extra bytes contain depends on the chip.
///
/// - **APA102** ([`ClockedFormat::Apa102`]): `ceil(n / 16)` bytes of `0xFF`.
///   They are ones rather than zeros because the APA102 reads a run of 32 or
///   more zero bits as a new start frame, which made the last LEDs (60 to 63
///   of 64 on an APA102-2020 DotStar HD 8x8) miss their colors. A full 32-bit
///   frame of ones is avoided too, since an LED past the end of the strip would
///   see it as pixel data. This is the end frame Adafruit's DotStar library
///   sends, and it is verified on hardware.
/// - **SK9822** ([`ClockedFormat::Sk9822`]): a reset frame of 4 bytes of
///   `0x00`, then `ceil(n / 16)` bytes of `0x00` for the propagation clocks.
///   The SK9822 only latches its colors after it sees 32 zero bits, so an
///   all-ones end frame would leave the update pending until the next start
///   frame (see
///   [cpldcpu's write-up](https://cpldcpu.com/2016/12/13/sk9822-a-clone-of-the-apa102/)).
///   This is not yet verified on an SK9822 strip in ledkit.
///
/// # Example
///
/// ```
/// use ledkit_core::{wire::clocked_frame, Apa102, Rgb};
///
/// let frame: Vec<u8> = clocked_frame::<Apa102, _>([Rgb::new(1, 2, 3)]).collect();
/// assert_eq!(frame, [0, 0, 0, 0, 0xFF, 3, 2, 1, 0xFF]);
/// ```
pub fn clocked_frame<C, I>(pixels: I) -> ClockedFrame<C, I::IntoIter>
where
    C: Chip,
    I: IntoIterator<Item = C::Pixel>,
    I::IntoIter: ExactSizeIterator,
{
    let pixels = pixels.into_iter();
    let end_left = ClockedFormat::of::<C>().end_frame_len(pixels.len());
    ClockedFrame {
        pixels,
        start_left: 4,
        current: None,
        next_byte: 0,
        body_done: false,
        end_left,
        chip: PhantomData,
    }
}

/// A complete DotStar frame, byte by byte. Made by [`clocked_frame`].
///
/// It is an [`ExactSizeIterator`]: `len()` is the number of bytes still to
/// come (start frame, LED data and end frame).
///
/// # Example
///
/// ```
/// use ledkit_core::wire::{clocked_frame, clocked_frame_len};
/// use ledkit_core::{Apa102, Rgb};
///
/// let frame = clocked_frame::<Apa102, _>([Rgb::RED; 3]);
/// assert_eq!(frame.len(), clocked_frame_len::<Apa102>(3));
/// ```
pub struct ClockedFrame<C: Chip, I> {
    pixels: I,
    start_left: usize,
    current: Option<C::Pixel>,
    /// 0 is the `0xFF` header byte, then one per channel.
    next_byte: usize,
    body_done: bool,
    end_left: usize,
    chip: PhantomData<C>,
}

impl<C: Chip, I> ClockedFrame<C, I> {
    /// Bytes left in the LED currently being sent (header plus channels).
    fn left_in_current(&self) -> usize {
        match self.current {
            Some(_) => 1 + C::ORDER.len() - self.next_byte,
            None => 0,
        }
    }
}

impl<C: Chip, I: Clone> Clone for ClockedFrame<C, I> {
    fn clone(&self) -> Self {
        Self {
            pixels: self.pixels.clone(),
            start_left: self.start_left,
            current: self.current,
            next_byte: self.next_byte,
            body_done: self.body_done,
            end_left: self.end_left,
            chip: PhantomData,
        }
    }
}

impl<C: Chip, I: fmt::Debug> fmt::Debug for ClockedFrame<C, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClockedFrame")
            .field("pixels", &self.pixels)
            .field("start_left", &self.start_left)
            .field("current", &self.current)
            .field("next_byte", &self.next_byte)
            .field("body_done", &self.body_done)
            .field("end_left", &self.end_left)
            .finish()
    }
}

impl<C: Chip, I: Iterator<Item = C::Pixel>> Iterator for ClockedFrame<C, I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if self.start_left > 0 {
            self.start_left -= 1;
            return Some(0x00);
        }
        while !self.body_done {
            if let Some(pixel) = self.current {
                if self.next_byte == 0 {
                    self.next_byte = 1;
                    return Some(0xFF);
                }
                if let Some(&channel) = C::ORDER.channels().get(self.next_byte - 1) {
                    self.next_byte += 1;
                    return Some(pixel.channel(channel));
                }
                self.current = None;
            }
            match self.pixels.next() {
                Some(pixel) => {
                    self.current = Some(pixel);
                    self.next_byte = 0;
                }
                None => self.body_done = true,
            }
        }
        if self.end_left > 0 {
            self.end_left -= 1;
            return Some(ClockedFormat::of::<C>().end_frame_byte());
        }
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let fixed = self.start_left + self.left_in_current() + self.end_left;
        if self.body_done {
            return (fixed, Some(fixed));
        }
        let per_led = 1 + C::ORDER.len();
        let (low, high) = self.pixels.size_hint();
        let low = low.saturating_mul(per_led).saturating_add(fixed);
        let high = high
            .and_then(|h| h.checked_mul(per_led))
            .and_then(|h| h.checked_add(fixed));
        (low, high)
    }
}

impl<C: Chip, I: ExactSizeIterator<Item = C::Pixel>> ExactSizeIterator for ClockedFrame<C, I> {}

// Once the pixels run out, `body_done` stops them being polled again.
impl<C: Chip, I: Iterator<Item = C::Pixel>> FusedIterator for ClockedFrame<C, I> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Apa102, ClocklessTiming, Rgb, Rgbw, Sk6812Rgbw, Sk9822, Ws2812b};

    #[test]
    fn grb_wire_order() {
        let bytes = clockless_bytes::<Ws2812b, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)])
            .collect::<Vec<_>>();
        assert_eq!(bytes, [2, 1, 3, 5, 4, 6]);
    }

    #[test]
    fn rgbw_wire_order() {
        let mut it = clockless_bytes::<Sk6812Rgbw, _>([Rgbw::new(1, 2, 3, 4)]);
        assert_eq!(
            [it.next(), it.next(), it.next(), it.next(), it.next()],
            [Some(2), Some(1), Some(3), Some(4), None]
        );
    }

    #[test]
    fn clockless_len_counts_down_exactly() {
        let mut it = clockless_bytes::<Sk6812Rgbw, _>([Rgbw::new(1, 2, 3, 4); 3]);
        for left in (0..=12).rev() {
            assert_eq!(it.len(), left);
            assert_eq!(it.size_hint(), (left, Some(left)));
            assert_eq!(it.next().is_some(), left > 0);
        }
        assert_eq!(it.next(), None);
        assert_eq!(it.len(), 0);
    }

    #[test]
    fn clockless_bytes_feed_spi_encode() {
        let enc = ClocklessTiming::of::<Ws2812b>()
            .spi_encoding(3_200_000)
            .unwrap();
        let mut out = [0xAAu8; 200];
        assert_eq!(
            enc.encode(
                clockless_bytes::<Ws2812b, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]),
                &mut out
            ),
            Ok(enc.buffer_len(6))
        );
        let mut direct = [0x55u8; 200];
        assert_eq!(
            enc.encode([2u8, 1, 3, 5, 4, 6], &mut direct),
            Ok(enc.buffer_len(6))
        );
        let used = enc.buffer_len(6);
        assert_eq!(out[..used], direct[..used]);
    }

    #[test]
    fn dotstar_frame() {
        let frame =
            clocked_frame::<Apa102, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]).collect::<Vec<_>>();
        assert_eq!(frame, [0, 0, 0, 0, 0xFF, 3, 2, 1, 0xFF, 6, 5, 4, 0xFF]);
        assert_eq!(clocked_frame_len::<Apa102>(2), 13);
    }

    #[test]
    fn sk9822_frame_has_a_zero_reset_frame() {
        let frame =
            clocked_frame::<Sk9822, _>([Rgb::new(1, 2, 3), Rgb::new(4, 5, 6)]).collect::<Vec<_>>();
        assert_eq!(
            frame,
            [0, 0, 0, 0, 0xFF, 3, 2, 1, 0xFF, 6, 5, 4, 0, 0, 0, 0, 0]
        );
        assert_eq!(clocked_frame_len::<Sk9822>(2), 17);
    }

    /// The bytes after the LED data, for a chip and LED count.
    fn end_frame<C: Chip<Pixel = Rgb>>(n: usize) -> Vec<u8> {
        let frame: Vec<u8> = clocked_frame::<C, _>(vec![Rgb::new(9, 8, 7); n]).collect();
        frame[4 + 4 * n..].to_vec()
    }

    #[test]
    fn apa102_end_frame_is_ceil_n_over_16_bytes_of_ff() {
        for (n, bytes) in [(0, 0), (1, 1), (15, 1), (16, 1), (17, 2), (64, 4)] {
            assert_eq!(end_frame::<Apa102>(n), vec![0xFF; bytes], "n = {n}");
            assert_eq!(clocked_frame_len::<Apa102>(n), 4 + 4 * n + bytes, "n = {n}");
        }
    }

    #[test]
    fn sk9822_end_frame_is_a_zero_reset_frame_then_clocks() {
        for (n, bytes) in [(0, 4), (1, 5), (15, 5), (16, 5), (17, 6), (64, 8)] {
            assert_eq!(end_frame::<Sk9822>(n), vec![0x00; bytes], "n = {n}");
            assert_eq!(clocked_frame_len::<Sk9822>(n), 4 + 4 * n + bytes, "n = {n}");
        }
    }

    #[test]
    fn dotstar_frame_lengths() {
        assert_eq!(clocked_frame_len::<Apa102>(0), 4);
        assert_eq!(clocked_frame_len::<Apa102>(16), 4 + 64 + 1);
        assert_eq!(clocked_frame_len::<Apa102>(17), 4 + 68 + 2);
        assert_eq!(clocked_frame_len::<Sk9822>(0), 8);
        assert_eq!(clocked_frame_len::<Sk9822>(16), 4 + 64 + 5);
        assert_eq!(clocked_frame_len::<Sk9822>(17), 4 + 68 + 6);
    }

    #[test]
    fn dotstar_frame_iterator_matches_frame_len() {
        let pixels = [Rgb::new(9, 8, 7); 17];
        for n in [0, 1, 16, 17] {
            let frame = clocked_frame::<Apa102, _>(pixels[..n].iter().copied());
            assert_eq!(frame.len(), clocked_frame_len::<Apa102>(n), "len, n = {n}");
            assert_eq!(
                frame.count(),
                clocked_frame_len::<Apa102>(n),
                "count, n = {n}"
            );
            let frame = clocked_frame::<Sk9822, _>(pixels[..n].iter().copied());
            assert_eq!(frame.len(), clocked_frame_len::<Sk9822>(n), "len, n = {n}");
            assert_eq!(
                frame.count(),
                clocked_frame_len::<Sk9822>(n),
                "count, n = {n}"
            );
        }
    }

    #[test]
    fn dotstar_len_counts_down_exactly() {
        let mut frame = clocked_frame::<Apa102, _>([Rgb::new(1, 2, 3); 17]);
        for left in (0..=clocked_frame_len::<Apa102>(17)).rev() {
            assert_eq!(frame.len(), left);
            assert_eq!(frame.next().is_some(), left > 0);
        }
        assert_eq!(frame.next(), None);
        assert_eq!(frame.len(), 0);

        let mut frame = clocked_frame::<Sk9822, _>([Rgb::new(1, 2, 3); 17]);
        for left in (0..=clocked_frame_len::<Sk9822>(17)).rev() {
            assert_eq!(frame.len(), left);
            assert_eq!(frame.next().is_some(), left > 0);
        }
        assert_eq!(frame.next(), None);
    }
}
