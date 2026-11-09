//! A ledkit backend that drives NeoPixels with the nRF52840's PWM peripheral and
//! EasyDMA, through `embassy-nrf` (bare metal, `no_std`), blocking or async.
//!
//! These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
//! precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
//! and the length of the high pulse tells `0` from `1`. The nRF52's PWM can play a list
//! of "duty" values from RAM, one per PWM period, without the processor's help
//! (EasyDMA reads them). ledkit sets the PWM period to one LED bit and writes one duty
//! value per bit, so the whole frame goes out in hardware and interrupts or the radio
//! can't disturb it.
//!
//! ## Which nRF backend do I use?
//!
//! | You want | Use |
//! |---|---|
//! | NeoPixels (WS2812B, SK6812, ...) on any pin, exact timing, no CPU | `ledkit-nrf` (this crate) |
//! | NeoPixels with less RAM (15 bytes per LED instead of 48) | `ledkit-spi`'s `ClocklessSpi::new_packed` on a SPIM at 4 MHz |
//! | DotStars (APA102, SK9822), which have a clock wire | `ledkit-spi`'s `ClockedSpi` on a SPIM |
//!
//! The nRF's serial port (UARTE) can't drive NeoPixels: it tops out at 1 Mbaud and
//! can't invert its output, so `ledkit-uart` refuses it.
//!
//! ## Quick tour
//!
//! This can only run on an nRF52840, so the snippet is not run by `cargo test`:
//!
//! ```ignore
//! use ledkit_core::prelude::*;
//! use ledkit_nrf::{buffer_len, PwmOutput};
//!
//! const LED_COUNT: usize = 8;
//!
//! let p = embassy_nrf::init(Default::default());
//! let output = PwmOutput::<Ws2812b, _>::new(
//!     p.PWM0,
//!     p.P1_15, // pin D10 on the Seeed XIAO nRF52840
//!     [0u16; buffer_len::<Ws2812b>(LED_COUNT)],
//! )
//! .unwrap();
//! let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
//! strip.fill(Rgb::BLUE);
//! strip.show().unwrap();
//! ```
//!
//! In an async (Embassy) program use `strip.show_async().await` instead (the
//! `OutputAsync` side of ledkit, feature `async`, on by default).
//!
//! Don't cancel `show_async()`, for example by dropping it inside `select`. Dropping it
//! stops the PWM half way through the frame, and the next frame may then join onto it
//! without a reset gap, so the colors come out wrong. Let each `show_async().await`
//! finish.
//!
//! ## Which PWM, which pin?
//!
//! The nRF52840 has four PWM peripherals, `PWM0` to `PWM3`, and each can drive any GPIO
//! pin. Use `PWM0` unless something else in your program already does. One `PwmOutput`
//! drives one strip.
//!
//! On the Seeed XIAO nRF52840 the pins marked on the board are:
//!
//! | Board pin | D0 | D1 | D2 | D3 | D4 | D5 | D6 | D7 | D8 | D9 | D10 |
//! |---|---|---|---|---|---|---|---|---|---|---|---|
//! | `embassy-nrf` name | `P0_02` | `P0_03` | `P0_28` | `P0_29` | `P0_04` | `P0_05` | `P1_11` | `P1_12` | `P1_13` | `P1_14` | `P1_15` |
//!
//! Avoid `P0_09` and `P0_10`: they are the NFC antenna pins and don't work as GPIO
//! until NFC is turned off in the chip's settings (UICR). The XIAO's own RGB LED is on
//! `P0_26`, `P0_30` and `P0_06`. Check the pins against your board's pinout card.
//!
//! The pin is driven with the nRF's "high drive" strength for sharp edges on a long
//! wire. The nRF52840's pins are 3.3 V; most WS2812B strips read that, but if the first
//! LED flickers, add a 5 V level shifter such as a 74AHCT125.
//!
//! ## Sizing the buffer
//!
//! The driver builds the whole frame in RAM first: one 16-bit duty word per color bit,
//! plus the reset gap (below). [`buffer_len`] is a `const fn`, so it sizes an array:
//!
//! ```ignore
//! use ledkit_core::Ws2812b;
//! use ledkit_nrf::buffer_len;
//!
//! // 30 LEDs * 3 colors * 8 bits + 205 reset words = 925 words
//! const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
//! let buffer = [0u16; BUFFER_LEN];
//! ```
//!
//! That is 48 bytes per RGB LED and 64 per RGBW LED: 100 WS2812B LEDs take about
//! 5.2 KB, 300 take about 15 KB, of the nRF52840's 256 KB. `B` can also be a
//! `&'static mut [u16; N]` (for example from the `static_cell` crate), which avoids
//! moving a large array through the stack. The buffer must be in RAM (EasyDMA can't read
//! flash), which an array or a `static mut` always is.
//!
//! One frame can be at most 32 767 words (the PWM's limit): about 1350 RGB LEDs or 1020
//! RGBW LEDs. A longer frame is refused with [`Error::FrameTooLong`].
//!
//! ## Timing
//!
//! The PWM counts a 16 MHz clock (62.5 ns per tick). Every bit has the same length, so
//! ledkit looks for the bit length whose `0` and `1` both fit the chip's datasheet
//! ([`ClocklessTiming::equal_period_ticks`](ledkit_core::ClocklessTiming::equal_period_ticks)):
//! 22 ticks (1.375 us) for the WS2812B, 20 ticks (1.25 us) for the WS2812, WS2813 and
//! WS2811, 19 for the SK6812. This happens when the program is compiled.
//!
//! The PWM's clock follows the chip's main clock. Start the 32 MHz crystal for exact
//! timing (`config.hfclk_source = HfclkSource::ExternalXtal` before
//! `embassy_nrf::init`); without it the internal oscillator is still close enough for
//! the LEDs.
//!
//! ## The reset gap
//!
//! The LEDs latch the new colors once the data line has been low for the chip's reset
//! time (280 us for the WS2812B). ledkit ends every frame with that many all-low duty
//! words (205 for the WS2812B: one more than the gap needs, because the PWM reports the
//! end of a frame as its last word starts), and `show()` returns only after they have
//! played, so two `show()` calls back to back both take effect. Between frames the pin
//! is held low.
//!
//! ## Async
//!
//! `show_async()` starts the frame, sleeps with an `embassy-time` timer for exactly as
//! long as the frame takes (other tasks run meanwhile), then checks that the PWM has
//! finished. So an async program needs an `embassy-time` driver, which every Embassy
//! program has (for `embassy-nrf`, its `time-driver-rtc1` feature). A blocking-only
//! program can turn off the default `async` feature and needs no time driver.
//!
//! ## Coming from `smart-leds`?
//!
//! With the `smart-leds` feature, [`PwmOutput`] implements `smart-leds`'
//! `SmartLedsWrite` (and `SmartLedsWriteAsync` with `async`), so code written for
//! `nrf-smartled` needs only a new constructor. Colors are sent exactly as given: no
//! brightness, gamma or dithering is added, so keep using `smart-leds`' own
//! `brightness()` and `gamma()` adaptors.
//!
//! ## Building
//!
//! Pick the chip with a Cargo feature (`nrf52840`). Your own project provides the rest of
//! the `embassy-nrf` setup: the `rt` feature, a time driver, `memory.x` and the target
//! `thumbv7em-none-eabihf`, which builds with plain stable Rust. Copy it from the
//! repository's `examples/nrf52840-pwm`, which uses this crate.
//!
//! ## Tested on hardware
//!
//! | Board | LEDs | Result |
//! |---|---|---|
//! | Seeed XIAO nRF52840 (D10, through a 74AHCT125) | WS2812B strip, 100 LEDs | blocking, async (with a busy task) and the 1–150 LED length sweep pass, 2026-11-08 |
//!
//! The duty words and timings are also covered by host tests in `ledkit-core`
//! (`equal_period_ticks`, `fill_pwm_words`). If you try another board or LED, please report
//! what you see.
//!
//! ## Features
//!
//! - `nrf52840`: the chip. Required.
//! - `async` (default): [`OutputAsync`] for [`PwmOutput`]; needs an `embassy-time` driver.
//! - `smart-leds`: the `smart-leds` write traits for [`PwmOutput`].
#![no_std]

