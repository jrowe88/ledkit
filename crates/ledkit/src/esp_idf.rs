//! ESP32 boards with ESP-IDF (`std`): NeoPixels from the RMT peripheral, plus board
//! helpers. Turn it on with the `esp-idf` feature; the chip comes from your build target.
//!
//! Everything from [`ledkit-esp-idf`](https://docs.rs/ledkit-esp-idf) is here
//! ([`RmtOutput`], [`Error`]), plus:
//!
//! - [`strip`] and [`strip_with_dma`]: a ready-to-use [`Strip`] from a pin.
//! - [`Clock`]: the time since start-up, for animations and the frame loop.
//!
//! ```ignore
//! use esp_idf_hal::delay::Delay;
//! use esp_idf_hal::peripherals::Peripherals;
//! use ledkit::prelude::*;
//!
//! let peripherals = Peripherals::take()?;
//! let mut strip = ledkit::esp_idf::strip::<Ws2812b, 30>(peripherals.pins.gpio4)?;
//!
//! let mut frames = Frames::new(ledkit::esp_idf::Clock::start(), Delay::new_default());
//! let rainbow = Rainbow::new();
//! loop {
//!     let now = frames.next_frame();
//!     rainbow.render(&mut strip, now);
//!     strip.show()?;
//! }
//! ```
//!
//! [`strip`] is `Strip::new(RmtOutput::new(pin)?)`; write that out whenever you like.
//! The output is blocking: ESP-IDF sends the frame while your task waits, and other
//! tasks (Wi-Fi, say) keep running.

pub use ledkit_esp_idf::*;

use esp_idf_hal::gpio::OutputPin;
use ledkit_core::{Chip, Strip};

/// A ready-to-use [`Strip`] of `N` LEDs of type `C` on `pin`.
///
/// ESP-IDF picks a free RMT channel. This is `Strip::new(RmtOutput::new(pin)?)`.
///
/// # Errors
///
/// As [`RmtOutput::new`]: [`Error::Esp`] if ESP-IDF has no free channel.
///
/// # Example
///
/// ```ignore
/// let mut strip = ledkit::esp_idf::strip::<Ws2812b, 30>(peripherals.pins.gpio4)?;
/// strip.fill(Rgb::BLUE);
/// strip.show()?;
/// ```
pub fn strip<'d, C: Chip, const N: usize>(
    pin: impl OutputPin + 'd,
) -> Result<Strip<C, N, RmtOutput<'d, C>>, Error> {
    Ok(Strip::new(RmtOutput::new(pin)?))
}

/// Like [`strip`], but the RMT reads the frame through DMA, so long strips stay clean
/// however busy the processor is. Only the ESP32-S3 (and P4) have RMT DMA; elsewhere
/// this returns an error.
///
/// This is `Strip::new(RmtOutput::new_with_dma(pin)?)`.
///
/// # Errors
///
/// As [`RmtOutput::new_with_dma`].
///
/// # Example
///
/// ```ignore
/// let mut strip = ledkit::esp_idf::strip_with_dma::<Ws2812b, 300>(peripherals.pins.gpio5)?;
/// ```
pub fn strip_with_dma<'d, C: Chip, const N: usize>(
    pin: impl OutputPin + 'd,
) -> Result<Strip<C, N, RmtOutput<'d, C>>, Error> {
    Ok(Strip::new(RmtOutput::new_with_dma(pin)?))
}

/// How long the program has been running, for animations and
/// [`Frames`](ledkit_anim::Frames). It reads `std::time::Instant`.
///
/// ```ignore
/// use ledkit::prelude::*;
///
/// let clock = ledkit::esp_idf::Clock::start();
/// let now = clock.now(); // a `Time`
/// ```
#[cfg(feature = "anim")]
#[derive(Copy, Clone, Debug)]
pub struct Clock {
    start: std::time::Instant,
}

#[cfg(feature = "anim")]
impl Clock {
    /// A clock that starts at 0 now.
    ///
    /// ```ignore
    /// let clock = ledkit::esp_idf::Clock::start();
    /// ```
    pub fn start() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "anim")]
impl ledkit_anim::Clock for Clock {
    fn now(&self) -> ledkit_anim::Time {
        self.start.elapsed().into()
    }
}
