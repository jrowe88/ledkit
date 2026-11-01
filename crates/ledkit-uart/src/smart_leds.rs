//! `smart-leds` write traits for [`ClocklessUart`].

use crate::{ClocklessUart, Error};
use ledkit_core::interop::SmartLedsColor;
use ledkit_core::{wire, Chip};
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

impl<C, U, D, B> SmartLedsWrite for ClocklessUart<C, U, D, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    U: embedded_io::Write,
    D: embedded_hal::delay::DelayNs,
    B: AsMut<[u8]>,
{
    type Error = Error<U::Error>;
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

impl<C, U, D, B> SmartLedsWriteAsync for ClocklessUart<C, U, D, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    U: embedded_io_async::Write,
    D: embedded_hal_async::delay::DelayNs,
    B: AsMut<[u8]>,
{
    type Error = Error<U::Error>;
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

/// The user's colors as this chip's wire bytes.
fn wire_bytes<C, T, I>(iterator: T) -> impl Iterator<Item = u8>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    T: IntoIterator<Item = I>,
    I: Into<<C::Pixel as SmartLedsColor>::Color>,
{
    iterator.into_iter().flat_map(|color| {
        let color: <C::Pixel as SmartLedsColor>::Color = color.into();
        let pixel: C::Pixel = color.into();
        wire::clockless_bytes::<C, _>([pixel])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{FakeDelay, FakePort};
    use ledkit_core::{Output, OutputAsync, Rgb, Rgbw, Sk6812Rgbw, Ws2812b};
    use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync, White, RGB8, RGBW};

    const BAUD: u32 = 4_000_000;

    fn uart<C: Chip>() -> ClocklessUart<C, FakePort, FakeDelay, Vec<u8>> {
        ClocklessUart::new(
            FakePort::default(),
            BAUD,
            FakeDelay::default(),
            vec![0u8; 256],
        )
        .unwrap()
    }

    #[test]
    fn rgb_matches_strip_output() {
        let mut a = uart::<Ws2812b>();
        let mut b = uart::<Ws2812b>();
        SmartLedsWrite::write(&mut a, [RGB8::new(255, 0, 0), RGB8::new(0, 0, 9)]).unwrap();
        Output::<Ws2812b>::write(&mut b, [Rgb::new(255, 0, 0), Rgb::new(0, 0, 9)].into_iter())
            .unwrap();
        let ((port_a, delay_a, _), (port_b, _, _)) = (a.release(), b.release());
        assert_eq!(port_a.bytes, port_b.bytes);
        assert_eq!((port_a.flushes, delay_a.waits_us), (2, vec![280, 280]));
    }

    #[test]
    fn rgbw_matches_strip_output() {
        let mut a = uart::<Sk6812Rgbw>();
        let mut b = uart::<Sk6812Rgbw>();
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
    fn async_matches_strip_output() {
        let mut a = uart::<Ws2812b>();
        let mut b = uart::<Ws2812b>();
        embassy_futures::block_on(SmartLedsWriteAsync::write(&mut a, [RGB8::new(1, 2, 3)]))
            .unwrap();
        embassy_futures::block_on(OutputAsync::<Ws2812b>::write(
            &mut b,
            [Rgb::new(1, 2, 3)].into_iter(),
        ))
        .unwrap();
        assert_eq!(a.release().0.bytes, b.release().0.bytes);
    }
}
