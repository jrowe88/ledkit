//! `smart-leds` driver traits for [`PwmOutput`] (feature `smart-leds`).
//!
//! These write the colors exactly as given: no brightness, gamma or
//! dithering. That is what `smart-leds` users expect, since they use
//! `smart-leds`' own `brightness()` and `gamma()` adaptors. The channel order
//! of the LED type (for example green-red-blue for a WS2812B) is still
//! applied.

use crate::{Error, PwmOutput};
use ledkit_core::interop::SmartLedsColor;
use ledkit_core::{wire, Chip};
use smart_leds_trait::SmartLedsWrite;
#[cfg(feature = "async")]
use smart_leds_trait::SmartLedsWriteAsync;

impl<C, B> SmartLedsWrite for PwmOutput<'_, C, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    B: AsMut<[u16]>,
{
    type Error = Error;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let used = self.encode_wire(wire_bytes::<C, T, I>(iterator))?;
        self.send_blocking(used)
    }
}

#[cfg(feature = "async")]
impl<C, B> SmartLedsWriteAsync for PwmOutput<'_, C, B>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    B: AsMut<[u16]>,
{
    type Error = Error;
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
    iterator.into_iter().flat_map(|color| {
        let color: <C::Pixel as SmartLedsColor>::Color = color.into();
        let pixel: C::Pixel = color.into();
        wire::clockless_bytes::<C, _>([pixel])
    })
}
