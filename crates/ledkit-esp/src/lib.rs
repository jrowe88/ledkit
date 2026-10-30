//! A ledkit backend that drives NeoPixels with the ESP32's RMT peripheral, through
//! `esp-hal` (bare metal, `no_std`), blocking or async.
//!
//! These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
//! precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
//! and the pulse widths tell `0` from `1`. The RMT (Remote Control Transceiver)
//! peripheral generates these pulses in hardware, so the timing is unaffected by
//! interrupts.
//!
//! `ledkit-esp` is for bare-metal projects built on `esp-hal`. For projects built on
//! ESP-IDF, use `ledkit-esp-idf` instead.
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
//! This can only run on an ESP32, so the snippet is not run by `cargo test`. It is
//! written for the ESP32-C6 (pick your chip with a Cargo feature, see "Features"):
//!
//! ```ignore
//! use esp_hal::rmt::{PulseCode, Rmt};
//! use esp_hal::time::Rate;
//! use ledkit_core::prelude::*;
//! use ledkit_esp::{buffer_len, RmtOutput};
//!
//! const LED_COUNT: usize = 8;
//!
//! let peripherals = esp_hal::init(esp_hal::Config::default());
//!
//! let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
//! // The RMT's clock. Pass it to `RmtOutput::new` below.
//! let rmt_hz = rmt.frequency().as_hz();
//!
//! let output = RmtOutput::<Ws2812b, _, _>::new(
//!     rmt.channel0,
//!     // The C6 DevKit's onboard LED is on GPIO8; use your strip's data pin.
//!     peripherals.GPIO8,
//!     rmt_hz,
//!     [PulseCode::default(); buffer_len::<Ws2812b>(LED_COUNT)],
//! )
//! .unwrap();
//! let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
//! strip.fill(Rgb::BLUE);
//! strip.show().unwrap();
//! ```
//!
//! For async, create the driver with `Rmt::new(..).unwrap().into_async()` and use
//! `strip.show_async().await` (the `OutputAsync` side of ledkit).
//!
//! ## Pass the frequency you gave `Rmt::new`
//!
//! **ledkit never sets up the RMT.** You create it with `Rmt::new` and a clock rate, and
//! the channel you hand over doesn't tell ledkit that rate. So pass it to
//! [`RmtOutput::new`] as `rmt_hz`; the simplest way is `rmt.frequency().as_hz()`, which
//! is the rate `esp-hal` really set up. ledkit divides it down to 10 MHz ticks (100 ns
//! each), which is why it must be 10 MHz times a whole number from 1 to 255, such as
//! 80 MHz (divider 8). Any other value is refused with [`Error::ClockNotUsable`]
//! instead of sending slightly wrong pulses.
//!
//! ESP32-H2 isn't supported yet. Its RMT runs from a 32 MHz clock, which can't make
//! ledkit's 10 MHz tick.
//!
//! ## Which channel?
//!
//! Any transmit channel works, but the number of them differs by chip, and the
//! channel you hand over must be a *transmit* one (`esp-hal` checks this at compile
//! time):
//!
//! | Chip | Transmit channels |
//! |---|---|
//! | ESP32 | 0 to 7 |
//! | ESP32-S2 | 0 to 3 |
//! | ESP32-S3 | 0 to 3 |
//! | ESP32-C3, C6 | 0 to 1 |
//!
//! The rest are for receiving. See `esp-hal`'s `rmt` documentation for your chip.
//!
//! ## Wiring
//!
//! One data wire from a GPIO pin to the LED's data-in, a shared ground, and 5 V power
//! for the LEDs. The board's 3.3 V signal is fine for most WS2812B strips; if the first
//! LED flickers, add a level shifter. On the ESP32-C6-DevKitC-1 the onboard LED is on
//! GPIO8.
//!
//! ## Sizing the buffer
//!
//! The driver builds the whole frame as RMT pulse codes first: one per color bit, plus
//! two (the reset gap and an end marker). Make the buffer exactly big enough with
//! [`buffer_len`], which is a `const fn`, so it sizes an array:
//!
//! ```ignore
//! use ledkit_core::Ws2812b;
//! use ledkit_esp::buffer_len;
//!
//! // 30 LEDs * 3 colors * 8 bits + 2 = 722 pulse codes
//! const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
//! let buffer = [PulseCode::default(); BUFFER_LEN];
//! ```
//!
//! The pulse codes are 4 bytes each, so 30 WS2812B LEDs take about 2.9 kB of RAM.
//!
//! ## The reset gap
//!
//! The LEDs latch the new colors once the data line has been low for the chip's reset
//! time (280 us for the WS2812B). ledkit appends that gap to every frame as a long low
//! pulse, and sending only finishes when the RMT has sent it. Two `show()` calls back
//! to back therefore both take effect. Between frames the output is held low.
//!
//! ## Coming from `smart-leds`?
//!
//! With the `smart-leds` feature, [`RmtOutput`] implements `smart-leds`'
//! `SmartLedsWrite` (blocking) and `SmartLedsWriteAsync` (async), so code written for
//! `esp-hal-smartled` needs only a new constructor. Colors are sent exactly as given:
//! no brightness, gamma or dithering is added, so keep using `smart-leds`' own
//! `brightness()` and `gamma()` adaptors.
//!
//! ```ignore
//! use ledkit_core::Ws2812b;
//! use ledkit_esp::{buffer_len, RmtOutput};
//! use smart_leds_trait::{SmartLedsWrite, RGB8};
//!
//! let mut ws = RmtOutput::<Ws2812b, _, _>::new(
//!     rmt.channel0,
//!     peripherals.GPIO8,
//!     rmt_hz,
//!     [PulseCode::default(); buffer_len::<Ws2812b>(8)],
//! )
//! .unwrap();
//! ws.write([RGB8::new(255, 0, 0); 8]).unwrap();
//! ```
//!
//! ## Building
//!
//! This crate drives `esp-hal`'s RMT, which `esp-hal` still marks as unstable, so the
//! crate turns on `esp-hal`'s `unstable` feature for you. You pick the chip with one
//! Cargo feature (see below) and your own project provides the rest of the `esp-hal`
//! setup: target, linker script and the `rt` feature. Copy it from the
//! `examples/esp32c6-spi` project of the repository, which uses the same `esp-hal`.
//!
//! The ESP32-C3 and C6 are RISC-V chips and build with plain stable Rust. The
//! ESP32, S2 and S3 use Xtensa cores and need the `esp` toolchain (installed by
//! `espup`).
//!
//! ## Tested on hardware
//!
//! | Board | LEDs | Result |
//! |---|---|---|
//! | ESP32-C6 dev kit | the onboard LED (`examples/esp32c6-rmt`) | pending |
//!
//! Everything else is covered by host tests of the shared pulse encoder in
//! `ledkit-core` (`fill_rmt_words`). If you try it on your hardware, please report what
//! you see.
//!
//! ## Features
//!
//! - `esp32`, `esp32c3`, `esp32c6`, `esp32s2`, `esp32s3`: the chip. Pick
//!   exactly one; `esp-hal` refuses to build with none or two.
//! - `smart-leds`: the `smart-leds` write traits for [`RmtOutput`].
#![no_std]