use core::marker::PhantomData;

use embassy_nrf::gpio::{Level, OutputDrive, Pin as GpioPin};
use embassy_nrf::pwm::{
    self, CounterMode, Prescaler, SequenceConfig, SequenceLoad, SequencePwm, SingleSequenceMode,
    SingleSequencer, PWM_CLK_HZ,
};
use embassy_nrf::Peri;
use ledkit_core::encode::{fill_pwm_words, pwm_reset_words, pwm_word_count, EqualPeriodTicks};
use ledkit_core::{wire, Chip, ClocklessTiming, Output};

#[cfg(feature = "async")]
use ledkit_core::OutputAsync;

mod error;
#[cfg(feature = "smart-leds")]
mod smart_leds;

pub use error::Error;

/// The most words one PWM sequence can hold (its 15-bit count register), and so the
/// longest frame a [`PwmOutput`] sends.
///
/// # Example
///
/// ```ignore
/// use ledkit_core::Ws2812b;
/// use ledkit_nrf::{buffer_len, MAX_FRAME_WORDS};
///
/// // 1300 WS2812B LEDs fit in one frame; 1400 don't.
/// assert!(buffer_len::<Ws2812b>(1300) <= MAX_FRAME_WORDS);
/// assert!(buffer_len::<Ws2812b>(1400) > MAX_FRAME_WORDS);
/// ```
pub const MAX_FRAME_WORDS: usize = 32_767;

