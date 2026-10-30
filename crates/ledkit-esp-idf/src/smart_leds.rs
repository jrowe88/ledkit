//! `smart-leds` driver trait for [`RmtOutput`] (feature `smart-leds`).
//!
//! This writes the colors exactly as given: no brightness, gamma or
//! dithering. That is what `smart-leds` users expect, since they use
//! `smart-leds`' own `brightness()` and `gamma()` adaptors. The channel order
//! of the LED type (for example green-red-blue for a WS2812B) is still
//! applied.

use crate::RmtOutput;
use esp_idf_hal::sys::EspError;
use ledkit_core::interop::SmartLedsColor;
use ledkit_core::{wire, Chip};
use smart_leds_trait::SmartLedsWrite;

impl<C> SmartLedsWrite for RmtOutput<'_, C>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
{
    type Error = EspError;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        // One pixel at a time, so the length need not be known up front.
        let wire_bytes = iterator.into_iter().flat_map(|color| {
            let color: <C::Pixel as SmartLedsColor>::Color = color.into();
            let pixel: C::Pixel = color.into();
            wire::clockless_bytes::<C, _>([pixel])
        });
        self.send_bytes(wire_bytes)
    }
}
