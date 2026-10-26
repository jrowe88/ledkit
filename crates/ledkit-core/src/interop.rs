//! Use any `smart-leds` driver as a ledkit output (feature `smart-leds`).
//!
//! Wrap a driver in [`SmartLedsOutput`] and hand it to a
//! [`Strip`](crate::Strip). ledkit still does brightness, gamma, color
//! correction and power limiting; the driver only sends the result.

use crate::{Chip, Output, OutputAsync, Pixel, Rgb, Rgbw};
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

/// A ledkit pixel that has an equivalent `smart-leds` color type.
///
/// [`Rgb`] becomes [`smart_leds_trait::RGB8`] and [`Rgbw`] becomes
/// [`smart_leds_trait::RGBW<u8>`].
///
/// # Example
///
/// ```
/// use ledkit_core::interop::SmartLedsColor;
/// use ledkit_core::Rgb;
///
/// let c = Rgb::new(1, 2, 3).to_smart_leds();
/// assert_eq!((c.r, c.g, c.b), (1, 2, 3));
/// ```
pub trait SmartLedsColor: Pixel {
    /// The matching `smart-leds` color type.
    type Color;

    /// Converts this pixel to the `smart-leds` color type.
    fn to_smart_leds(self) -> Self::Color;
}

impl SmartLedsColor for Rgb {
    type Color = smart_leds_trait::RGB8;

    fn to_smart_leds(self) -> Self::Color {
        self.into()
    }
}

impl SmartLedsColor for Rgbw {
    type Color = smart_leds_trait::RGBW<u8>;

    fn to_smart_leds(self) -> Self::Color {
        self.into()
    }
}

/// Lets any `smart-leds` driver act as a ledkit [`Output`] (or
/// [`OutputAsync`]).
///
/// The `smart-leds` driver applies its own color order, so ledkit's channel
/// order for the chip is not used on this path. Pick the ledkit chip type
/// only for its pixel type: an RGB chip for an RGB driver, an RGBW chip (such
/// as [`Sk6812Rgbw`](crate::Sk6812Rgbw)) for an RGBW driver.
///
/// # Example
///
/// ```
/// use core::convert::Infallible;
/// use ledkit_core::interop::SmartLedsOutput;
/// use ledkit_core::{Rgb, Strip, Ws2812b};
/// use smart_leds_trait::{SmartLedsWrite, RGB8};
///
/// /// A tiny stand-in for a real driver: it counts the pixels it is given.
/// struct Counter(usize);
///
/// impl SmartLedsWrite for Counter {
///     type Error = Infallible;
///     type Color = RGB8;
///     fn write<T, I>(&mut self, iterator: T) -> Result<(), Infallible>
///     where
///         T: IntoIterator<Item = I>,
///         I: Into<RGB8>,
///     {
///         self.0 = iterator.into_iter().count();
///         Ok(())
///     }
/// }
///
/// let mut strip: Strip<Ws2812b, 3, SmartLedsOutput<Counter>> =
///     Strip::new(SmartLedsOutput(Counter(0)));
/// strip.fill(Rgb::RED);
/// strip.show().unwrap();
/// assert_eq!(strip.output().0 .0, 3);
/// ```
#[derive(Debug, Clone)]
pub struct SmartLedsOutput<D>(
    /// The wrapped `smart-leds` driver.
    pub D,
);

impl<C, D> Output<C> for SmartLedsOutput<D>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    D: SmartLedsWrite,
    D::Color: From<<C::Pixel as SmartLedsColor>::Color>,
{
    type Error = D::Error;

    fn write<I>(&mut self, pixels: I) -> Result<(), D::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.0.write(pixels.map(SmartLedsColor::to_smart_leds))
    }
}

impl<C, D> OutputAsync<C> for SmartLedsOutput<D>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    D: SmartLedsWriteAsync,
    D::Color: From<<C::Pixel as SmartLedsColor>::Color>,
{
    type Error = D::Error;

    async fn write<I>(&mut self, pixels: I) -> Result<(), D::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.0
            .write(pixels.map(SmartLedsColor::to_smart_leds))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Brightness, Rgb, Rgbw, Sk6812Rgbw, Strip, Ws2812b};
    use core::convert::Infallible;
    use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync, RGB8, RGBW};
    use std::vec::Vec;

    #[derive(Default)]
    struct FakeDriver {
        received: Vec<RGB8>,
    }

    impl SmartLedsWrite for FakeDriver {
        type Error = Infallible;
        type Color = RGB8;
        fn write<T, I>(&mut self, iterator: T) -> Result<(), Infallible>
        where
            T: IntoIterator<Item = I>,
            I: Into<RGB8>,
        {
            self.received = iterator.into_iter().map(Into::into).collect();
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeRgbwDriver {
        received: Vec<RGBW<u8>>,
    }

    impl SmartLedsWrite for FakeRgbwDriver {
        type Error = Infallible;
        type Color = RGBW<u8>;
        fn write<T, I>(&mut self, iterator: T) -> Result<(), Infallible>
        where
            T: IntoIterator<Item = I>,
            I: Into<RGBW<u8>>,
        {
            self.received = iterator.into_iter().map(Into::into).collect();
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeAsyncDriver {
        received: Vec<RGB8>,
    }

    impl SmartLedsWriteAsync for FakeAsyncDriver {
        type Error = Infallible;
        type Color = RGB8;
        #[allow(clippy::unused_async_trait_impl)] // the trait requires an async fn; a fake has nothing to wait for
        async fn write<T, I>(&mut self, iterator: T) -> Result<(), Infallible>
        where
            T: IntoIterator<Item = I>,
            I: Into<RGB8>,
        {
            self.received = iterator.into_iter().map(Into::into).collect();
            Ok(())
        }
    }

    #[test]
    fn strip_drives_an_async_smart_leds_driver() {
        let mut s: Strip<Ws2812b, 2, SmartLedsOutput<FakeAsyncDriver>> =
            Strip::new(SmartLedsOutput(FakeAsyncDriver::default()));
        s.set(1, Rgb::new(7, 8, 9));
        s.set_brightness(Brightness::exact(128));
        s.set_dithering(false);
        embassy_futures::block_on(s.show_async()).unwrap();
        // (v * 129 + 128) >> 8 for 7, 8, 9.
        assert_eq!(
            s.output().0.received,
            [RGB8::new(0, 0, 0), RGB8::new(4, 4, 5)]
        );
    }

    #[test]
    fn strip_drives_a_smart_leds_driver() {
        let mut s: Strip<Ws2812b, 2, SmartLedsOutput<FakeDriver>> =
            Strip::new(SmartLedsOutput(FakeDriver::default()));
        s.set(0, Rgb::new(1, 2, 3));
        s.show().unwrap();
        assert_eq!(
            s.output().0.received,
            [RGB8::new(1, 2, 3), RGB8::new(0, 0, 0)]
        );
    }

    #[test]
    fn rgbw_strip_drives_an_rgbw_driver() {
        let mut s: Strip<Sk6812Rgbw, 1, SmartLedsOutput<FakeRgbwDriver>> =
            Strip::new(SmartLedsOutput(FakeRgbwDriver::default()));
        s.set(0, Rgbw::new(1, 2, 3, 4));
        s.show().unwrap();
        let got = s.output().0.received[0];
        assert_eq!((got.r, got.g, got.b, got.a.0), (1, 2, 3, 4));
    }
}
