//! ESP32 boards with esp-hal (bare metal, `no_std`): NeoPixels from the RMT peripheral,
//! blocking or async, plus board helpers. Turn it on with your chip's feature
//! (`esp32c6`, `esp32s3`, ...).
//!
//! Everything from [`ledkit-esp`](https://docs.rs/ledkit-esp) is here
//! ([`RmtOutput`], [`buffer_len`], [`Error`]), plus:
//!
//! - [`strip!`]: a ready-to-use [`Strip`] from the RMT and a pin, in one line.
//! - [`strip_on`] and [`pulse_buffer!`]: the same, on an RMT channel you set up
//!   yourself, for when you use the RMT's other channels too.
//! - [`Clock`]: the time since start-up, for animations and the frame loop.
//!
//! ## The short way
//!
//! ```ignore
//! use esp_hal::delay::Delay;
//! use ledkit::prelude::*;
//!
//! let peripherals = esp_hal::init(esp_hal::Config::default());
//! let mut strip = ledkit::esp::strip!(Ws2812b, 30, peripherals.RMT, peripherals.GPIO4)?;
//! strip.set_brightness(Brightness::from_percent(25));
//!
//! let mut frames = Frames::new(ledkit::esp::Clock::start(), Delay::new());
//! let rainbow = Rainbow::new();
//! loop {
//!     let now = frames.next_frame();
//!     rainbow.render(&mut strip, now);
//!     strip.show()?;
//! }
//! ```
//!
//! ## The same, by hand
//!
//! [`strip!`] does exactly this; write it out when you want to change a step:
//!
//! ```ignore
//! use esp_hal::rmt::{PulseCode, Rmt};
//! use esp_hal::time::Rate;
//! use ledkit::esp::{buffer_len, RmtOutput};
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80))?;
//! let rmt_hz = rmt.frequency().as_hz();
//! let output = RmtOutput::<Ws2812b, _, _>::new(
//!     rmt.channel0,
//!     peripherals.GPIO4,
//!     rmt_hz,
//!     [PulseCode::default(); buffer_len::<Ws2812b>(30)],
//! )?;
//! let mut strip = Strip::<Ws2812b, 30, _>::new(output);
//! ```
//!
//! ## Async
//!
//! Add `async` as the last argument of [`strip!`] for an output you `.await`:
//!
//! ```ignore
//! let mut strip = ledkit::esp::strip!(Ws2812b, 30, peripherals.RMT, peripherals.GPIO4, async)?;
//! strip.show_async().await?;
//! ```
//!
//! Wait between frames with [`FramesAsync`](ledkit_anim::FramesAsync) and any async
//! delay: an esp-hal `OneShotTimer` made async, or `embassy_time::Delay` under Embassy.

pub use ledkit_esp::*;

use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::rmt::TxChannelCreator;
use esp_hal::DriverMode;
use ledkit_core::{Chip, Strip};

/// The RMT clock [`strip!`] sets up: 80 MHz, which ledkit divides to its 10 MHz ticks.
///
/// ```ignore
/// let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(ledkit::esp::RMT_HZ))?;
/// ```
pub const RMT_HZ: u32 = 80_000_000;

/// A [`Strip`] on an RMT channel you set up yourself.
///
/// This is [`RmtOutput::new`] and [`Strip::new`] in one call. Use it instead of
/// [`strip!`] when the RMT drives other things too (more strips on its other channels,
/// or an IR remote), so you keep the `Rmt` and pick the channel. Make the `buffer` with
/// [`pulse_buffer!`].
///
/// # Errors
///
/// As [`RmtOutput::new`]: [`Error::ClockNotUsable`] if `rmt_hz` can't be divided to
/// 10 MHz, and [`Error::Config`] if esp-hal refuses the channel.
///
/// # Example
///
/// ```ignore
/// let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(ledkit::esp::RMT_HZ))?;
/// let rmt_hz = rmt.frequency().as_hz();
/// let mut strip = ledkit::esp::strip_on::<Ws2812b, 30, _, _>(
///     rmt.channel1,
///     peripherals.GPIO5,
///     rmt_hz,
///     ledkit::esp::pulse_buffer!(Ws2812b, 30),
/// )?;
/// ```
pub fn strip_on<'ch, C: Chip, const N: usize, Dm: DriverMode, B>(
    channel: impl TxChannelCreator<'ch, Dm>,
    pin: impl PeripheralOutput<'ch>,
    rmt_hz: u32,
    buffer: B,
) -> Result<Strip<C, N, RmtOutput<'ch, C, Dm, B>>, Error> {
    Ok(Strip::new(RmtOutput::new(channel, pin, rmt_hz, buffer)?))
}

/// How long the program has been running, for animations and
/// [`Frames`](ledkit_anim::Frames).
///
/// It reads esp-hal's system timer, so it needs no peripheral of its own. Start it once,
/// near the top of `main`.
///
/// ```ignore
/// use ledkit::prelude::*;
///
/// let clock = ledkit::esp::Clock::start();
/// // ...
/// let now = clock.now(); // a `Time`
/// ```
#[cfg(feature = "anim")]
#[derive(Copy, Clone, Debug)]
pub struct Clock {
    start: esp_hal::time::Instant,
}