/// The longest bit the PWM can count, in ticks (its 15-bit counter top).
const MAX_PERIOD: u32 = 32_767;

/// The bit timing of `C` on the PWM's 16 MHz clock, worked out at compile time.
const fn ticks_of<C: Chip>() -> EqualPeriodTicks {
    match ClocklessTiming::of::<C>().equal_period_ticks(PWM_CLK_HZ, MAX_PERIOD) {
        Ok(ticks) => ticks,
        Err(_) => panic!("this LED's timing can't be made with the nRF PWM's 16 MHz clock"),
    }
}

/// How many all-low words end each frame of `C`, for its reset gap.
///
/// One more than the gap needs: the PWM reports the end of a sequence when its last word
/// starts, not when it ends, and `write` stops the PWM at that report.
const fn reset_words_of<C: Chip>() -> usize {
    let ticks = ticks_of::<C>();
    pwm_reset_words(
        ClocklessTiming::of::<C>().reset_us,
        ticks.period,
        PWM_CLK_HZ,
    ) + 1
}

/// How many 16-bit words [`PwmOutput`] needs for `led_count` LEDs: one per color bit,
/// plus the reset gap.
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
/// use ledkit_nrf::buffer_len;
///
/// const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
/// let buffer = [0u16; BUFFER_LEN];
/// assert_eq!(buffer.len(), 925); // 30 * 24 bits + 205 reset words
/// ```
#[must_use]
pub const fn buffer_len<C: Chip>(led_count: usize) -> usize {
    pwm_word_count(led_count * C::ORDER.len(), reset_words_of::<C>())
}

