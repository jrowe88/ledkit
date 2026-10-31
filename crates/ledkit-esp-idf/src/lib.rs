//! A ledkit backend that drives NeoPixels with the ESP32's RMT peripheral, through
//! `esp-idf-hal` (the `std` flavour of Rust on ESP32, running on top of ESP-IDF).
//!
//! These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
//! precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
//! and the pulse widths tell `0` from `1`. The RMT (Remote Control Transceiver)
//! peripheral generates these pulses in hardware, so the timing is unaffected by Wi-Fi
//! interrupts or task scheduling.
//!
//! `ledkit-esp-idf` is for projects built on ESP-IDF. For bare-metal projects with
//! `esp-hal`, use `ledkit-esp` instead.
//!
//! ## Which ESP crate do I use?
//!
//! | Your project | Use |
//! |---|---|
//! | esp-hal: bare metal, `no_std`, stable Rust, works with Embassy | `ledkit-esp` |
//! | ESP-IDF: `std`, Wi-Fi through `esp-idf-svc`, nightly Rust | `ledkit-esp-idf` |
//! | Any board with an SPI bus (including ESP32 boards, no RMT needed) | `ledkit-spi` |
//!
//! ## Quick tour
//!
//! This can only run on an ESP32, so the snippet is not run by `cargo test`:
//!
//! ```ignore
//! use esp_idf_hal::peripherals::Peripherals;
//! use ledkit_core::prelude::*;
//! use ledkit_esp_idf::RmtOutput;
//!
//! const LED_COUNT: usize = 8;
//!
//! esp_idf_hal::sys::link_patches();
//! let peripherals = Peripherals::take().unwrap();
//!
//! // The C6 DevKit's onboard LED is on GPIO8; use your strip's data pin.
//! let output = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8).unwrap();
//! let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
//! strip.fill(Rgb::BLUE);
//! strip.show().unwrap();
//! ```
//!
//! ## Wiring
//!
//! One data wire from a GPIO pin to the LED's data-in, a shared ground, and 5 V power
//! for the LEDs. The board's 3.3 V signal is fine for most WS2812B strips; if the first
//! LED flickers, add a level shifter. On the ESP32-C6-DevKitC-1 the onboard LED is on
//! GPIO8.
//!
//! [`RmtOutput::new`] takes the pin and asks ESP-IDF for any free RMT transmit channel,
//! so you never pick a channel number. You only choose the LED type and the pin.
//!
//! ## The reset gap
//!
//! The LEDs latch the new colors once the data line has been idle for the chip's reset
//! time (280 us for the WS2812B). [`RmtOutput`] remembers when the last frame ended and,
//! if `show()` is called again too soon, waits out the rest of the gap before sending.
//! Two `show()` calls back to back therefore both take effect, instead of the second
//! frame being glued onto the first and showing the wrong colors. The wait uses
//! `std::thread::sleep`. Waits shorter than one FreeRTOS tick are busy-waited
//! precisely; only a chip with a reset of a tick or more would yield to the
//! scheduler, which is still safe.
//!
//! ## Coming from `smart-leds`?
//!
//! With the `smart-leds` feature, [`RmtOutput`] implements `smart-leds`'
//! `SmartLedsWrite`, so code written for `ws2812-esp32-rmt-driver` needs only a new
//! constructor. Colors are sent exactly as given: no brightness, gamma or dithering is
//! added, so keep using `smart-leds`' own `brightness()` and `gamma()` adaptors.
//!
//! ```ignore
//! use ledkit_core::Ws2812b;
//! use ledkit_esp_idf::RmtOutput;
//! use smart_leds_trait::{SmartLedsWrite, RGB8};
//!
//! let mut ws = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8).unwrap();
//! ws.write([RGB8::new(255, 0, 0); 8]).unwrap();
//! ```
//!
//! ## Building
//!
//! This crate only builds for ESP-IDF targets (for example `riscv32imac-esp-espidf`), with
//! the setup `esp-idf-sys` needs: a nightly Rust with `rust-src`, `ldproxy`, and a
//! `.cargo/config.toml` that sets the target, `build-std` and `ESP_IDF_VERSION`.
//! Copy the setup of `examples/esp32c6-esp-idf`; its README walks through it.
//! On Windows, keep the build folder short (the example's `.cargo/config.toml` sets
//! `target-dir = "C:/t/ledkit"`), because `esp-idf-sys` rejects output paths over 88 characters.
//!
//! It depends on the published `esp-idf-hal` 0.47 from crates.io and needs no
//! `[patch.crates-io]` section. ESP-IDF projects that already depend on `esp-idf-svc` can
//! use it as is, as long as both resolve to the same `esp-idf-hal`.
//!
//! ## Tested on hardware
//!
//! | Board | LEDs | Result |
//! |---|---|---|
//! | ESP32-C6 dev kit (chip rev v0.2), 2026-10-30 | the onboard LED | pass: colors in order, back-to-back frames latch |
//! | ESP32-C6 dev kit (chip rev v0.2), 2026-10-30 | the onboard LED, slow rainbow | flickered with dithering on (20% brightness, about 45 frames a second), steady with it off; the example now turns it off, see its troubleshooting |
//! | ESP32-C6, the pulse project (first consumer) | its status LED | pass (2026-10-30): breathes as before after pulse raised its own minimum brightness |
//!
//! The example is `examples/esp32c6-esp-idf` in the repository. If you try it on your
//! hardware, please report what you see.
//!
//! ## Features
//!
//! - `smart-leds`: the `smart-leds` write trait for [`RmtOutput`].

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use esp_idf_hal::gpio::OutputPin;
use esp_idf_hal::rmt::config::{TransmitConfig, TxChannelConfig};
use esp_idf_hal::rmt::encoder::{BytesEncoder, BytesEncoderConfig};
use esp_idf_hal::rmt::{PinState, Pulse, PulseTicks, Symbol, TxChannelDriver};
use esp_idf_hal::sys::EspError;
use esp_idf_hal::units::Hertz;
use ledkit_core::encode::BitPulse;
use ledkit_core::{wire, Chip, ClocklessTiming, Output, ResetGap};