#[cfg(feature = "anim")]
impl Clock {
    /// A clock that starts at 0 now.
    ///
    /// ```ignore
    /// let clock = ledkit::esp::Clock::start();
    /// ```
    pub fn start() -> Self {
        Self {
            start: esp_hal::time::Instant::now(),
        }
    }
}

#[cfg(feature = "anim")]
impl ledkit_anim::Clock for Clock {
    fn now(&self) -> ledkit_anim::Time {
        // Wraps after about 49 days, like `Time` itself.
        ledkit_anim::Time::from_millis(self.start.elapsed().as_millis() as u32)
    }
}

/// A ready-to-use [`Strip`] from the RMT peripheral and a pin, in one line.
///
/// `strip!(LedType, LED_COUNT, peripherals.RMT, pin)` sets the RMT up at [`RMT_HZ`],
/// takes its channel 0, sizes the frame buffer for `LED_COUNT` LEDs, and gives back
/// `Result<Strip<LedType, LED_COUNT, _>, ledkit::esp::Error>`. `LED_COUNT` must be a
/// constant. Add `async` as a last argument for an async output
/// (`strip.show_async().await`).
///
/// It's a macro, not a function, because the buffer's length is worked out from
/// `LED_COUNT` at compile time, which a generic function can't do on stable Rust.
///
/// The buffer is part of the strip, so it sits on the stack: about 100 bytes per
/// LED. For hundreds of LEDs, use [`strip_on`] with a `&'static mut` buffer instead.
///
/// The [module docs](self) show what it does, written out by hand.
///
/// # Example
///
/// ```ignore
/// use ledkit::prelude::*;
///
/// let peripherals = esp_hal::init(esp_hal::Config::default());
/// let mut strip = ledkit::esp::strip!(Ws2812b, 30, peripherals.RMT, peripherals.GPIO4)?;
/// strip.fill(Rgb::BLUE);
/// strip.show()?;
/// ```
#[doc(inline)]
pub use crate::__ledkit_esp_strip as strip;

/// A frame buffer for `LED_COUNT` LEDs of a type, for [`strip_on`] (or for
/// [`RmtOutput::new`]).
///
/// `pulse_buffer!(LedType, LED_COUNT)` is
/// `[PulseCode::default(); buffer_len::<LedType>(LED_COUNT)]`.
///
/// # Example
///
/// ```ignore
/// let buffer = ledkit::esp::pulse_buffer!(Ws2812b, 30);
/// ```
#[doc(inline)]
pub use crate::__ledkit_esp_pulse_buffer as pulse_buffer;

/// Used by the macros; not part of the public API.
#[doc(hidden)]
pub mod __private {
    pub use esp_hal::rmt::PulseCode;

    use super::{strip_on, Error, RmtOutput, RMT_HZ};
    use esp_hal::gpio::interconnect::PeripheralOutput;
    use esp_hal::peripherals::RMT;
    use esp_hal::rmt::Rmt;
    use esp_hal::time::Rate;
    use esp_hal::{Async, Blocking};
    use ledkit_core::{Chip, Strip};

    /// `strip!` without `async`.
    pub fn blocking_strip<'d, C: Chip, const N: usize, B>(
        rmt: RMT<'d>,
        pin: impl PeripheralOutput<'d>,
        buffer: B,
    ) -> Result<Strip<C, N, RmtOutput<'d, C, Blocking, B>>, Error> {
        let rmt = Rmt::new(rmt, Rate::from_hz(RMT_HZ))?;
        let rmt_hz = rmt.frequency().as_hz();
        strip_on(rmt.channel0, pin, rmt_hz, buffer)
    }

    /// `strip!` with `async`.
    pub fn async_strip<'d, C: Chip, const N: usize, B>(
        rmt: RMT<'d>,
        pin: impl PeripheralOutput<'d>,
        buffer: B,
    ) -> Result<Strip<C, N, RmtOutput<'d, C, Async, B>>, Error> {
        let rmt = Rmt::new(rmt, Rate::from_hz(RMT_HZ))?.into_async();
        let rmt_hz = rmt.frequency().as_hz();
        strip_on(rmt.channel0, pin, rmt_hz, buffer)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ledkit_esp_strip {
    ($led:ty, $count:expr, $rmt:expr, $pin:expr, async $(,)?) => {
        $crate::esp::__private::async_strip::<$led, { $count }, _>(
            $rmt,
            $pin,
            $crate::esp::pulse_buffer!($led, $count),
        )
    };
    ($led:ty, $count:expr, $rmt:expr, $pin:expr $(,)?) => {
        $crate::esp::__private::blocking_strip::<$led, { $count }, _>(
            $rmt,
            $pin,
            $crate::esp::pulse_buffer!($led, $count),
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __ledkit_esp_pulse_buffer {
    ($led:ty, $count:expr $(,)?) => {
        [$crate::esp::__private::PulseCode::default(); $crate::esp::buffer_len::<$led>($count)]
    };
}
