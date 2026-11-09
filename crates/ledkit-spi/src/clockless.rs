//! NeoPixels over SPI.

use crate::Error;
use core::marker::PhantomData;
use ledkit_core::encode::SpiEncoding;
use ledkit_core::{wire, Chip, ClocklessTiming, Output, OutputAsync, TimingError};

/// Sends NeoPixel-style (one data wire) LEDs over an SPI bus.
///
/// NeoPixels have no clock wire, and their bits are short and long pulses.
/// This driver imitates those pulses by sending carefully chosen SPI bytes
/// on the bus's data pin (MOSI). Connect that pin to the LED data input.
///
/// - `C` is your LED type, such as [`Ws2812b`](ledkit_core::Ws2812b).
/// - `SPI` is your SPI bus. It can be any `embedded-hal` 1.0 `SpiBus`
///   (blocking) or `embedded-hal-async` `SpiBus` (async).
/// - `B` is a byte buffer the frame is built in, for example `[u8; N]`.
///   Use [`buffer_len`] to find the size you need.
///
/// **ledkit never sets up the bus.** You create and configure the SPI
/// peripheral, its clock speed and the data pin with your board's HAL, then
/// pass the bus here together with the clock rate it really runs at, in
/// hertz. Not every speed works for every LED: [`new`](Self::new) tells you
/// if yours doesn't, and the error names clocks that do.
///
/// You usually don't call `write` yourself. Put the driver in a
/// [`Strip`](ledkit_core::Strip) and call `strip.show()`.
///
/// # The reset gap
///
/// NeoPixels need a short pause in the data to latch the colors. This
/// driver adds it as zero bytes at the end of every frame and waits for the
/// bus to finish (`flush`) before returning, so frames are never run
/// together, however often you call `show()`.
///
/// The very first frame also starts with a reset gap. Some SPI peripherals
/// hold their data pin high until their first transfer (the ESP32-S3's
/// does). LEDs read that high as one extra bit, which shifts the whole first
/// frame by a bit and turns the first LED's color wrong (a red LED shows
/// green). Starting low for a reset time ends that stray bit before the
/// colors come.
///
/// # DotStars
///
/// DotStars (APA102 and SK9822) have their own clock wire and don't need
/// this trick. Using one here is a build error.
///
/// ```compile_fail
/// # use ledkit_core::Apa102;
/// # use ledkit_spi::ClocklessSpi;
/// # struct Bus;
/// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
/// # impl embedded_hal::spi::SpiBus for Bus {
/// #     fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn write(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// // DotStars need a clocked backend instead.
/// let _ = ClocklessSpi::<Apa102, _, _>::new(Bus, 3_200_000, [0u8; 64]);
/// ```
///
/// # Example
///
/// ```
/// use ledkit_core::{Rgb, Strip, Ws2812b};
/// use ledkit_spi::{buffer_len, ClocklessSpi};
/// # struct Bus;
/// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
/// # impl embedded_hal::spi::SpiBus for Bus {
/// #     fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn write(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # let spi = Bus;
///
/// const LED_COUNT: usize = 8;
/// const SPI_HZ: u32 = 3_200_000; // the clock you configured your SPI bus with
///
/// let output = ClocklessSpi::<Ws2812b, _, _>::new(
///     spi,
///     SPI_HZ,
///     [0u8; buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)],
/// )
/// .unwrap();
/// let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
/// strip.fill(Rgb::BLUE);
/// strip.show().unwrap();
/// ```
#[derive(Debug)]
pub struct ClocklessSpi<C: Chip, SPI, B> {
    spi: SPI,
    buffer: B,
    encoding: SpiEncoding,
    /// The most bytes handed to the bus in one `write` call.
    max_write: usize,
    /// Whether a frame has gone out. Until then the data line may rest high,
    /// so the next frame starts with a reset gap.
    started: bool,
    chip: PhantomData<C>,
}

