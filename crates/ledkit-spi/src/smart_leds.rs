//! `smart-leds` driver traits for the SPI backends (feature `smart-leds`).
//!
//! These write the colors exactly as given: no brightness, gamma or
//! dithering. That is what `smart-leds` users expect, since they use
//! `smart-leds`' own `brightness()` and `gamma()` adaptors. The channel order
//! of the LED type (for example green-red-blue for a WS2812B) is still
//! applied.

use crate::clocked::{write_frame, write_frame_async};
use crate::{ClockedSpi, ClocklessSpi, Error};
use ledkit_core::interop::SmartLedsColor;
use ledkit_core::{wire, Chip};
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

impl<C, SPI, B> SmartLedsWrite for ClocklessSpi<C, SPI, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    SPI: embedded_hal::spi::SpiBus,
    B: AsMut<[u8]>,
{
    type Error = Error<SPI::Error>;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let used = self.encode_wire(wire_bytes::<C, T, I>(iterator))?;
        self.send(used)
    }
}

impl<C, SPI, B> SmartLedsWriteAsync for ClocklessSpi<C, SPI, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    SPI: embedded_hal_async::spi::SpiBus,
    B: AsMut<[u8]>,
{
    type Error = Error<SPI::Error>;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let used = self.encode_wire(wire_bytes::<C, T, I>(iterator))?;
        self.send_async(used).await
    }
}

impl<C, SPI> SmartLedsWrite for ClockedSpi<C, SPI>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    SPI: embedded_hal::spi::SpiBus,
{
    type Error = Error<SPI::Error>;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        write_frame::<C, _, _>(&mut self.spi, self.header, pixels::<C, T, I>(iterator))
    }
}

impl<C, SPI> SmartLedsWriteAsync for ClockedSpi<C, SPI>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    SPI: embedded_hal_async::spi::SpiBus,
{
    type Error = Error<SPI::Error>;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        write_frame_async::<C, _, _>(&mut self.spi, self.header, pixels::<C, T, I>(iterator)).await
    }
}

/// The user's colors as this chip's pixels.
fn pixels<C, T, I>(iterator: T) -> impl Iterator<Item = C::Pixel>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    T: IntoIterator<Item = I>,
    I: Into<<C::Pixel as SmartLedsColor>::Color>,
{
    iterator.into_iter().map(|color| {
        let color: <C::Pixel as SmartLedsColor>::Color = color.into();
        color.into()
    })
}