mod error;
#[cfg(feature = "smart-leds")]
mod smart_leds;

pub use error::Error;

/// RMT tick rate: 10 MHz, so one tick is 100 ns.
const RESOLUTION: Hertz = Hertz(10_000_000);

/// The longest single RMT pulse, in ticks.
const MAX_TICKS: u32 = 32_767;

/// A chain of clockless LEDs of type `C` (for example [`ledkit_core::Ws2812b`]) on one
/// GPIO pin.
///
/// Hand it to [`Strip::new`](ledkit_core::Strip::new); the strip calls
/// [`Output::write`] for you on every `show()`.
///
/// # Example
///
/// ```ignore
/// use ledkit_core::prelude::*;
/// use ledkit_esp_idf::RmtOutput;
///
/// // GPIO8 is the onboard LED of the ESP32-C6-DevKitC-1.
/// let output = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8)?;
/// let mut strip = Strip::<Ws2812b, 1, _>::new(output);
/// strip.fill(Rgb::GREEN);
/// strip.show()?;
/// ```
pub struct RmtOutput<'d, C: Chip> {
    channel: TxChannelDriver<'d>,
    encoder: BytesEncoder,
    tx_config: TransmitConfig,
    /// The frame as bytes, reused between frames so `show()` doesn't allocate.
    bytes: Vec<u8>,
    /// How long the data line must stay idle between frames.
    gap: ResetGap,
    /// When the last frame finished, if there was one.
    last_frame_end: Option<Instant>,
    chip: PhantomData<C>,
}

