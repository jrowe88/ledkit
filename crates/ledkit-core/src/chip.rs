//! The [`Chip`] trait.

use crate::{ChannelOrder, Pixel, PowerModel, Protocol};

/// Describes one kind of LED chip: its pixel type, wire order and signal.
///
/// The ready-made chips are listed in the crate root; you rarely implement
/// this yourself unless your LED is not listed.
///
/// # Example
///
/// ```
/// use ledkit_core::{ChannelOrder, Chip, Ws2812b};
///
/// assert_eq!(Ws2812b::NAME, "WS2812B");
/// assert_eq!(Ws2812b::ORDER, ChannelOrder::GRB);
/// ```
pub trait Chip {
    /// The color type this chip stores per LED.
    type Pixel: Pixel;
    /// The chip's name, as printed on its datasheet.
    const NAME: &'static str;
    /// The order the chip expects its color channels in.
    const ORDER: ChannelOrder;
    /// How the chip is talked to.
    const PROTOCOL: Protocol;
    /// How much current the chip draws. Defaults to a typical 5 V LED.
    const POWER: PowerModel = PowerModel::TYPICAL_5V;
}