impl<C: Chip, SPI, B: AsMut<[u8]>> ClocklessSpi<C, SPI, B> {
    /// Creates a driver using the byte-aligned bit pattern (the default).
    ///
    /// Each LED bit becomes 4 or 8 whole SPI bits, which keeps the data
    /// simple and works on every SPI peripheral. `spi_hz` is the clock your
    /// bus really runs at. For a WS2812B, 2.3 to 6.8 MHz works.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::SpiClockUnusable`] if no pattern fits this
    /// clock; the error names the nearest clocks that work.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus;
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    /// # let spi = Bus;
    ///
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new(spi, 3_200_000, [0u8; 256]).unwrap();
    /// assert_eq!(output.encoding().bits_per_led_bit, 4);
    /// ```
    pub fn new(spi: SPI, spi_hz: u32, buffer: B) -> Result<Self, TimingError> {
        let timing = const { ClocklessTiming::of::<C>() };
        let encoding = timing.spi_encoding_byte_aligned(spi_hz)?;
        Ok(Self::with_encoding(spi, encoding, buffer))
    }

    /// Creates a driver using the packed bit pattern.
    ///
    /// It packs the bits tighter, so the buffer is smaller: at 4 MHz a
    /// WS2812B needs 5 SPI bits per LED bit instead of the 8 that
    /// [`new`](Self::new) uses there. It does not unlock extra clock speeds
    /// for the WS2812B; `new` accepts the same range. The cost is that the
    /// frame is no longer whole bytes per LED bit. Use it only when your SPI sends bytes back to
    /// back, with no pauses (typically DMA). On a bus with pauses between
    /// bytes, a pause can land inside a high pulse and corrupt a color. If
    /// `new` works for your clock, use it instead.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::SpiClockUnusable`] if no pattern fits this clock.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus;
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    /// # let spi = Bus;
    ///
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new_packed(spi, 4_000_000, [0u8; 256]).unwrap();
    /// assert_eq!(output.encoding().bits_per_led_bit, 5);
    /// ```
    pub fn new_packed(spi: SPI, spi_hz: u32, buffer: B) -> Result<Self, TimingError> {
        let timing = const { ClocklessTiming::of::<C>() };
        let encoding = timing.spi_encoding(spi_hz)?;
        Ok(Self::with_encoding(spi, encoding, buffer))
    }

    fn with_encoding(spi: SPI, encoding: SpiEncoding, buffer: B) -> Self {
        Self {
            spi,
            buffer,
            encoding,
            max_write: usize::MAX,
            started: false,
            chip: PhantomData,
        }
    }

    /// Limits how many bytes are given to the SPI bus in one `write` call.
    ///
    /// Some SPI drivers can only send so much at once. For example, the SPI
    /// driver (`imxrt-hal`'s LPSPI) on the Teensy 4 sends at most 512 bytes
    /// per write. With this limit, ledkit cuts the frame into pieces of at
    /// most `bytes` bytes, writes them one after another, and flushes once
    /// at the end. By default there is no limit and the frame goes out in a
    /// single write.
    ///
    /// This is safe with the byte-aligned pattern that [`new`](Self::new)
    /// uses, as long as each pause between two writes stays well below the
    /// LED's reset time (280 µs for the WS2812B). Then a pause only makes a
    /// low phase a little longer. A blocking HAL on a microcontroller starts
    /// the next write right away, so this holds there. Under an operating
    /// system, or with an async executor that may run other tasks between
    /// writes, a pause can last longer than the reset time, and the LEDs
    /// latch half a frame; avoid chunking in that case. Do not use it with [`new_packed`](Self::new_packed):
    /// there a pause can land in the middle of a high pulse and corrupt the
    /// color. In debug builds, this method panics if the driver was made with
    /// `new_packed`, because chunked writes need the byte-aligned encoding
    /// from `new`.
    ///
    /// # Panics
    ///
    /// Panics if `bytes` is 0. In debug builds it also panics if the driver
    /// uses the packed encoding from `new_packed`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus;
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    /// # let spi = Bus;
    ///
    /// // A bus that can send at most 512 bytes per write.
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new(spi, 3_200_000, [0u8; 1024])
    ///     .unwrap()
    ///     .with_max_write_len(512);
    /// ```
    #[must_use]
    pub fn with_max_write_len(mut self, bytes: usize) -> Self {
        assert!(
            bytes > 0,
            "the maximum write length must be at least 1 byte"
        );
        debug_assert!(
            matches!(self.encoding.bits_per_led_bit, 4 | 8),
            "chunked writes need the byte-aligned encoding from `new`, not `new_packed`"
        );
        self.max_write = bytes;
        self
    }