use core::marker::PhantomData;

use esp_hal::gpio::interconnect::PeripheralOutput;
use esp_hal::gpio::Level;
use esp_hal::rmt::{Channel, PulseCode, Tx, TxChannelConfig, TxChannelCreator};
use esp_hal::{Async, Blocking, DriverMode};
use ledkit_core::encode::{fill_rmt_words, rmt_clock_divider, rmt_word_count, PulseTicks};
use ledkit_core::{wire, Chip, ClocklessTiming, Output, OutputAsync};

mod error;
#[cfg(feature = "smart-leds")]
mod smart_leds;

pub use error::Error;

/// The tick rate ledkit uses inside the RMT: 10 MHz, so one tick is 100 ns.
const TICK_HZ: u32 = 10_000_000;

/// The longest single RMT pulse half, in ticks (15 bits).
const MAX_TICKS: u32 = 32_767;

/// How many buffer entries [`RmtOutput`] needs for `led_count` LEDs: one pulse code per
/// color bit, plus the reset gap and the end marker.
///
/// This is a `const fn`, so you can size an array with it. Using a clocked chip (a
/// DotStar, which needs a clock wire) panics, which inside a `const` is a build error.
///
/// # Panics
///
/// Panics if `C` is a clocked chip such as the APA102.
///
/// # Example
///
/// ```ignore
/// use ledkit_core::Ws2812b;
/// use ledkit_esp::buffer_len;
///
/// const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
/// let buffer = [PulseCode::default(); BUFFER_LEN];
/// assert_eq!(buffer.len(), 722);
/// ```
#[must_use]
pub const fn buffer_len<C: Chip>(led_count: usize) -> usize {
    // Clocked chips have no pulse timing; this fails the build for them.
    let _clockless = const { ClocklessTiming::of::<C>() };
    rmt_word_count(led_count * C::ORDER.len())
}

