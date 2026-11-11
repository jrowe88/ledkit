#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "esp-idf"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "_esp")]
pub mod esp;
#[cfg(feature = "esp-idf")]
pub mod esp_idf;

/// Colors: [`Rgb`](color::Rgb), [`Rgbw`](color::Rgbw), hues, gradients, gamma and
/// palettes. The [`ledkit-color`](https://docs.rs/ledkit-color) crate.
///
/// ```
/// use ledkit::color::Rgb;
///
/// let orange = Rgb::new(255, 128, 0);
/// ```
pub use ledkit_color as color;

/// The LED chip catalogue, the [`Strip`](core::Strip) buffer, brightness, dithering and
/// the power limit, and the output traits backends implement. The
/// [`ledkit-core`](https://docs.rs/ledkit-core) crate.
///
/// ```
/// use ledkit::core::{Milliamps, Ws2812b};
///
/// let limit = Milliamps(500);
/// ```
pub use ledkit_core as core;

/// Matrices, segments and [`Canvas`](layout::Canvas): (x, y) positions on a panel. The
/// [`ledkit-layout`](https://docs.rs/ledkit-layout) crate.
///
/// ```
/// use ledkit::layout::{Corner, MatrixLayout};
///
/// const PANEL: MatrixLayout = MatrixLayout::new(8, 8).starting_at(Corner::BottomLeft);
/// ```
#[cfg(feature = "layout")]
pub use ledkit_layout as layout;

/// Effects, the ways to combine them, and the frame loop
/// ([`Frames`](anim::Frames)). The [`ledkit-anim`](https://docs.rs/ledkit-anim) crate.
///
/// ```
/// use ledkit::anim::Rainbow;
///
/// let rainbow = Rainbow::new();
/// ```
#[cfg(feature = "anim")]
pub use ledkit_anim as anim;

/// Fonts, text areas and a scrolling ticker. The
/// [`ledkit-text`](https://docs.rs/ledkit-text) crate.
///
/// ```
/// use ledkit::text::{Font, Ticker};
///
/// let mut ticker = Ticker::new(Font::X11_5X8);
/// ticker.set_messages(&["Hello"]);
/// ```
#[cfg(feature = "text")]
pub use ledkit_text as text;

/// NeoPixels and DotStars from any `embedded-hal` 1.0 SPI bus, blocking or async. The
/// [`ledkit-spi`](https://docs.rs/ledkit-spi) crate. Feature `spi`.
///
/// ```ignore
/// // `spi` is your board's SPI bus, set up as usual.
/// let mut strip = Strip::<Apa102, 64, _>::new(ledkit::spi::ClockedSpi::new(spi));
/// ```
#[cfg(feature = "spi")]
pub use ledkit_spi as spi;

/// NeoPixels from any `embedded-io` serial port with an inverted output. The
/// [`ledkit-uart`](https://docs.rs/ledkit-uart) crate. Feature `uart`.
///
/// ```ignore
/// // `uart` is your board's serial port, set up at `BAUD` with an inverted output.
/// let output = ledkit::uart::ClocklessUart::<Ws2812b, _, _, _>::new(
///     uart, BAUD, delay, [0u8; ledkit::uart::buffer_len::<Ws2812b>(30, BAUD)])?;
/// ```
#[cfg(feature = "uart")]
pub use ledkit_uart as uart;

/// nRF52840 PWM + EasyDMA output, blocking or async. The
/// [`ledkit-nrf`](https://docs.rs/ledkit-nrf) crate. Feature `nrf52840`.
///
/// ```ignore
/// // See ledkit-nrf's docs for setting up the PWM.
/// let output = ledkit::nrf::PwmOutput::<Ws2812b, _>::new(/* .. */);
/// ```
#[cfg(feature = "nrf52840")]
pub use ledkit_nrf as nrf;

/// Everything a typical program needs, in one `use`.
///
/// The chip types (`Ws2812b`, `Apa102`, ...), [`Strip`](crate::core::Strip), colors,
/// brightness and the power limit; with the default features also the layout types,
/// the effects, the frame loop and the text types. Backend outputs (`RmtOutput`,
/// `ClockedSpi`, ...) are not in it: name your backend once, at setup.
///
/// ```
/// use ledkit::prelude::*;
///
/// let color = Rgb::new(0, 0, 255);
/// let brightness = Brightness::from_percent(25);
/// let panel = MatrixLayout::new(8, 8).starting_at(Corner::BottomLeft);
/// let rainbow = Rainbow::new();
/// let font = Font::X11_5X8;
/// ```
pub mod prelude {
    pub use ledkit_core::prelude::*;
    pub use ledkit_core::Pixel;

    #[cfg(feature = "layout")]
    pub use ledkit_layout::{
        Canvas, Corner, Layout, MatrixLayout, Order, Rotation, Segment, SegmentMut, Wiring,
    };

    #[cfg(feature = "anim")]
    pub use ledkit_anim::prelude::*;

    #[cfg(feature = "text")]
    pub use ledkit_text::prelude::*;
}