    /// The SPI bit pattern in use.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus;
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    /// # let spi = Bus;
    ///
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new(spi, 3_200_000, [0u8; 256]).unwrap();
    /// // One LED bit is sent as 4 SPI bits: one or three high, then low.
    /// assert_eq!(output.encoding().bits_per_led_bit, 4);
    /// ```
    #[must_use]
    pub fn encoding(&self) -> SpiEncoding {
        self.encoding
    }

    /// The SPI bus, for example to check its settings.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus(u32);
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    ///
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new(Bus(7), 3_200_000, [0u8; 256]).unwrap();
    /// assert_eq!(output.bus().0, 7);
    /// ```
    #[must_use]
    pub fn bus(&self) -> &SPI {
        &self.spi
    }

    /// Takes the driver apart and gives back the SPI bus and the buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Ws2812b;
    /// use ledkit_spi::ClocklessSpi;
    /// # struct Bus(u32);
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    ///
    /// let output = ClocklessSpi::<Ws2812b, _, _>::new(Bus(7), 3_200_000, [0u8; 256]).unwrap();
    /// let (spi, buffer) = output.release();
    /// assert_eq!((spi.0, buffer.len()), (7, 256));
    /// ```
    #[must_use]
    pub fn release(self) -> (SPI, B) {
        (self.spi, self.buffer)
    }

    /// Fills the buffer with the frame and returns how many bytes to send.
    /// Fails before anything is sent if the buffer is too small.
    fn encode_frame<I>(&mut self, pixels: I) -> Result<usize, ledkit_core::BufferTooSmall>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.encode_wire(wire::clockless_bytes::<C, _>(pixels))
    }

    /// Like `encode_frame`, but from wire bytes of any length.
    pub(crate) fn encode_wire(
        &mut self,
        wire: impl IntoIterator<Item = u8>,
    ) -> Result<usize, ledkit_core::BufferTooSmall> {
        self.encoding.encode_from_iter(wire, self.buffer.as_mut())
    }

    /// How many zero bytes to send before the next frame: a reset gap before
    /// the first frame, none after that.
    fn gap_len(&self) -> usize {
        if self.started {
            0
        } else {
            self.encoding.reset_bytes as usize
        }
    }
}

impl<C: Chip, SPI: embedded_hal::spi::SpiBus, B: AsMut<[u8]>> ClocklessSpi<C, SPI, B> {
    /// Sends the first `used` buffer bytes (in writes of at most
    /// `max_write` bytes), then flushes once.
    pub(crate) fn send(&mut self, used: usize) -> Result<(), Error<SPI::Error>> {
        let gap = self.gap_len();
        let buffer = self.buffer.as_mut();
        // The frame ends in its reset gap, so its tail is the zeros to send.
        for piece in buffer[used - gap..used].chunks(self.max_write) {
            self.spi.write(piece).map_err(Error::Spi)?;
        }
        for piece in buffer[..used].chunks(self.max_write) {
            self.spi.write(piece).map_err(Error::Spi)?;
        }
        self.spi.flush().map_err(Error::Spi)?;
        self.started = true;
        Ok(())
    }
}

impl<C: Chip, SPI: embedded_hal_async::spi::SpiBus, B: AsMut<[u8]>> ClocklessSpi<C, SPI, B> {
    /// The async twin of `send`.
    pub(crate) async fn send_async(&mut self, used: usize) -> Result<(), Error<SPI::Error>> {
        let gap = self.gap_len();
        let buffer = self.buffer.as_mut();
        // The frame ends in its reset gap, so its tail is the zeros to send.
        for piece in buffer[used - gap..used].chunks(self.max_write) {
            self.spi.write(piece).await.map_err(Error::Spi)?;
        }
        for piece in buffer[..used].chunks(self.max_write) {
            self.spi.write(piece).await.map_err(Error::Spi)?;
        }
        self.spi.flush().await.map_err(Error::Spi)?;
        self.started = true;
        Ok(())
    }
}

impl<C, SPI, B> Output<C> for ClocklessSpi<C, SPI, B>
where
    C: Chip,
    SPI: embedded_hal::spi::SpiBus,
    B: AsMut<[u8]>,
{
    type Error = Error<SPI::Error>;

    fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_frame(pixels)?;
        self.send(used)
    }
}

