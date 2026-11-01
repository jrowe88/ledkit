//! DotStars over SPI.

use crate::Error;
use core::iter::Once;
use core::marker::PhantomData;
use ledkit_core::wire::{clockless_bytes, ClocklessBytes};
use ledkit_core::{Chip, ClockedFormat, Output, OutputAsync, Protocol};

/// How many bytes are collected before each write to the bus. Small enough
/// for the stack of a tiny microcontroller, big enough to keep the bus busy.
const CHUNK: usize = 64;

/// Sends DotStar-style (data wire plus clock wire) LEDs over an SPI bus.
///
/// DotStars (APA102 and SK9822) have their own clock wire, so SPI fits them
/// naturally: the bus clock and data go straight to the LEDs and no bit
/// pattern tricks are needed.
///
/// - `C` is your LED type, such as [`Apa102`](ledkit_core::Apa102) or
///   [`Sk9822`](ledkit_core::Sk9822).
/// - `SPI` is your SPI bus. It can be any `embedded-hal` 1.0 `SpiBus`
///   (blocking) or `embedded-hal-async` `SpiBus` (async).
///
/// **Wiring:** connect the bus data pin (MOSI) to the LED strip data input
/// (DI) and the bus clock pin (SCK) to the strip clock input (CI).
///
/// **Speed:** because the LEDs get a clock, any SPI speed works, up to the
/// chip's own limit. Until those limits are measured, 8 MHz or less is a safe
/// choice.
///
/// **ledkit never sets up the bus.** You create and configure the SPI
/// peripheral, its clock speed and its pins with your board's HAL, then pass
/// the bus here.
///
/// The frame is sent in small pieces from a little stack buffer, so no
/// buffer of your own is needed. DotStars need no reset gap: the end of the
/// frame is part of what is sent, and the driver waits for the bus to finish
/// (`flush`) before returning.
///
/// You usually don't call `write` yourself. Put the driver in a
/// [`Strip`](ledkit_core::Strip) and call `strip.show()`.
///
/// # NeoPixels
///
/// NeoPixels (WS2812B and friends) have no clock wire. Using one here is a
/// build error; use [`ClocklessSpi`](crate::ClocklessSpi) instead.
///
/// ```compile_fail
/// # use ledkit_core::Ws2812b;
/// # use ledkit_spi::ClockedSpi;
/// # struct Bus;
/// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
/// # impl embedded_hal::spi::SpiBus for Bus {
/// #     fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn write(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// // NeoPixels need ClocklessSpi.
/// let _ = ClockedSpi::<Ws2812b, _>::new(Bus);
/// ```
///
/// # Example
///
/// ```
/// use ledkit_core::{Apa102, Rgb, Strip};
/// use ledkit_spi::ClockedSpi;
/// # struct Bus;
/// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
/// # impl embedded_hal::spi::SpiBus for Bus {
/// #     fn read(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn write(&mut self, _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer(&mut self, _: &mut [u8], _: &[u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn transfer_in_place(&mut self, _: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
/// #     fn flush(&mut self) -> Result<(), Self::Error> { Ok(()) }
/// # }
/// # let spi = Bus; // your HAL SPI bus, at 8 MHz or less
///
/// let mut strip = Strip::<Apa102, 8, _>::new(ClockedSpi::new(spi));
/// strip.fill(Rgb::BLUE);
/// strip.show().unwrap();
/// ```
#[derive(Debug)]
pub struct ClockedSpi<C: Chip, SPI> {
    pub(crate) spi: SPI,
    /// Each LED's first byte: `0xE0` plus the 5-bit global brightness.
    pub(crate) header: u8,
    chip: PhantomData<C>,
}

impl<C: Chip, SPI> ClockedSpi<C, SPI> {
    /// Creates a driver for a DotStar type on this SPI bus.
    ///
    /// Using a NeoPixel type is a build error; see the type docs.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Sk9822;
    /// use ledkit_spi::ClockedSpi;
    /// # struct Bus(u32);
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    ///
    /// let output = ClockedSpi::<Sk9822, _>::new(Bus(7));
    /// assert_eq!(output.bus().0, 7);
    /// ```
    #[must_use]
    pub fn new(spi: SPI) -> Self {
        const {
            assert!(
                matches!(C::PROTOCOL, Protocol::Clocked(_)),
                "ClockedSpi is for DotStars (APA102, SK9822); NeoPixels need ClocklessSpi"
            );
        }
        Self {
            spi,
            header: 0xFF,
            chip: PhantomData,
        }
    }