/// The wire bytes of the user's colors, one pixel at a time, so the length
/// need not be known up front.
fn wire_bytes<C, T, I>(iterator: T) -> impl Iterator<Item = u8>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    T: IntoIterator<Item = I>,
    I: Into<<C::Pixel as SmartLedsColor>::Color>,
{
    pixels::<C, T, I>(iterator).flat_map(|pixel| wire::clockless_bytes::<C, _>([pixel]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeAsyncBus, FakeBus};
    use crate::{buffer_len, Error};
    use ledkit_core::{
        Apa102, BufferTooSmall, Output, OutputAsync, Rgb, Rgbw, Sk6812Rgbw, Ws2812b,
    };
    use smart_leds_trait::{White, RGB8, RGBW};

    const HZ: u32 = 3_200_000;

    fn clockless<C: Chip, B>(bus: B, len: usize) -> ClocklessSpi<C, B, Vec<u8>> {
        ClocklessSpi::new(bus, HZ, vec![0u8; len]).unwrap()
    }

    #[test]
    fn rgb_matches_strip_output() {
        let mut a = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        let mut b = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        SmartLedsWrite::write(&mut a, [RGB8::new(255, 0, 0), RGB8::new(0, 0, 9)]).unwrap();
        Output::<Ws2812b>::write(&mut b, [Rgb::new(255, 0, 0), Rgb::new(0, 0, 9)].into_iter())
            .unwrap();
        let (bus_a, bus_b) = (a.release().0, b.release().0);
        assert_eq!(bus_a.bytes, bus_b.bytes);
        assert_eq!((bus_a.writes, bus_a.flushes), (1, 1));
    }

    #[test]
    fn rgbw_matches_strip_output() {
        let mut a = clockless::<Sk6812Rgbw, _>(FakeBus::default(), 256);
        let mut b = clockless::<Sk6812Rgbw, _>(FakeBus::default(), 256);
        let color = RGBW {
            r: 1,
            g: 2,
            b: 3,
            a: White(4),
        };
        SmartLedsWrite::write(&mut a, [color]).unwrap();
        Output::<Sk6812Rgbw>::write(&mut b, [Rgbw::new(1, 2, 3, 4)].into_iter()).unwrap();
        assert_eq!(a.release().0.bytes, b.release().0.bytes);
    }

    #[test]
    fn clocked_matches_strip_output() {
        let colors: Vec<RGB8> = (0..17u8).map(|i| RGB8::new(i, 2, 3)).collect();
        let mut a = ClockedSpi::<Apa102, _>::new(FakeBus::default());
        let mut b = ClockedSpi::<Apa102, _>::new(FakeBus::default());
        SmartLedsWrite::write(&mut a, colors.iter().copied()).unwrap();
        Output::<Apa102>::write(&mut b, colors.iter().map(|c| Rgb::new(c.r, c.g, c.b))).unwrap();
        assert_eq!(a.release().bytes, b.release().bytes);
    }

    #[test]
    fn accepts_iterators_without_a_known_length() {
        let colors = (0..10u8)
            .map(|i| RGB8::new(i, 0, 0))
            .filter(|c| c.r % 2 == 0);
        let mut a = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        SmartLedsWrite::write(&mut a, colors).unwrap();
        let mut b = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        let reds: Vec<Rgb> = (0..10u8)
            .filter(|i| i % 2 == 0)
            .map(|i| Rgb::new(i, 0, 0))
            .collect();
        Output::<Ws2812b>::write(&mut b, reds.into_iter()).unwrap();
        assert_eq!(a.release().0.bytes, b.release().0.bytes);
    }

    #[test]
    fn async_matches_blocking() {
        let colors = [RGB8::new(255, 0, 0), RGB8::new(0, 0, 9)];
        let mut blocking = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        SmartLedsWrite::write(&mut blocking, colors).unwrap();
        let mut a = clockless::<Ws2812b, _>(FakeAsyncBus::default(), 256);
        embassy_futures::block_on(SmartLedsWriteAsync::write(&mut a, colors)).unwrap();
        let (bus, _) = a.release();
        assert_eq!(bus.bytes, blocking.release().0.bytes);
        assert_eq!((bus.writes, bus.flushes), (1, 1));

        let mut rgbw = clockless::<Sk6812Rgbw, _>(FakeAsyncBus::default(), 256);
        let c = RGBW {
            r: 1,
            g: 2,
            b: 3,
            a: White(4),
        };
        embassy_futures::block_on(SmartLedsWriteAsync::write(&mut rgbw, [c])).unwrap();
        let mut want = clockless::<Sk6812Rgbw, _>(FakeAsyncBus::default(), 256);
        embassy_futures::block_on(OutputAsync::<Sk6812Rgbw>::write(
            &mut want,
            [Rgbw::new(1, 2, 3, 4)].into_iter(),
        ))
        .unwrap();
        assert_eq!(rgbw.release().0.bytes, want.release().0.bytes);

        let colors: Vec<RGB8> = (0..17u8).map(|i| RGB8::new(i, 2, 3)).collect();
        let mut clocked = ClockedSpi::<Apa102, _>::new(FakeAsyncBus::default());
        embassy_futures::block_on(SmartLedsWriteAsync::write(&mut clocked, colors.clone()))
            .unwrap();
        let mut want = ClockedSpi::<Apa102, _>::new(FakeBus::default());
        SmartLedsWrite::write(&mut want, colors).unwrap();
        assert_eq!(clocked.release().bytes, want.release().bytes);
    }

    #[test]
    fn chunked_writes_match_unchunked() {
        let colors: Vec<RGB8> = (0..10u8).map(|i| RGB8::new(i, 2, 3)).collect();
        let mut whole = clockless::<Ws2812b, _>(FakeBus::default(), 512);
        SmartLedsWrite::write(&mut whole, colors.clone()).unwrap();
        let whole = whole.release().0;

        let mut chunked = clockless::<Ws2812b, _>(FakeBus::default(), 512).with_max_write_len(50);
        SmartLedsWrite::write(&mut chunked, colors.clone()).unwrap();
        let bus = chunked.release().0;
        assert_eq!(bus.bytes, whole.bytes);
        assert_eq!(
            (bus.writes, bus.flushes),
            (whole.bytes.len().div_ceil(50), 1)
        );

        let mut a = clockless::<Ws2812b, _>(FakeAsyncBus::default(), 512).with_max_write_len(50);
        embassy_futures::block_on(SmartLedsWriteAsync::write(&mut a, colors)).unwrap();
        let bus = a.release().0;
        assert_eq!(bus.bytes, whole.bytes);
        assert_eq!(
            (bus.writes, bus.flushes),
            (whole.bytes.len().div_ceil(50), 1)
        );
    }

    #[test]
    fn empty_input_sends_the_empty_frame() {
        let mut a = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        let mut b = clockless::<Ws2812b, _>(FakeBus::default(), 256);
        SmartLedsWrite::write(&mut a, core::iter::empty::<RGB8>()).unwrap();
        Output::<Ws2812b>::write(&mut b, core::iter::empty()).unwrap();
        let bytes = a.release().0.bytes;
        assert_eq!(bytes, b.release().0.bytes);
        assert_ne!(bytes.len(), 0);

        let mut c = ClockedSpi::<Apa102, _>::new(FakeBus::default());
        SmartLedsWrite::write(&mut c, core::iter::empty::<RGB8>()).unwrap();
        // No LEDs: just the 4-byte start frame (the APA102 end frame is empty).
        assert_eq!(c.release().bytes, [0u8; 4]);
    }

    #[test]
    fn too_small_buffer_sends_nothing() {
        let mut out = clockless::<Ws2812b, _>(FakeBus::default(), 10);
        let err = SmartLedsWrite::write(&mut out, [RGB8::new(1, 2, 3); 2]).unwrap_err();
        assert_eq!(
            err,
            Error::BufferTooSmall(BufferTooSmall {
                needed: buffer_len::<Ws2812b>(2, HZ)
            })
        );
        assert_eq!(out.release().0.writes, 0);
    }
}