/// A chain of clockless LEDs of type `C` (for example [`ledkit_core::Ws2812b`]) on one
/// GPIO pin, driven by one of the nRF52840's PWM peripherals.
///
/// - `C` is your LED type.
/// - `B` is the buffer the frame is built in, for example `[u16; N]` or
///   `&'static mut [u16; N]`. Use [`buffer_len`] to find the size you need.
///
/// It implements [`Output`] (blocking) and, with the `async` feature, [`OutputAsync`].
/// You usually don't call `write` yourself. Put the driver in a
/// [`Strip`](ledkit_core::Strip) and call `strip.show()` (or `show_async()`).
///
/// To get the PWM and the pin back, drop the output. The pin is then left floating (an
/// unconnected input), so the LEDs may see noise until you use it again; a 10 kΩ
/// pull-down resistor from the data wire to GND keeps it low.
///
/// # DotStars
///
/// DotStars (APA102 and SK9822) have their own clock wire and don't need this. Using
/// one here is a build error; use `ledkit-spi`'s `ClockedSpi` on a SPIM instead.
///
/// ```ignore
/// // Does not compile: DotStars need a clocked backend such as ledkit-spi.
/// // (This is `ignore` because ledkit-nrf only builds for an nRF target, so a
/// // `compile_fail` doctest would "fail" for the wrong reason on the host.)
/// let _ = PwmOutput::<Apa102, _>::new(p.PWM0, p.P1_15, [0u16; 64]);
/// ```
///
/// # Example
///
/// ```ignore
/// use ledkit_core::prelude::*;
/// use ledkit_nrf::{buffer_len, PwmOutput};
///
/// const LED_COUNT: usize = 8;
/// let p = embassy_nrf::init(Default::default());
/// let output =
///     PwmOutput::<Ws2812b, _>::new(p.PWM0, p.P1_15, [0u16; buffer_len::<Ws2812b>(LED_COUNT)])
///         .unwrap();
/// let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
/// strip.fill(Rgb::GREEN);
/// strip.show().unwrap();
/// ```
pub struct PwmOutput<'d, C: Chip, B> {
    pwm: SequencePwm<'d>,
    buffer: B,
    chip: PhantomData<C>,
}

impl<'d, C: Chip, B> PwmOutput<'d, C, B> {
    /// Sets up the PWM peripheral `pwm` to send `C`'s signal on `pin`, building frames in
    /// `buffer` (see [`buffer_len`]).
    ///
    /// The PWM runs at 16 MHz with one bit per period; the pin is driven low between
    /// frames, with high drive strength. Using a clocked chip (a DotStar) is a compile
    /// error.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Pwm`] if `embassy-nrf` refuses the setup (not expected).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = PwmOutput::<Ws2812b, _>::new(p.PWM0, p.P1_15, [0u16; buffer_len::<Ws2812b>(8)])?;
    /// ```
    pub fn new<T: pwm::Instance>(
        pwm: Peri<'d, T>,
        pin: Peri<'d, impl GpioPin>,
        buffer: B,
    ) -> Result<Self, Error> {
        // Evaluated at compile time: a clocked chip, or one whose timing the PWM can't
        // make, fails the build.
        let ticks = const { ticks_of::<C>() };

        let mut config = pwm::Config::default();
        config.counter_mode = CounterMode::Up;
        config.prescaler = Prescaler::Div1;
        config.max_duty = ticks.period as u16;
        config.sequence_load = SequenceLoad::Common;
        config.ch0_drive = OutputDrive::HighDrive;
        config.ch0_idle_level = Level::Low;
        let pwm = SequencePwm::new_1ch(pwm, pin, config)?;

        Ok(Self {
            pwm,
            buffer,
            chip: PhantomData,
        })
    }
}

