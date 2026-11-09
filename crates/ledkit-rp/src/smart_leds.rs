//! `smart-leds` driver traits for [`PioOutput`] and [`PioDmaOutput`] (feature
//! `smart-leds`).
//!
//! These write the colors exactly as given: no brightness, gamma or dithering.
//! That is what `smart-leds` users expect, since they use `smart-leds`' own
//! `brightness()` and `gamma()` adaptors. The channel order of the LED type (for
//! example green-red-blue for a WS2812B) is still applied.

use crate::hal::dma::SingleChannel;
use crate::hal::gpio::AnyPin;
use crate::hal::pio::{PIOExt, StateMachineIndex};
use crate::{Error, MicrosecondClock, PioDmaOutput, PioOutput};
use ledkit_core::interop::SmartLedsColor;
use ledkit_core::{wire, Chip};
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

impl<C, P, SM, I, T> SmartLedsWrite for PioOutput<C, P, SM, I, T>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    type Error = Error;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<It, Col>(&mut self, iterator: It) -> Result<(), Self::Error>
    where
        It: IntoIterator<Item = Col>,
        Col: Into<Self::Color>,
    {
        self.send_blocking(wire_bytes::<C, It, Col>(iterator));
        Ok(())
    }
}

impl<C, P, SM, I, T> SmartLedsWriteAsync for PioOutput<C, P, SM, I, T>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    type Error = Error;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    async fn write<It, Col>(&mut self, iterator: It) -> Result<(), Self::Error>
    where
        It: IntoIterator<Item = Col>,
        Col: Into<Self::Color>,
    {
        self.send_async(wire_bytes::<C, It, Col>(iterator)).await;
        Ok(())
    }
}

impl<C, P, SM, I, CH, T> SmartLedsWrite for PioDmaOutput<C, P, SM, I, CH, T>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    CH: SingleChannel,
    T: MicrosecondClock,
{
    type Error = Error;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    fn write<It, Col>(&mut self, iterator: It) -> Result<(), Self::Error>
    where
        It: IntoIterator<Item = Col>,
        Col: Into<Self::Color>,
    {
        self.send_blocking(wire_bytes::<C, It, Col>(iterator))
    }
}

impl<C, P, SM, I, CH, T> SmartLedsWriteAsync for PioDmaOutput<C, P, SM, I, CH, T>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    CH: SingleChannel,
    T: MicrosecondClock,
{
    type Error = Error;
    type Color = <C::Pixel as SmartLedsColor>::Color;

    async fn write<It, Col>(&mut self, iterator: It) -> Result<(), Self::Error>
    where
        It: IntoIterator<Item = Col>,
        Col: Into<Self::Color>,
    {
        self.send_async(wire_bytes::<C, It, Col>(iterator)).await
    }
}

/// The wire bytes of the user's colors, one pixel at a time, so the length need
/// not be known up front.
fn wire_bytes<C, It, Col>(iterator: It) -> impl Iterator<Item = u8>
where
    C: Chip,
    C::Pixel: SmartLedsColor,
    <C::Pixel as SmartLedsColor>::Color: Into<C::Pixel>,
    It: IntoIterator<Item = Col>,
    Col: Into<<C::Pixel as SmartLedsColor>::Color>,
{
    iterator.into_iter().flat_map(|color| {
        let color: <C::Pixel as SmartLedsColor>::Color = color.into();
        let pixel: C::Pixel = color.into();
        wire::clockless_bytes::<C, _>([pixel])
    })
}