/// A chain of clockless LEDs of type `C` (for example [`ledkit_core::Ws2812b`]) on one
/// GPIO pin, driven by one RMT transmit channel.
///
/// - `C` is your LED type.
/// - `Dm` is the `esp-hal` driver mode of the RMT: [`Blocking`] or [`Async`]. It follows
///   from the channel you pass to [`new`](Self::new). A blocking output implements
///   [`Output`] (and `SmartLedsWrite`); an async one implements [`OutputAsync`] (and
///   `SmartLedsWriteAsync`).
/// - `B` is the buffer the frame is built in, for example `[PulseCode; N]`. Use
///   [`buffer_len`] to find the size you need.
///
/// **ledkit never sets up the RMT.** You create it with `Rmt::new` and pass a channel and
/// a pin, together with the clock rate you gave `Rmt::new`.
///
/// You usually don't call `write` yourself. Put the driver in a
/// [`Strip`](ledkit_core::Strip) and call `strip.show()` (or `show_async()`).
///
/// # DotStars
///
/// DotStars (APA102 and SK9822) have their own clock wire and don't need this. Using
/// one here is a build error.
///
/// ```ignore
/// // Does not compile: DotStars need a clocked backend such as ledkit-spi.
/// // (This is `ignore` because ledkit-esp only builds for an ESP32 target, so a
/// // `compile_fail` doctest would "fail" for the wrong reason on the host.)
/// let _ = RmtOutput::<Apa102, _, _>::new(rmt.channel0, peripherals.GPIO8, 80_000_000, [PulseCode::default(); 64]);
/// ```
///
/// # Example
///
/// ```ignore
/// use esp_hal::rmt::{PulseCode, Rmt};
/// use esp_hal::time::Rate;
/// use ledkit_core::prelude::*;
/// use ledkit_esp::{buffer_len, RmtOutput};
///
/// const LED_COUNT: usize = 8;
/// let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
/// let rmt_hz = rmt.frequency().as_hz();
///
/// let output = RmtOutput::<Ws2812b, _, _>::new(
///     rmt.channel0,
///     peripherals.GPIO8,
///     rmt_hz,
///     [PulseCode::default(); buffer_len::<Ws2812b>(LED_COUNT)],
/// )
/// .unwrap();
/// let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
/// strip.fill(Rgb::GREEN);
/// strip.show().unwrap();
/// ```
#[derive(Debug)]
pub struct RmtOutput<'ch, C: Chip, Dm: DriverMode, B> {
    /// The configured channel. A blocking `transmit` consumes it and hands it back
    /// when done, so it sits in an `Option` while a frame is in flight.
    channel: Option<Channel<'ch, Dm, Tx>>,
    buffer: B,
    ticks: PulseTicks,
    chip: PhantomData<C>,
}