impl<C, SPI, B> OutputAsync<C> for ClocklessSpi<C, SPI, B>
where
    C: Chip,
    SPI: embedded_hal_async::spi::SpiBus,
    B: AsMut<[u8]>,
{
    type Error = Error<SPI::Error>;

    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_frame(pixels)?;
        self.send_async(used).await
    }
}

/// How many buffer bytes [`ClocklessSpi::new`] needs for `led_count` LEDs.
///
/// This is a `const fn`, so you can size an array with it. If `spi_hz` can't
/// drive this LED type, it panics, which inside a `const` is a build error
/// (see [`ClocklessSpi::new`] for what the clock must be).
///
/// # Panics
///
/// Panics if `spi_hz` can't drive this LED type.
///
/// # Example
///
/// ```
/// use ledkit_core::Ws2812b;
/// use ledkit_spi::buffer_len;
///
/// const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30, 3_200_000);
/// let buffer = [0u8; BUFFER_LEN];
/// assert_eq!(buffer.len(), 473);
/// ```
#[must_use]
pub const fn buffer_len<C: Chip>(led_count: usize, spi_hz: u32) -> usize {
    match ClocklessTiming::of::<C>().spi_encoding_byte_aligned(spi_hz) {
        Ok(encoding) => encoding.buffer_len(led_count * C::ORDER.len()),
        Err(_) => panic!("this SPI clock can't drive this LED type; see ClocklessSpi::new's error for working clocks"),
    }
}