impl<C: Chip, B: AsMut<[u16]>> PwmOutput<'_, C, B> {
    /// Fills the buffer with the frame and returns how many words to send. Fails before
    /// anything is sent if the buffer is too small or the frame too long.
    pub(crate) fn encode_wire(
        &mut self,
        wire: impl IntoIterator<Item = u8>,
    ) -> Result<usize, Error> {
        let ticks = const { ticks_of::<C>() };
        let reset_words = const { reset_words_of::<C>() };
        let used = fill_pwm_words(wire, ticks, reset_words, self.buffer.as_mut())?;
        if used > MAX_FRAME_WORDS {
            return Err(Error::FrameTooLong {
                words: used,
                max: MAX_FRAME_WORDS,
            });
        }
        Ok(used)
    }

    /// Plays the first `used` words once and waits until they, including the reset gap,
    /// are out.
    pub(crate) fn send_blocking(&mut self, used: usize) -> Result<(), Error> {
        let mut done = self.pwm.event_loops_done();
        // embassy-nrf clears this event only when the PWM is set up, so clear it before
        // every frame or the wait below would end at once from the second frame on.
        done.clear();
        let words = &self.buffer.as_mut()[..used];
        // Dropping the sequencer stops the PWM, so it lives until the frame is out.
        let sequencer = SingleSequencer::new(&mut self.pwm, words, SequenceConfig::default());
        sequencer.start(SingleSequenceMode::Times(1))?;
        // A CPU cycle is at least a quarter of a PWM tick (64 MHz against 16 MHz), so
        // this many polls cover eight times the frame: far more than it needs.
        let mut polls_left = frame_ticks::<C>(used).saturating_mul(32);
        while !done.is_triggered() {
            if polls_left == 0 {
                return Err(Error::Timeout);
            }
            polls_left -= 1;
        }
        drop(sequencer);
        Ok(())
    }

    /// The async twin of `send_blocking`: sleeps for the frame's length, then checks
    /// that the PWM has finished.
    ///
    /// Dropping the future mid-frame stops the PWM with a partial frame on the LEDs, and
    /// the next frame may join onto it without a reset gap, so don't cancel it.
    #[cfg(feature = "async")]
    pub(crate) async fn send_async(&mut self, used: usize) -> Result<(), Error> {
        use embassy_time::{Duration, Instant, Timer};

        let mut done = self.pwm.event_loops_done();
        done.clear();
        let words = &self.buffer.as_mut()[..used];
        let sequencer = SingleSequencer::new(&mut self.pwm, words, SequenceConfig::default());
        sequencer.start(SingleSequenceMode::Times(1))?;
        let frame =
            Duration::from_micros(frame_ticks::<C>(used) / u64::from(PWM_CLK_HZ / 1_000_000));
        let started = Instant::now();
        Timer::after(frame).await;
        // Normally done by now; allow the frame again plus 1 ms before giving up.
        let deadline = started + frame + frame + Duration::from_millis(1);
        while !done.is_triggered() {
            if Instant::now() > deadline {
                return Err(Error::Timeout);
            }
            embassy_futures::yield_now().await;
        }
        drop(sequencer);
        Ok(())
    }
}

/// How many PWM clock ticks `words` duty words of `C` take to play.
fn frame_ticks<C: Chip>(words: usize) -> u64 {
    let period = const { ticks_of::<C>().period };
    words as u64 * u64::from(period)
}

impl<C: Chip, B: AsMut<[u16]>> Output<C> for PwmOutput<'_, C, B> {
    type Error = Error;

    /// Sends one frame and waits until it, and its reset gap, are out (about 33 us per
    /// RGB LED, plus 280 us for a WS2812B's reset).
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

#[cfg(feature = "async")]
impl<C: Chip, B: AsMut<[u16]>> OutputAsync<C> for PwmOutput<'_, C, B> {
    type Error = Error;

    /// Sends one frame and waits (without blocking the executor) until it, and its
    /// reset gap, are out.
    ///
    /// Don't cancel this future (for example inside `select`): a frame dropped half way
    /// leaves a partial frame on the LEDs, and the next frame may join onto it without
    /// a reset gap.
    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_wire(wire::clockless_bytes::<C, _>(pixels))?;
        self.send_async(used).await
    }
}