    /// The SPI bus, for example to check its settings.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Apa102;
    /// use ledkit_spi::ClockedSpi;
    /// # struct Bus(u32);
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    ///
    /// let output = ClockedSpi::<Apa102, _>::new(Bus(7));
    /// assert_eq!(output.bus().0, 7);
    /// ```
    #[must_use]
    pub fn bus(&self) -> &SPI {
        &self.spi
    }

    /// Takes the driver apart and gives back the SPI bus.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Apa102;
    /// use ledkit_spi::ClockedSpi;
    /// # struct Bus(u32);
    /// # impl embedded_hal::spi::ErrorType for Bus { type Error = core::convert::Infallible; }
    ///
    /// let spi = ClockedSpi::<Apa102, _>::new(Bus(7)).release();
    /// assert_eq!(spi.0, 7);
    /// ```
    #[must_use]
    pub fn release(self) -> SPI {
        self.spi
    }
}

impl<C, SPI> Output<C> for ClockedSpi<C, SPI>
where
    C: Chip,
    SPI: embedded_hal::spi::SpiBus,
{
    type Error = Error<SPI::Error>;

    /// Sends `level` (0 to 31; larger counts as 31) as every LED's global
    /// brightness from the next frame on. This is how
    /// [`Strip::set_hd_brightness`](ledkit_core::Strip::set_hd_brightness) works.
    fn set_global_brightness(&mut self, level: u8) -> bool {
        self.header = 0xE0 | level.min(31);
        true
    }

    fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        write_frame::<C, _, _>(&mut self.spi, self.header, pixels)
    }
}

impl<C, SPI> OutputAsync<C> for ClockedSpi<C, SPI>
where
    C: Chip,
    SPI: embedded_hal_async::spi::SpiBus,
{
    type Error = Error<SPI::Error>;

    /// The same as the blocking
    /// [`set_global_brightness`](Output::set_global_brightness).
    fn set_global_brightness(&mut self, level: u8) -> bool {
        self.header = 0xE0 | level.min(31);
        true
    }

    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        write_frame_async::<C, _, _>(&mut self.spi, self.header, pixels).await
    }
}

/// Sends one DotStar frame over a blocking bus, then flushes.
///
/// Works with any pixel iterator, even one that does not know its length.
/// `header` is each LED's first byte (`0xE0` plus its global brightness).
pub(crate) fn write_frame<C, SPI, I>(
    spi: &mut SPI,
    header: u8,
    pixels: I,
) -> Result<(), Error<SPI::Error>>
where
    C: Chip,
    SPI: embedded_hal::spi::SpiBus,
    I: Iterator<Item = C::Pixel>,
{
    let mut bytes = FrameBytes::<C, I>::new(header, pixels);
    let mut chunk = [0u8; CHUNK];
    loop {
        let filled = bytes.fill(&mut chunk);
        if filled == 0 {
            break;
        }
        spi.write(&chunk[..filled]).map_err(Error::Spi)?;
    }
    spi.flush().map_err(Error::Spi)
}

/// The async twin of [`write_frame`]. Only the bus calls differ; the frame
/// itself comes from the shared `FrameBytes`.
pub(crate) async fn write_frame_async<C, SPI, I>(
    spi: &mut SPI,
    header: u8,
    pixels: I,
) -> Result<(), Error<SPI::Error>>
where
    C: Chip,
    SPI: embedded_hal_async::spi::SpiBus,
    I: Iterator<Item = C::Pixel>,
{
    let mut bytes = FrameBytes::<C, I>::new(header, pixels);
    let mut chunk = [0u8; CHUNK];
    loop {
        let filled = bytes.fill(&mut chunk);
        if filled == 0 {
            break;
        }
        spi.write(&chunk[..filled]).await.map_err(Error::Spi)?;
    }
    spi.flush().await.map_err(Error::Spi)
}

/// Where [`FrameBytes`] is in the frame.
enum Stage {
    /// Sending the zero bytes of the start frame; this many are left.
    Start(usize),
    /// Sending LEDs.
    Leds,
    /// Sending the bytes of the end frame; this many are left.
    End(usize),
}

/// The bytes of a DotStar frame, made on demand from any pixel iterator.
///
/// The end frame depends on the chip's [`ClockedFormat`]: `ceil(n / 16)` bytes
/// of `0xFF` for the APA102, or 4 reset bytes plus `ceil(n / 16)` bytes, all
/// `0x00`, for the SK9822 (see
/// [`wire::clocked_frame`](ledkit_core::wire::clocked_frame) for why). It
/// depends on the LED count `n`, which is counted as the pixels stream
/// through, so the iterator needs no known length.
struct FrameBytes<C: Chip, I> {
    pixels: I,
    stage: Stage,
    /// The channel bytes still to send for the LED in progress.
    channels: Option<ClocklessBytes<C, Once<C::Pixel>>>,
    led_count: usize,
    /// Each LED's first byte: `0xE0` plus the global brightness.
    header: u8,
}