// The driver handles (channel, encoder, transmit config) have nothing useful to
// print, so only the state a beginner can reason about is shown.
impl<C: Chip> std::fmt::Debug for RmtOutput<'_, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RmtOutput")
            .field("gap", &self.gap)
            .field("last_frame_end", &self.last_frame_end)
            .field("frame_bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl<'d, C: Chip> RmtOutput<'d, C> {
    /// Claims an RMT transmit channel on `pin` and configures it for `C`'s timing.
    ///
    /// ESP-IDF picks the channel. Using a clocked chip (a DotStar, which needs a clock
    /// wire) is a compile error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Esp`] if ESP-IDF can't give out a channel (for example, they are
    /// all taken), and [`Error::Timing`] if the chip's pulses don't fit the RMT.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8)?;
    /// ```
    pub fn new(pin: impl OutputPin + 'd) -> Result<Self, Error> {
        // Evaluated at compile time: a clocked chip can't be driven this way, and using
        // one here fails the build.
        let timing = const { ClocklessTiming::of::<C>() };
        let gap = const { ResetGap::of::<C>() };
        // ledkit checks each pulse against the chip's datasheet window at this tick rate.
        let ticks = timing.pulse_ticks(RESOLUTION.0, MAX_TICKS)?;

        let channel = TxChannelDriver::new(
            pin,
            &TxChannelConfig {
                resolution: RESOLUTION,
                ..Default::default()
            },
        )?;

        // Turns each byte into 8 RMT symbols, most significant bit first.
        let encoder = BytesEncoder::with_config(&BytesEncoderConfig {
            bit0: symbol(ticks.zero_pulse())?,
            bit1: symbol(ticks.one_pulse())?,
            msb_first: true,
            ..Default::default()
        })?;

        Ok(Self {
            channel,
            encoder,
            tx_config: TransmitConfig::default(),
            bytes: Vec::new(),
            gap,
            last_frame_end: None,
            chip: PhantomData,
        })
    }

    /// Sends the given wire bytes as one frame, after waiting out the reset gap of
    /// the previous frame.
    pub(crate) fn send_bytes(&mut self, bytes: impl Iterator<Item = u8>) -> Result<(), EspError> {
        self.bytes.clear();
        self.bytes.extend(bytes);

        // Zero LEDs: nothing to send, and the RMT driver rejects an empty signal.
        // The line stays low, and `last_frame_end` is left alone, so the next real
        // frame still waits out the gap after the previous one.
        if self.bytes.is_empty() {
            return Ok(());
        }

        if let Some(end) = self.last_frame_end {
            let elapsed_us = u64::try_from(end.elapsed().as_micros()).unwrap_or(u64::MAX);
            let remaining_us = self.gap.remaining_us(elapsed_us);
            if remaining_us > 0 {
                std::thread::sleep(Duration::from_micros(u64::from(remaining_us)));
            }
        }

        let result = self
            .channel
            .send_and_wait(&mut self.encoder, &self.bytes, &self.tx_config);
        // Deliberately set even if the send failed: a failed send may have partly driven
        // the line, so the next frame must wait the full gap.
        self.last_frame_end = Some(Instant::now());
        result
    }
}

impl<C: Chip> Output<C> for RmtOutput<'_, C> {
    type Error = EspError;

    /// Sends one frame and blocks until the transmission completes (~30 us per LED).
    ///
    /// If the previous frame ended less than the chip's reset time ago, this first
    /// waits out the difference, so the LEDs latch each frame separately.
    fn write<I>(&mut self, pixels: I) -> Result<(), EspError>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        // ledkit puts each pixel's channels in the chip's order (green, red, blue for the
        // WS2812B).
        self.send_bytes(wire::clockless_bytes::<C, _>(pixels))
    }
}

/// One bit's RMT symbol: high for `pulse.high` ticks, then low for `pulse.low` ticks.
fn symbol(pulse: BitPulse) -> Result<Symbol, EspError> {
    let half = |state, ticks: u16| -> Result<Pulse, EspError> {
        Ok(Pulse::new(state, PulseTicks::new(ticks)?))
    };
    Ok(Symbol::new(
        half(PinState::High, pulse.high)?,
        half(PinState::Low, pulse.low)?,
    ))
}