/// Like [`buffer_len`], but for [`ClocklessSpi::new_packed`].
///
/// # Panics
///
/// Panics if `spi_hz` can't drive this LED type.
///
/// # Example
///
/// ```
/// use ledkit_core::Ws2812b;
/// use ledkit_spi::buffer_len_packed;
///
/// const BUFFER_LEN: usize = buffer_len_packed::<Ws2812b>(30, 4_000_000);
/// assert!(BUFFER_LEN > 0);
/// ```
#[must_use]
pub const fn buffer_len_packed<C: Chip>(led_count: usize, spi_hz: u32) -> usize {
    match ClocklessTiming::of::<C>().spi_encoding(spi_hz) {
        Ok(encoding) => encoding.buffer_len(led_count * C::ORDER.len()),
        Err(_) => panic!("this SPI clock can't drive this LED type; see ClocklessSpi::new's error for working clocks"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeAsyncBus, FakeBus, FakeError};
    use crate::Error;
    use ledkit_core::{
        wire, BufferTooSmall, Chip, ClocklessTiming, Output, OutputAsync, Rgb, Rgbw, Sk6812Rgbw,
        Strip, TimingError, Ws2812b,
    };

    const HZ: u32 = 3_200_000;

    fn expected<C: Chip>(pixels: &[C::Pixel]) -> Vec<u8>
    where
        C::Pixel: Copy,
    {
        let e = ClocklessTiming::of::<C>()
            .spi_encoding_byte_aligned(HZ)
            .unwrap();
        let mut out = vec![0u8; e.buffer_len(pixels.len() * C::ORDER.len())];
        let n = e
            .encode(
                wire::clockless_bytes::<C, _>(pixels.iter().copied()),
                &mut out,
            )
            .unwrap();
        out.truncate(n);
        out
    }

    /// The reset gap sent before the first frame, in bytes.
    fn gap<C: Chip>() -> usize {
        ClocklessTiming::of::<C>()
            .spi_encoding_byte_aligned(HZ)
            .unwrap()
            .reset_bytes as usize
    }

    /// What the bus gets for a driver's first frame: the reset gap, then the frame.
    fn first_frame<C: Chip>(pixels: &[C::Pixel]) -> Vec<u8>
    where
        C::Pixel: Copy,
    {
        let mut bytes = vec![0u8; gap::<C>()];
        bytes.extend(expected::<C>(pixels));
        bytes
    }

    #[test]
    fn sends_the_encoded_frame_then_flushes() {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 256]).unwrap();
        Output::<Ws2812b>::write(&mut out, [Rgb::RED].into_iter()).unwrap();
        let (bus, _) = out.release();
        assert_eq!(bus.bytes, first_frame::<Ws2812b>(&[Rgb::RED]));
        assert_eq!((bus.writes, bus.flushes), (2, 1));
    }

    #[test]
    fn only_the_first_frame_starts_with_a_reset_gap() {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 256]).unwrap();
        Output::<Ws2812b>::write(&mut out, [Rgb::RED].into_iter()).unwrap();
        Output::<Ws2812b>::write(&mut out, [Rgb::BLUE].into_iter()).unwrap();
        let (bus, _) = out.release();
        let mut want = first_frame::<Ws2812b>(&[Rgb::RED]);
        want.extend(expected::<Ws2812b>(&[Rgb::BLUE]));
        assert_eq!(bus.bytes, want);
        assert_eq!((bus.writes, bus.flushes), (3, 2));
    }

    /// Sends one frame of two LEDs; gives back the bus and the frame's
    /// length without the first frame's reset gap.
    fn frame_of(max: Option<usize>) -> (FakeBus, usize) {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 512]).unwrap();
        if let Some(m) = max {
            out = out.with_max_write_len(m);
        }
        Output::<Ws2812b>::write(&mut out, [Rgb::RED, Rgb::BLUE].into_iter()).unwrap();
        let (bus, _) = out.release();
        let len = bus.bytes.len() - gap::<Ws2812b>();
        (bus, len)
    }

    #[test]
    fn chunked_bytes_match_unchunked() {
        let (whole, _) = frame_of(None);
        for max in [1, 7, 50, 1000] {
            let (chunked, _) = frame_of(Some(max));
            assert_eq!(chunked.bytes, whole.bytes, "max {max}");
        }
    }

    #[test]
    fn chunked_write_and_flush_counts() {
        let (_, len) = frame_of(None);
        let gap = gap::<Ws2812b>();
        for max in [1, 7, 50, 1000] {
            let (bus, _) = frame_of(Some(max));
            assert_eq!(
                bus.writes,
                gap.div_ceil(max) + len.div_ceil(max),
                "max {max}"
            );
            assert_eq!(bus.flushes, 1);
        }
    }

    #[test]
    fn exact_multiple_sends_no_empty_write() {
        let (_, len) = frame_of(None);
        let gap = gap::<Ws2812b>();
        // Divide the frame exactly: every frame write is full, none is empty.
        let (bus, _) = frame_of(Some(len));
        assert_eq!((bus.writes, bus.flushes), (gap.div_ceil(len) + 1, 1));
        let half = (1..len).rev().find(|d| len % d == 0).unwrap();
        let (bus, _) = frame_of(Some(half));
        assert_eq!(bus.writes, gap.div_ceil(half) + len / half);
        assert_eq!(bus.bytes.len(), gap + len);
    }

    #[test]
    fn async_chunking_matches() {
        let (whole, len) = frame_of(None);
        let mut out = ClocklessSpi::<Ws2812b, _, _>::new(FakeAsyncBus::default(), HZ, [0u8; 512])
            .unwrap()
            .with_max_write_len(50);
        embassy_futures::block_on(OutputAsync::<Ws2812b>::write(
            &mut out,
            [Rgb::RED, Rgb::BLUE].into_iter(),
        ))
        .unwrap();
        let (bus, _) = out.release();
        assert_eq!(bus.bytes, whole.bytes);
        assert_eq!(
            (bus.writes, bus.flushes),
            (gap::<Ws2812b>().div_ceil(50) + len.div_ceil(50), 1)
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "byte-aligned encoding")]
    fn packed_encoding_rejects_chunked_writes() {
        let _ = ClocklessSpi::<Ws2812b, _, _>::new_packed(FakeBus::default(), 4_000_000, [0u8; 8])
            .unwrap()
            .with_max_write_len(64);
    }

    #[test]
    #[should_panic(expected = "at least 1 byte")]
    fn zero_max_write_len_panics() {
        let _ = ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 8])
            .unwrap()
            .with_max_write_len(0);
    }

    #[test]
    fn drives_a_strip() {
        const N: usize = 3;
        let mut strip = Strip::<Ws2812b, N, _>::new(
            ClocklessSpi::new(FakeBus::default(), HZ, [0u8; buffer_len::<Ws2812b>(N, HZ)]).unwrap(),
        );
        strip.fill(Rgb::BLUE);
        strip.show().unwrap();
        assert_eq!(strip.output().encoding().bits_per_led_bit, 4);
        assert_eq!(
            strip.output().bus().bytes,
            first_frame::<Ws2812b>(&[Rgb::BLUE; N])
        );
    }

    #[test]
    fn buffer_len_is_const_and_matches_core() {
        const LEN: usize = buffer_len::<Ws2812b>(30, HZ);
        assert_eq!(LEN, 473);
        let e = ClocklessTiming::of::<Sk6812Rgbw>()
            .spi_encoding_byte_aligned(HZ)
            .unwrap();
        assert_eq!(buffer_len::<Sk6812Rgbw>(1, HZ), e.buffer_len(4));
    }

    #[test]
    fn packed_buffer_len_matches_core() {
        let e = ClocklessTiming::of::<Ws2812b>()
            .spi_encoding(4_000_000)
            .unwrap();
        assert_eq!(
            buffer_len_packed::<Ws2812b>(10, 4_000_000),
            e.buffer_len(30)
        );
    }

    #[test]
    fn unusable_clock_is_an_error() {
        assert!(matches!(
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), 1_000_000, [0u8; 8]),
            Err(TimingError::SpiClockUnusable { .. })
        ));
    }

    #[test]
    fn default_is_byte_aligned_packed_is_not() {
        let a =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), 4_000_000, [0u8; 1]).unwrap();
        let p = ClocklessSpi::<Ws2812b, _, _>::new_packed(FakeBus::default(), 4_000_000, [0u8; 1])
            .unwrap();
        assert!(matches!(a.encoding().bits_per_led_bit, 4 | 8));
        assert_eq!(p.encoding().bits_per_led_bit, 5);
    }

    #[test]
    fn too_small_buffer_sends_nothing() {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 10]).unwrap();
        let err = Output::<Ws2812b>::write(&mut out, [Rgb::RED; 2].into_iter()).unwrap_err();
        assert_eq!(
            err,
            Error::BufferTooSmall(BufferTooSmall {
                needed: buffer_len::<Ws2812b>(2, HZ)
            })
        );
        assert_eq!(out.release().0.writes, 0);
    }

    #[test]
    fn zero_leds_sends_lead_in_and_reset() {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeBus::default(), HZ, [0u8; 200]).unwrap();
        Output::<Ws2812b>::write(&mut out, core::iter::empty()).unwrap();
        assert_eq!(out.release().0.bytes, first_frame::<Ws2812b>(&[]));
    }

    #[test]
    fn bus_errors_come_back_as_spi_errors() {
        let bus = FakeBus {
            fail: true,
            ..FakeBus::default()
        };
        let mut out = ClocklessSpi::<Ws2812b, _, _>::new(bus, HZ, [0u8; 256]).unwrap();
        let err = Output::<Ws2812b>::write(&mut out, [Rgb::RED].into_iter()).unwrap_err();
        assert_eq!(err, Error::Spi(FakeError));
    }

    #[test]
    fn async_matches_blocking() {
        let mut out =
            ClocklessSpi::<Ws2812b, _, _>::new(FakeAsyncBus::default(), HZ, [0u8; 256]).unwrap();
        embassy_futures::block_on(OutputAsync::<Ws2812b>::write(
            &mut out,
            [Rgb::RED, Rgb::GREEN].into_iter(),
        ))
        .unwrap();
        let (bus, _) = out.release();
        assert_eq!(bus.bytes, first_frame::<Ws2812b>(&[Rgb::RED, Rgb::GREEN]));
        assert_eq!((bus.writes, bus.flushes), (2, 1));

        let mut rgbw =
            ClocklessSpi::<Sk6812Rgbw, _, _>::new(FakeAsyncBus::default(), HZ, [0u8; 256]).unwrap();
        let pixel = Rgbw::new(1, 2, 3, 4);
        embassy_futures::block_on(OutputAsync::<Sk6812Rgbw>::write(
            &mut rgbw,
            [pixel].into_iter(),
        ))
        .unwrap();
        assert_eq!(rgbw.release().0.bytes, first_frame::<Sk6812Rgbw>(&[pixel]));
    }

    #[test]
    fn async_bus_errors_come_back_as_spi_errors() {
        let bus = FakeAsyncBus {
            fail: true,
            ..FakeAsyncBus::default()
        };
        let mut out = ClocklessSpi::<Ws2812b, _, _>::new(bus, HZ, [0u8; 256]).unwrap();
        let err = embassy_futures::block_on(OutputAsync::<Ws2812b>::write(
            &mut out,
            [Rgb::RED].into_iter(),
        ))
        .unwrap_err();
        assert_eq!(err, Error::Spi(FakeError));
    }
}