impl<C: Chip, I: Iterator<Item = C::Pixel>> FrameBytes<C, I> {
    fn new(header: u8, pixels: I) -> Self {
        Self {
            header,
            pixels,
            stage: Stage::Start(4),
            channels: None,
            led_count: 0,
        }
    }

    /// Fills `chunk` from the front and returns how many bytes it got; 0
    /// means the frame is finished.
    fn fill(&mut self, chunk: &mut [u8]) -> usize {
        let mut filled = 0;
        for slot in chunk.iter_mut() {
            let Some(byte) = self.next() else { break };
            *slot = byte;
            filled += 1;
        }
        filled
    }
}

impl<C: Chip, I: Iterator<Item = C::Pixel>> Iterator for FrameBytes<C, I> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        loop {
            match &mut self.stage {
                Stage::Start(left) if *left > 0 => {
                    *left -= 1;
                    return Some(0x00);
                }
                Stage::End(left) if *left > 0 => {
                    *left -= 1;
                    return Some(ClockedFormat::of::<C>().end_frame_byte());
                }
                Stage::Start(_) => self.stage = Stage::Leds,
                Stage::Leds => {
                    if let Some(byte) = self.channels.as_mut().and_then(Iterator::next) {
                        return Some(byte);
                    }
                    if let Some(pixel) = self.pixels.next() {
                        self.led_count += 1;
                        self.channels = Some(clockless_bytes::<C, _>(core::iter::once(pixel)));
                        // Every LED starts with its global brightness.
                        return Some(self.header);
                    }
                    self.stage = Stage::End(ClockedFormat::of::<C>().end_frame_len(self.led_count));
                }
                Stage::End(_) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeAsyncBus, FakeBus, FakeError};
    use crate::Error;
    use ledkit_core::{wire, Apa102, Brightness, Output, OutputAsync, Rgb, Sk9822, Strip};

    fn frame_matches_core_for<C: Chip<Pixel = Rgb>>() {
        for n in [0usize, 1, 15, 16, 17, 40, 64] {
            let pixels: Vec<Rgb> = (0..n).map(|i| Rgb::new(i as u8, 2, 3)).collect();
            let mut out = ClockedSpi::<C, _>::new(FakeBus::default());
            Output::<C>::write(&mut out, pixels.iter().copied()).unwrap();
            let bus = out.release();
            let want: Vec<u8> = wire::clocked_frame::<C, _>(pixels.iter().copied()).collect();
            assert_eq!(bus.bytes, want);
            assert_eq!(bus.bytes.len(), wire::clocked_frame_len::<C>(n));
            assert_eq!(
                bus.writes,
                wire::clocked_frame_len::<C>(n).div_ceil(64).max(1)
            );
            assert_eq!(bus.flushes, 1);
        }
    }

    #[test]
    fn frame_matches_core_for_many_lengths() {
        frame_matches_core_for::<Apa102>();
        frame_matches_core_for::<Sk9822>();
    }

    #[test]
    fn end_frames_by_chip() {
        for n in [0usize, 1, 15, 16, 17, 64] {
            let clocks = n.div_ceil(16);
            let pixels = vec![Rgb::new(1, 2, 3); n];

            let mut apa = FakeBus::default();
            write_frame::<Apa102, _, _>(&mut apa, 0xFF, pixels.iter().copied()).unwrap();
            assert_eq!(apa.bytes[4 + 4 * n..], vec![0xFF; clocks], "apa102 n = {n}");

            let mut sk = FakeBus::default();
            write_frame::<Sk9822, _, _>(&mut sk, 0xFF, pixels.iter().copied()).unwrap();
            assert_eq!(
                sk.bytes[4 + 4 * n..],
                vec![0x00; 4 + clocks],
                "sk9822 n = {n}"
            );
        }
    }

