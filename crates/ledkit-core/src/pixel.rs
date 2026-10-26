//! Channels, channel orders and the [`Pixel`] trait.

use ledkit_color::{Rgb, Rgbw};

/// One color channel of an LED.
///
/// # Example
///
/// ```
/// use ledkit_core::Channel;
///
/// assert_ne!(Channel::Red, Channel::Green);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Channel {
    /// The red channel.
    Red,
    /// The green channel.
    Green,
    /// The blue channel.
    Blue,
    /// The dedicated white channel found on RGBW LEDs.
    White,
}

/// The order in which an LED chip expects its color channels on the wire.
///
/// Many NeoPixels want green first (`GRB`); others want red first (`RGB`).
///
/// # Example
///
/// ```
/// use ledkit_core::{Channel, ChannelOrder};
///
/// assert_eq!(ChannelOrder::GRB.len(), 3);
/// assert_eq!(ChannelOrder::GRB.channels()[0], Channel::Green);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChannelOrder {
    channels: [Channel; 4],
    len: u8,
}

impl ChannelOrder {
    const fn three(a: Channel, b: Channel, c: Channel) -> Self {
        Self {
            channels: [a, b, c, Channel::White],
            len: 3,
        }
    }

    const fn four(a: Channel, b: Channel, c: Channel, d: Channel) -> Self {
        Self {
            channels: [a, b, c, d],
            len: 4,
        }
    }

    /// The channels in wire order (3 or 4 of them).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{Channel, ChannelOrder};
    ///
    /// assert_eq!(
    ///     ChannelOrder::GRB.channels(),
    ///     &[Channel::Green, Channel::Red, Channel::Blue]
    /// );
    /// ```
    pub const fn channels(&self) -> &[Channel] {
        let (used, _) = self.channels.split_at(self.len as usize);
        used
    }

    /// How many channels this order has (3 or 4).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::ChannelOrder;
    ///
    /// assert_eq!(ChannelOrder::RGB.len(), 3);
    /// assert_eq!(ChannelOrder::GRBW.len(), 4);
    /// ```
    // Never empty, so an `is_empty` method would be pointless.
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    /// Red, green, blue.
    pub const RGB: Self = Self::three(Channel::Red, Channel::Green, Channel::Blue);
    /// Red, blue, green.
    pub const RBG: Self = Self::three(Channel::Red, Channel::Blue, Channel::Green);
    /// Green, red, blue (the most common NeoPixel order).
    pub const GRB: Self = Self::three(Channel::Green, Channel::Red, Channel::Blue);
    /// Green, blue, red.
    pub const GBR: Self = Self::three(Channel::Green, Channel::Blue, Channel::Red);
    /// Blue, red, green.
    pub const BRG: Self = Self::three(Channel::Blue, Channel::Red, Channel::Green);
    /// Blue, green, red.
    pub const BGR: Self = Self::three(Channel::Blue, Channel::Green, Channel::Red);
    /// Red, green, blue, white.
    pub const RGBW: Self = Self::four(Channel::Red, Channel::Green, Channel::Blue, Channel::White);
    /// Green, red, blue, white.
    pub const GRBW: Self = Self::four(Channel::Green, Channel::Red, Channel::Blue, Channel::White);
    /// White, red, green, blue.
    pub const WRGB: Self = Self::four(Channel::White, Channel::Red, Channel::Green, Channel::Blue);
    /// White, green, red, blue.
    pub const WGRB: Self = Self::four(Channel::White, Channel::Green, Channel::Red, Channel::Blue);
}

/// A color type that can be stored in a strip: [`Rgb`] or [`Rgbw`].
///
/// # Example
///
/// ```
/// use ledkit_core::{Channel, Pixel, Rgb};
///
/// let c = Rgb::new(10, 20, 30);
/// assert_eq!(c.channel(Channel::Green), 20);
/// assert_eq!(<Rgb as Pixel>::CHANNELS, 3);
/// ```
pub trait Pixel: Copy + Default + PartialEq + core::fmt::Debug + From<Rgb> {
    /// How many channels this pixel type has (3 or 4).
    const CHANNELS: usize;

    /// Reads one channel. Asking an [`Rgb`] for `White` gives 0.
    fn channel(&self, channel: Channel) -> u8;

    /// Builds a new pixel by passing every channel and its value through `f`.
    #[must_use]
    fn map_channels(self, f: impl FnMut(Channel, u8) -> u8) -> Self;
}

impl Pixel for Rgb {
    const CHANNELS: usize = 3;

    fn channel(&self, channel: Channel) -> u8 {
        match channel {
            Channel::Red => self.r,
            Channel::Green => self.g,
            Channel::Blue => self.b,
            Channel::White => 0,
        }
    }

    fn map_channels(self, mut f: impl FnMut(Channel, u8) -> u8) -> Self {
        Rgb::new(
            f(Channel::Red, self.r),
            f(Channel::Green, self.g),
            f(Channel::Blue, self.b),
        )
    }
}

impl Pixel for Rgbw {
    const CHANNELS: usize = 4;

    fn channel(&self, channel: Channel) -> u8 {
        match channel {
            Channel::Red => self.r,
            Channel::Green => self.g,
            Channel::Blue => self.b,
            Channel::White => self.w,
        }
    }

    fn map_channels(self, mut f: impl FnMut(Channel, u8) -> u8) -> Self {
        Rgbw::new(
            f(Channel::Red, self.r),
            f(Channel::Green, self.g),
            f(Channel::Blue, self.b),
            f(Channel::White, self.w),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grb_order() {
        assert_eq!(
            ChannelOrder::GRB.channels(),
            &[Channel::Green, Channel::Red, Channel::Blue]
        );
    }

    #[test]
    fn grbw_order_has_four() {
        assert_eq!(ChannelOrder::GRBW.len(), 4);
        assert_eq!(ChannelOrder::GRBW.channels()[3], Channel::White);
    }

    #[test]
    fn rgb_channels() {
        let c = Rgb::new(1, 2, 3);
        assert_eq!(
            (
                c.channel(Channel::Red),
                c.channel(Channel::Green),
                c.channel(Channel::Blue),
                c.channel(Channel::White)
            ),
            (1, 2, 3, 0)
        );
    }

    #[test]
    fn rgbw_channels() {
        assert_eq!(Rgbw::new(1, 2, 3, 4).channel(Channel::White), 4);
    }

    #[test]
    fn map_channels_sees_each_channel() {
        let c = Rgbw::new(1, 2, 3, 4)
            .map_channels(|ch, v| if ch == Channel::White { 0 } else { v * 10 });
        assert_eq!(c, Rgbw::new(10, 20, 30, 0));
    }

    #[test]
    fn channel_counts() {
        assert_eq!(<Rgb as Pixel>::CHANNELS, 3);
        assert_eq!(<Rgbw as Pixel>::CHANNELS, 4);
    }
}