impl<'ch, C: Chip, Dm: DriverMode, B> RmtOutput<'ch, C, Dm, B> {
    /// Configures an RMT transmit channel for `C`'s timing and connects it to `pin`.
    ///
    /// `rmt_hz` is the RMT's real clock rate, in hertz: the value you gave `Rmt::new`
    /// (or read it back with `Rmt::frequency()`). It must be 10 MHz
    /// times a whole number from 1 to 255 (80 MHz is the usual choice); ledkit divides
    /// it to 10 MHz ticks. `buffer` holds the frame; see [`buffer_len`].
    ///
    /// The channel is set up to hold the pin low whenever it isn't sending, with no
    /// carrier. Using a clocked chip (a DotStar) is a compile error.
    ///
    /// A chip whose reset is longer than 65 534 ticks (6.5 ms) has its reset capped
    /// at that. No chip in the catalogue comes close, but a custom `Chip` could.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ClockNotUsable`] if `rmt_hz` can't be divided to 10 MHz,
    /// [`Error::Timing`] if the chip's pulses don't fit the RMT (not expected for the
    /// catalogue), and [`Error::Config`] if `esp-hal` refuses the channel.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = RmtOutput::<Ws2812b, _, _>::new(
    ///     rmt.channel0,
    ///     peripherals.GPIO8,
    ///     80_000_000,
    ///     [PulseCode::default(); buffer_len::<Ws2812b>(8)],
    /// )?;
    /// ```
    pub fn new(
        channel: impl TxChannelCreator<'ch, Dm>,
        pin: impl PeripheralOutput<'ch>,
        rmt_hz: u32,
        buffer: B,
    ) -> Result<Self, Error> {
        // Evaluated at compile time: a clocked chip can't be driven this way, and
        // using one here fails the build.
        let timing = const { ClocklessTiming::of::<C>() };
        let divider = rmt_clock_divider(rmt_hz, TICK_HZ).ok_or(Error::ClockNotUsable { rmt_hz })?;
        // ledkit checks each pulse against the chip's datasheet window at this tick rate.
        let ticks = timing.pulse_ticks(TICK_HZ, MAX_TICKS)?;

        // Idle output on and low: between frames (and before the first) the pin is
        // driven low instead of floating, which the LEDs read as the reset gap.
        let config = TxChannelConfig::default()
            .with_clk_divider(divider)
            .with_idle_output_level(Level::Low)
            .with_idle_output(true)
            .with_carrier_modulation(false);
        let channel = channel.configure_tx(&config)?.with_pin(pin);

        Ok(Self {
            channel: Some(channel),
            buffer,
            ticks,
            chip: PhantomData,
        })
    }
}

impl<C: Chip, Dm: DriverMode, B: AsMut<[PulseCode]>> RmtOutput<'_, C, Dm, B> {
    /// Fills the buffer with the frame and returns how many pulse codes to send.
    /// Fails before anything is sent if the buffer is too small.
    pub(crate) fn encode_wire(
        &mut self,
        wire: impl IntoIterator<Item = u8>,
    ) -> Result<usize, Error> {
        Ok(fill_rmt_words(&self.ticks, wire, self.buffer.as_mut())?)
    }
}

impl<C: Chip, B: AsMut<[PulseCode]>> RmtOutput<'_, C, Blocking, B> {
    /// Sends the first `used` buffer entries and waits until the RMT has sent them,
    /// including the reset gap.
    pub(crate) fn send_blocking(&mut self, used: usize) -> Result<(), Error> {
        let channel = self.channel.take().ok_or(Error::ChannelLost)?;
        let frame = &self.buffer.as_mut()[..used];
        // Both outcomes of `transmit` and `wait` give the channel back.
        let (channel, result) = match channel.transmit(frame) {
            Ok(transaction) => match transaction.wait() {
                Ok(channel) => (channel, Ok(())),
                Err((e, channel)) => (channel, Err(Error::Rmt(e))),
            },
            Err((e, channel)) => (channel, Err(Error::Rmt(e))),
        };
        self.channel = Some(channel);
        result
    }
}

impl<C: Chip, B: AsMut<[PulseCode]>> RmtOutput<'_, C, Async, B> {
    /// The async twin of `send_blocking`.
    pub(crate) async fn send_async(&mut self, used: usize) -> Result<(), Error> {
        let channel = self.channel.as_mut().ok_or(Error::ChannelLost)?;
        let frame = &self.buffer.as_mut()[..used];
        channel.transmit(frame).await.map_err(Error::Rmt)
    }
}

impl<C: Chip, B: AsMut<[PulseCode]>> Output<C> for RmtOutput<'_, C, Blocking, B> {
    type Error = Error;

    /// Sends one frame and waits until it, and its reset gap, are out (about 30 us
    /// per LED).
    fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        // ledkit puts each pixel's channels in the chip's order (green, red, blue for
        // the WS2812B).
        let used = self.encode_wire(wire::clockless_bytes::<C, _>(pixels))?;
        self.send_blocking(used)
    }
}

impl<C: Chip, B: AsMut<[PulseCode]>> OutputAsync<C> for RmtOutput<'_, C, Async, B> {
    type Error = Error;

    /// Sends one frame and waits (without blocking the executor) until it, and its
    /// reset gap, are out.
    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_wire(wire::clockless_bytes::<C, _>(pixels))?;
        self.send_async(used).await
    }
}