    #[test]
    fn frame_does_not_need_a_known_length() {
        let pixels = (0..20u8)
            .map(|i| Rgb::new(i, 2, 3))
            .filter(|pixel| pixel.r % 2 == 0);
        let mut bus = FakeBus::default();
        write_frame::<Apa102, _, _>(&mut bus, 0xFF, pixels.clone()).unwrap();
        let pixels: Vec<Rgb> = pixels.collect();
        let want: Vec<u8> = wire::clocked_frame::<Apa102, _>(pixels.iter().copied()).collect();
        assert_eq!(bus.bytes, want);
        let mut bus = FakeBus::default();
        write_frame::<Sk9822, _, _>(&mut bus, 0xFF, pixels.iter().copied()).unwrap();
        let want: Vec<u8> = wire::clocked_frame::<Sk9822, _>(pixels.iter().copied()).collect();
        assert_eq!(bus.bytes, want);
    }

    #[test]
    fn drives_a_strip_of_sk9822() {
        let mut strip = Strip::<Sk9822, 4, _>::new(ClockedSpi::new(FakeBus::default()));
        strip.fill(Rgb::RED);
        strip.show().unwrap();
        let bus = strip.output().bus();
        assert_eq!(bus.bytes.len(), wire::clocked_frame_len::<Sk9822>(4));
        assert_eq!(bus.flushes, 1);
    }

    #[test]
    fn hd_strip_sends_the_global_brightness() {
        let mut strip = Strip::<Apa102, 3, _>::new(ClockedSpi::new(FakeBus::default()));
        strip.set_dithering(false);
        strip.set_brightness(Brightness::exact(3));
        strip.set_hd_brightness(true);
        strip.fill(Rgb::new(255, 128, 0));
        strip.show().unwrap();
        // Level 3 is global 2, channels scaled by 47.
        let want: Vec<u8> = wire::clocked_frame::<Apa102, _>([Rgb::new(48, 24, 0); 3])
            .with_global_brightness(2)
            .collect();
        assert_eq!(strip.output().bus().bytes, want);
        assert_eq!(want[4], 0xE2);

        // Turning HD off goes back to full global brightness.
        strip.set_hd_brightness(false);
        let mut async_strip = Strip::<Apa102, 3, _>::new(ClockedSpi::new(FakeAsyncBus::default()));
        async_strip.set_dithering(false);
        async_strip.set_brightness(Brightness::exact(3));
        async_strip.set_hd_brightness(true);
        async_strip.fill(Rgb::new(255, 128, 0));
        embassy_futures::block_on(async_strip.show_async()).unwrap();
        assert_eq!(async_strip.output().bus().bytes, want);
        strip.show().unwrap();
        let plain: Vec<u8> = wire::clocked_frame::<Apa102, _>([Rgb::new(4, 2, 0); 3]).collect();
        assert_eq!(strip.output().bus().bytes[want.len()..], plain);
    }

    #[test]
    fn global_brightness_is_capped_at_31() {
        let mut out = ClockedSpi::<Apa102, _>::new(FakeBus::default());
        assert!(Output::<Apa102>::set_global_brightness(&mut out, 200));
        Output::<Apa102>::write(&mut out, [Rgb::BLACK].into_iter()).unwrap();
        assert_eq!(out.bus().bytes[4], 0xFF);
    }

    #[test]
    fn async_matches_blocking() {
        let pixels: Vec<Rgb> = (0..17u8).map(|i| Rgb::new(i, 2, 3)).collect();
        let mut out = ClockedSpi::<Apa102, _>::new(FakeAsyncBus::default());
        embassy_futures::block_on(OutputAsync::<Apa102>::write(
            &mut out,
            pixels.iter().copied(),
        ))
        .unwrap();
        let bus = out.release();
        let want: Vec<u8> = wire::clocked_frame::<Apa102, _>(pixels.iter().copied()).collect();
        assert_eq!(bus.bytes, want);
        assert_eq!(bus.flushes, 1);
    }

    #[test]
    fn bus_errors_come_back_as_spi_errors() {
        let bus = FakeBus {
            fail: true,
            ..FakeBus::default()
        };
        let mut out = ClockedSpi::<Apa102, _>::new(bus);
        let err = Output::<Apa102>::write(&mut out, [Rgb::RED].into_iter()).unwrap_err();
        assert_eq!(err, Error::Spi(FakeError));
    }

    #[test]
    fn async_bus_errors_come_back_as_spi_errors() {
        let bus = FakeAsyncBus {
            fail: true,
            ..FakeAsyncBus::default()
        };
        let mut out = ClockedSpi::<Apa102, _>::new(bus);
        let err = embassy_futures::block_on(OutputAsync::<Apa102>::write(
            &mut out,
            [Rgb::RED].into_iter(),
        ))
        .unwrap_err();
        assert_eq!(err, Error::Spi(FakeError));
    }
}
