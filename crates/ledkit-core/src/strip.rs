//! The [`Strip`] pixel buffer.

use crate::pipeline::{hd_split, process, PipelineSettings, DITHER_SEQUENCE, NO_DITHER};
use crate::power::{limited_brightness, WORST_CASE_DITHER};
use crate::{Brightness, Chip, Milliamps, Output, OutputAsync, Rgb};
use core::marker::PhantomData;
use core::ops::{Bound, RangeBounds};
use ledkit_color::{correction, GammaTable};

/// A row of LEDs: a buffer of `N` colors plus the settings used to send it.
///
/// You pick the chip (what is printed on your LEDs), the number of LEDs and
/// an output (the driver that actually talks to your hardware). Set colors
/// with [`set`](Strip::set) or [`fill`](Strip::fill), then call
/// [`show`](Strip::show) to send them. The colors you set are never changed
/// by `show`; brightness and friends are applied on the way out.
///
/// A new strip is black, at full brightness, with no color gamma, no color
/// correction, no power limit and dithering on.
///
/// # Example
///
/// ```
/// use ledkit_core::testing::MemoryOutput;
/// use ledkit_core::{Rgb, Strip, Ws2812b};
///
/// // A strip of 8 NeoPixels. A real program would use a hardware driver
/// // instead of `MemoryOutput`.
/// let mut strip: Strip<Ws2812b, 8, MemoryOutput<Rgb, 8>> = Strip::new(MemoryOutput::new());
///
/// strip.fill(Rgb::BLUE);
/// strip.set(0, Rgb::RED);
/// strip.set_brightness(0.25); // a quarter as bright to the eye
/// strip.show().unwrap();
///
/// assert_eq!(strip.get(0), Some(Rgb::RED));
/// assert_eq!(strip.output().frames_written, 1);
/// ```
#[derive(Debug)]
pub struct Strip<C: Chip, const N: usize, O> {
    pixels: [C::Pixel; N],
    output: O,
    brightness: Brightness,
    gamma: Option<&'static GammaTable>,
    correction: Rgb,
    power_limit: Option<Milliamps>,
    dithering: bool,
    hd: bool,
    frame: u8,
    last_raw: Option<u8>,
    chip: PhantomData<C>,
}

impl<C: Chip, const N: usize, O> Strip<C, N, O> {
    /// Creates an all-black strip that sends through `output`.
    ///
    /// It starts at full brightness, with no color gamma, no color
    /// correction and no power limit. Dithering is on, which suits
    /// animations that call `show` about 100 times a second or more; for a
    /// mostly still, dim display that calls `show` slowly, see
    /// [`set_dithering`](Strip::set_dithering). DotStar "HD" brightness is
    /// on; see [`set_hd_brightness`](Strip::set_hd_brightness).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::testing::MemoryOutput;
    /// use ledkit_core::{Rgb, Strip, Ws2812b};
    ///
    /// let strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// assert_eq!(strip.get(0), Some(Rgb::BLACK));
    /// ```
    #[must_use]
    pub fn new(output: O) -> Self {
        Self {
            pixels: [C::Pixel::default(); N],
            output,
            brightness: Brightness::FULL,
            gamma: None,
            correction: correction::NONE,
            power_limit: None,
            dithering: true,
            hd: true,
            frame: 0,
            last_raw: None,
            chip: PhantomData,
        }
    }

    /// Sets one LED. An `index` past the end is ignored.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set(0, Rgb::RED);
    /// strip.set(99, Rgb::RED); // past the end: ignored
    /// assert_eq!(strip.get(0), Some(Rgb::RED));
    /// ```
    pub fn set(&mut self, index: usize, color: impl Into<C::Pixel>) {
        if let Some(slot) = self.pixels.get_mut(index) {
            *slot = color.into();
        }
    }

    /// Reads one LED's color, or `None` if `index` is past the end.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set(2, Rgb::BLUE);
    /// assert_eq!(strip.get(2), Some(Rgb::BLUE));
    /// assert_eq!(strip.get(4), None);
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Option<C::Pixel> {
        self.pixels.get(index).copied()
    }

    /// Sets every LED to one color.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::ORANGE);
    /// assert!(strip.pixels().iter().all(|&p| p == Rgb::ORANGE));
    /// ```
    pub fn fill(&mut self, color: impl Into<C::Pixel>) {
        self.pixels.fill(color.into());
    }

    /// Sets a range of LEDs to one color, for example `strip.fill_range(2..5, color)`.
    ///
    /// Parts of the range outside the strip are ignored; an empty or
    /// backwards range does nothing.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill_range(1..3, Rgb::GREEN);
    /// assert_eq!(
    ///     strip.pixels(),
    ///     &[Rgb::BLACK, Rgb::GREEN, Rgb::GREEN, Rgb::BLACK]
    /// );
    /// ```
    pub fn fill_range(&mut self, range: impl RangeBounds<usize>, color: impl Into<C::Pixel>) {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e.saturating_add(1),
            Bound::Excluded(&e) => e,
            Bound::Unbounded => N,
        };
        let end = end.min(N);
        if start >= end {
            return;
        }
        let color = color.into();
        for slot in &mut self.pixels[start..end] {
            *slot = color;
        }
    }

    /// Sets every LED to black (off). The strip changes when you next call `show`.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::WHITE);
    /// strip.clear();
    /// assert_eq!(strip.get(0), Some(Rgb::BLACK));
    /// ```
    pub fn clear(&mut self) {
        self.pixels.fill(C::Pixel::default());
    }

    /// How many LEDs the strip has.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// assert_eq!(strip.len(), 4);
    /// ```
    #[must_use]
    pub const fn len(&self) -> usize {
        N
    }

    /// True if the strip has no LEDs.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// assert!(!strip.is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    /// All the colors, as stored (before brightness and the other settings).
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set(3, Rgb::RED);
    /// assert_eq!(strip.pixels()[3], Rgb::RED);
    /// ```
    #[must_use]
    pub fn pixels(&self) -> &[C::Pixel; N] {
        &self.pixels
    }

    /// All the colors, for changing in place.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// for (i, p) in strip.pixels_mut().iter_mut().enumerate() {
    ///     *p = if i % 2 == 0 { Rgb::RED } else { Rgb::BLUE };
    /// }
    /// assert_eq!(strip.get(1), Some(Rgb::BLUE));
    /// ```
    pub fn pixels_mut(&mut self) -> &mut [C::Pixel; N] {
        &mut self.pixels
    }

    /// Sets the overall brightness, for example `strip.set_brightness(0.5)`.
    ///
    /// A fraction or [`Brightness::from_percent`] is "perceived" brightness:
    /// half looks about half as bright. [`Brightness::exact`] skips that curve.
    ///
    /// Calling this before every `show()` is an intended use, for example to
    /// make the strip breathe. It is cheap, your stored colors are never
    /// touched, and dithering and the power limit are worked out again on
    /// every `show()`, so they follow the new brightness. To fade single
    /// pixels instead, use [`Dim::dimmed`](crate::Dim::dimmed).
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::Brightness;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set_brightness(0.5); // looks half as bright
    /// strip.set_brightness(Brightness::from_percent(25));
    /// strip.set_brightness(Brightness::exact(64)); // a quarter of the power
    /// assert_eq!(strip.brightness(), Brightness::exact(64));
    /// ```
    pub fn set_brightness(&mut self, b: impl Into<Brightness>) {
        self.brightness = b.into();
    }

    /// The brightness you asked for.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::Brightness;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set_brightness(Brightness::from_percent(50));
    /// assert_eq!(strip.brightness(), Brightness::from_percent(50));
    /// ```
    #[must_use]
    pub fn brightness(&self) -> Brightness {
        self.brightness
    }

    /// Turns on a color gamma table (such as `GAMMA_2_6`) for every `show`.
    ///
    /// Use this when your colors come from images, color pickers or web
    /// colors: those values are made for screens, and without a curve
    /// dim colors look too bright on LEDs. It is off by default because
    /// colors you pick by hand or compute (rainbows, fades) already look
    /// right, and a gamma curve would darken them and lose detail in the
    /// dark shades.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::prelude::GAMMA_2_6;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set_color_gamma(&GAMMA_2_6);
    /// strip.set(0, Rgb::new(128, 255, 0));
    /// strip.show().unwrap();
    /// assert_eq!(strip.output().last_frame()[0], Rgb::new(42, 255, 0));
    /// ```
    pub fn set_color_gamma(&mut self, table: &'static GammaTable) {
        self.gamma = Some(table);
    }

    /// Turns the color gamma off again (the default).
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::prelude::GAMMA_2_6;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set_color_gamma(&GAMMA_2_6);
    /// strip.clear_color_gamma();
    /// strip.set(0, Rgb::new(128, 255, 0));
    /// strip.show().unwrap();
    /// assert_eq!(strip.output().last_frame()[0], Rgb::new(128, 255, 0));
    /// ```
    pub fn clear_color_gamma(&mut self) {
        self.gamma = None;
    }

    /// Sets a white-balance correction, such as `correction::TYPICAL_LED_STRIP`.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::prelude::correction;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.set_correction(correction::TYPICAL_LED_STRIP);
    /// strip.fill(Rgb::WHITE);
    /// strip.show().unwrap();
    /// // The stored colors are unchanged; only what is sent is corrected.
    /// assert_eq!(strip.get(0), Some(Rgb::WHITE));
    /// ```
    pub fn set_correction(&mut self, correction: Rgb) {
        self.correction = correction;
    }

    /// Keeps the estimated current under `limit` by lowering the brightness when needed.
    ///
    /// The current is an estimate for your chip, not a measurement.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::Milliamps;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::WHITE); // about 244 mA at full brightness
    /// strip.set_power_limit(Milliamps(100));
    /// strip.show().unwrap();
    /// assert!(strip.effective_brightness().output_level() < 255);
    /// ```
    pub fn set_power_limit(&mut self, limit: Milliamps) {
        self.power_limit = Some(limit);
    }

    /// Removes the power limit (the default).
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::Milliamps;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::WHITE);
    /// strip.set_power_limit(Milliamps(100));
    /// strip.clear_power_limit();
    /// strip.show().unwrap();
    /// assert_eq!(strip.effective_brightness().output_level(), 255);
    /// ```
    pub fn clear_power_limit(&mut self) {
        self.power_limit = None;
    }

    /// Turns dithering on or off (it is on by default).
    ///
    /// Dithering varies the rounding a little from frame to frame so dim
    /// colors look smoother over time. It shines when `show` is called
    /// quickly (about 100 times a second or more, as animations do), because
    /// your eyes blend the frames together. If you call `show` slowly or
    /// irregularly at low brightness, the changes are slow enough to see as
    /// a gentle twinkle; call `set_dithering(false)` in that case.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// // A slow clock display: turn dithering off to avoid a visible twinkle.
    /// strip.set_dithering(false);
    /// strip.set_brightness(0.1);
    /// strip.fill(Rgb::WHITE);
    /// strip.show().unwrap();
    /// ```
    pub fn set_dithering(&mut self, on: bool) {
        self.dithering = on;
    }

    /// Turns DotStar "HD" brightness on or off (it is on by default).
    ///
    /// DotStars (APA102, SK9822) have a 5-bit brightness level in every LED,
    /// on top of the 8-bit color channels. Normally ledkit leaves it at full
    /// and dims through the channels alone, so a dim strip has few color
    /// steps left: at 10% brightness each channel only goes from 0 to about
    /// 2, and soft colors round to off. With HD on, ledkit turns the 5-bit
    /// level down for the dim half of the range and keeps the channels wide,
    /// so dim colors stay smooth and true. Above about half brightness HD
    /// changes nothing, because the 5-bit level doesn't scale the light
    /// exactly in proportion, and turning it down there would make HD dimmer.
    ///
    /// On paper the light is the same either way, but very short color
    /// pulses barely light some DotStars: on an Adafruit DotStar HD 8x8
    /// (APA102-2020), 10% brightness is dark without HD and lit with it.
    ///
    /// It only works with an output that sends the 5-bit level, such as
    /// `ledkit-spi`'s `ClockedSpi`. With any other output (and with
    /// NeoPixels) it does nothing. One catch on the APA102 (not the SK9822):
    /// it makes that 5-bit level with a slow on/off flicker (a few hundred
    /// times a second), which some cameras pick up as banding at low
    /// brightness. If you film your LEDs, turn HD off.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Apa102, Rgb, Strip};
    /// let mut strip: Strip<Apa102, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// // Filming at low brightness: avoid the APA102's 5-bit flicker.
    /// strip.set_hd_brightness(false);
    /// assert!(!strip.hd_brightness());
    /// ```
    pub fn set_hd_brightness(&mut self, on: bool) {
        self.hd = on;
    }

    /// Whether DotStar "HD" brightness is on; see
    /// [`set_hd_brightness`](Self::set_hd_brightness).
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Apa102, Rgb, Strip};
    /// let strip: Strip<Apa102, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// assert!(strip.hd_brightness());
    /// ```
    #[must_use]
    pub fn hd_brightness(&self) -> bool {
        self.hd
    }

    /// The brightness the last successful `show` really used, after power limiting.
    ///
    /// It reflects the last successful `show`: it is updated each time
    /// `show` (or `show_async`) sends a frame, so changing the colors,
    /// brightness or power limit afterwards does not change it until the
    /// next one. A `show` that returns an error leaves it as it was. Before
    /// the first successful `show` it is the brightness you set. It is always an exact output
    /// level ([`Brightness::exact`]), so compare it with another exact value
    /// or use [`Brightness::output_level`].
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// # use ledkit_core::Milliamps;
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::WHITE);
    /// strip.set_power_limit(Milliamps(100));
    /// strip.show().unwrap();
    /// let used = strip.effective_brightness().output_level();
    /// assert!(used < 255); // dimmed to stay under 100 mA
    /// ```
    #[must_use]
    pub fn effective_brightness(&self) -> Brightness {
        Brightness::exact(
            self.last_raw
                .unwrap_or_else(|| self.brightness.output_level()),
        )
    }

    /// The output (driver), for reading.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.show().unwrap();
    /// assert_eq!(strip.output().frames_written, 1);
    /// ```
    #[must_use]
    pub fn output(&self) -> &O {
        &self.output
    }

    /// The output (driver), for changing.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.output_mut().frames_written = 0;
    /// assert_eq!(strip.output().frames_written, 0);
    /// ```
    pub fn output_mut(&mut self) -> &mut O {
        &mut self.output
    }

    /// Ends the strip and gives back its output (driver), for example to take
    /// the bus back out of it or to build a new strip on it.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.show().unwrap();
    /// let output = strip.release();
    /// assert_eq!(output.frames_written, 1);
    /// ```
    #[must_use]
    pub fn release(self) -> O {
        self.output
    }

    /// Works out the settings for this frame, with `hd` saying whether it
    /// goes out with HD brightness (so the power limit counts what HD
    /// sends). Nothing is changed until [`commit`](Self::commit) is called
    /// after a successful send.
    fn prepare(&self, hd: bool) -> PipelineSettings {
        let mut settings = PipelineSettings {
            correction: self.correction,
            gamma: self.gamma,
            brightness: self.brightness.output_level(),
            dither: if self.dithering {
                DITHER_SEQUENCE[usize::from(self.frame % 8)]
            } else {
                NO_DITHER
            },
        };
        if let Some(limit) = self.power_limit {
            let assumed = if self.dithering {
                WORST_CASE_DITHER
            } else {
                NO_DITHER
            };
            settings.brightness =
                limited_brightness(&self.pixels, &settings, C::POWER, limit, assumed, hd);
        }
        settings
    }

    /// Records a frame that was sent: remembers its brightness and moves
    /// the dither on to the next frame.
    fn commit(&mut self, brightness: u8) {
        self.last_raw = Some(brightness);
        self.frame = self.frame.wrapping_add(1);
    }

    /// Sends the strip to the LEDs.
    ///
    /// Brightness, gamma, correction and the power limit are applied on the
    /// way out; the stored colors stay as you set them.
    ///
    /// # Errors
    ///
    /// Returns the output's error if sending failed. A failed `show`
    /// changes nothing: [`effective_brightness`](Strip::effective_brightness)
    /// and the dither step stay as they were.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::RED);
    /// strip.show().unwrap();
    /// assert_eq!(strip.output().last_frame(), &[Rgb::RED; 4]);
    /// ```
    pub fn show(&mut self) -> Result<(), O::Error>
    where
        O: Output<C>,
    {
        // Full global level first; this also tells us whether the output
        // takes one at all.
        let hd = Output::<C>::set_global_brightness(&mut self.output, 31) && self.hd;
        let mut settings = self.prepare(hd);
        let brightness = settings.brightness;
        if hd {
            let (global, channel) = hd_split(brightness);
            Output::<C>::set_global_brightness(&mut self.output, global);
            settings.brightness = channel;
        }
        self.output
            .write(self.pixels.iter().map(|p| process(*p, &settings)))?;
        self.commit(brightness);
        Ok(())
    }

    /// Like [`show`](Strip::show), but `async`.
    ///
    /// # Errors
    ///
    /// Returns the output's error if sending failed. As with `show`, a
    /// failed send changes nothing.
    ///
    /// # Example
    ///
    /// ```
    /// # use ledkit_core::testing::MemoryOutput;
    /// # use ledkit_core::{Rgb, Strip, Ws2812b};
    /// let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
    /// strip.fill(Rgb::RED);
    /// embassy_futures::block_on(strip.show_async()).unwrap();
    /// assert_eq!(strip.output().last_frame(), &[Rgb::RED; 4]);
    /// ```
    pub async fn show_async(&mut self) -> Result<(), O::Error>
    where
        O: OutputAsync<C>,
    {
        let hd = OutputAsync::<C>::set_global_brightness(&mut self.output, 31) && self.hd;
        let mut settings = self.prepare(hd);
        let brightness = settings.brightness;
        if hd {
            let (global, channel) = hd_split(brightness);
            OutputAsync::<C>::set_global_brightness(&mut self.output, global);
            settings.brightness = channel;
        }
        self.output
            .write(self.pixels.iter().map(|p| process(*p, &settings)))
            .await?;
        self.commit(brightness);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MemoryOutput;
    use crate::{Apa102, Brightness, Milliamps, Rgb, Rgbw, Sk6812Rgbw, Ws2812b};
    use core::convert::Infallible;

    type Mem<const N: usize> = MemoryOutput<Rgb, N>;
    fn strip<const N: usize>() -> Strip<Ws2812b, N, Mem<N>> {
        Strip::new(MemoryOutput::new())
    }
    #[test]
    fn new_strip_is_black() {
        let s = strip::<4>();
        assert!(s.pixels().iter().all(|p| *p == Rgb::BLACK));
        assert_eq!(s.len(), 4);
    }
    #[test]
    fn release_gives_back_the_output() {
        let mut s = strip::<2>();
        s.fill(Rgb::RED);
        s.show().unwrap();
        let output = s.release();
        assert_eq!(output.frames_written, 1);
    }
    #[test]
    fn set_get_and_out_of_range() {
        let mut s = strip::<3>();
        s.set(1, Rgb::RED);
        s.set(3, Rgb::BLUE);
        s.set(usize::MAX, Rgb::BLUE);
        assert_eq!(s.get(1), Some(Rgb::RED));
        assert_eq!(s.get(3), None);
        assert_eq!(s.pixels(), &[Rgb::BLACK, Rgb::RED, Rgb::BLACK]);
    }
    #[test]
    fn fill_range_clamps() {
        let mut s = strip::<4>();
        s.fill_range(2..10, Rgb::GREEN);
        assert_eq!(
            s.pixels(),
            &[Rgb::BLACK, Rgb::BLACK, Rgb::GREEN, Rgb::GREEN]
        );
        s.fill(Rgb::BLUE);
        s.clear();
        assert!(s.pixels().iter().all(|p| *p == Rgb::BLACK));
    }
    #[test]
    #[allow(clippy::reversed_empty_ranges)] // the backwards range is the point of the test
    fn fill_range_edge_cases_never_panic() {
        let mut s = strip::<4>();
        s.fill_range(..=usize::MAX, Rgb::RED);
        assert_eq!(s.pixels(), &[Rgb::RED; 4]);

        s.clear();
        s.fill_range((Bound::Excluded(usize::MAX), Bound::Unbounded), Rgb::RED);
        assert_eq!(s.pixels(), &[Rgb::BLACK; 4]);

        s.fill_range(3..1, Rgb::RED);
        assert_eq!(s.pixels(), &[Rgb::BLACK; 4]);

        s.fill_range(10..20, Rgb::RED);
        assert_eq!(s.pixels(), &[Rgb::BLACK; 4]);

        s.fill_range((Bound::Excluded(0), Bound::Included(2)), Rgb::BLUE);
        assert_eq!(s.pixels(), &[Rgb::BLACK, Rgb::BLUE, Rgb::BLUE, Rgb::BLACK]);

        s.fill_range(3..=3, Rgb::GREEN);
        assert_eq!(s.pixels(), &[Rgb::BLACK, Rgb::BLUE, Rgb::BLUE, Rgb::GREEN]);
    }
    #[test]
    fn many_shows_keep_the_dither_phase() {
        let mut s = strip::<1>();
        s.set(0, Rgb::new(200, 0, 0));
        s.set_brightness(Brightness::exact(128));
        for _ in 0..300 {
            s.show().unwrap();
        }
        // The 300th show was frame 299: 299 % 8 = 3, offset 192, red 101.
        assert_eq!(s.output().last_frame()[0].r, 101);
        // The next one is frame 300: 300 % 8 = 4, DITHER_SEQUENCE[4] = 32, red 100.
        assert_eq!(DITHER_SEQUENCE[4], 32);
        s.show().unwrap();
        assert_eq!(s.output().last_frame()[0].r, 100);
        assert_eq!(s.output().frames_written, 301);
    }
    #[test]
    fn effective_brightness_reflects_the_last_show() {
        let mut s = strip::<10>();
        s.fill(Rgb::WHITE);
        s.set_power_limit(Milliamps(300));
        s.set_dithering(false);
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(122));
        s.clear_power_limit();
        assert_eq!(s.effective_brightness(), Brightness::exact(122));
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(255));
    }
    #[test]
    fn show_sends_processed_pixels_without_changing_stored_ones() {
        let mut s = strip::<2>();
        s.fill(Rgb::new(200, 200, 200));
        s.set_brightness(Brightness::exact(128));
        s.set_dithering(false);
        s.show().unwrap();
        assert_eq!(s.output().last_frame(), &[Rgb::new(101, 101, 101); 2]);
        assert_eq!(s.pixels(), &[Rgb::new(200, 200, 200); 2]);
        assert_eq!(s.output().frames_written, 1);
    }
    #[test]
    fn defaults_send_colors_unchanged() {
        let mut s = strip::<1>();
        for _ in 0..8 {
            s.set(0, Rgb::ORANGE);
            s.show().unwrap();
            assert_eq!(s.output().last_frame(), &[Rgb::ORANGE]);
        }
    }
    #[test]
    fn half_brightness_is_perceptual() {
        let mut s = strip::<1>();
        s.set(0, Rgb::WHITE);
        s.set_brightness(0.5);
        s.set_dithering(false);
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(56));
        // scale_channel(255, 56, 128) = (255*57+128)>>8 = 57
        assert_eq!(s.output().last_frame()[0], Rgb::new(57, 57, 57));
    }
    #[test]
    fn color_gamma_is_opt_in() {
        let mut s = strip::<1>();
        s.set(0, Rgb::new(128, 255, 0));
        s.set_color_gamma(&ledkit_color::GAMMA_2_6);
        s.show().unwrap();
        assert_eq!(s.output().last_frame()[0], Rgb::new(42, 255, 0));
        s.clear_color_gamma();
        s.show().unwrap();
        assert_eq!(s.output().last_frame()[0], Rgb::new(128, 255, 0));
    }
    #[test]
    fn power_limit_applies_on_show() {
        let mut s = strip::<10>();
        s.fill(Rgb::WHITE);
        s.set_power_limit(Milliamps(300));
        s.set_dithering(false);
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(122));
        assert_eq!(s.output().last_frame()[0], Rgb::new(123, 123, 123));
    }
    #[test]
    fn power_limit_follows_the_dithering_setting() {
        let mut s = strip::<10>();
        s.fill(Rgb::WHITE);
        s.set_power_limit(Milliamps(363));
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(149));
        s.set_dithering(false);
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(150));
    }
    #[test]
    fn dithering_is_on_by_default_and_varies_over_frames() {
        let mut s = strip::<1>();
        s.set(0, Rgb::new(200, 0, 0));
        s.set_brightness(Brightness::exact(128));
        let mut reds = [0u8; 8];
        for r in &mut reds {
            s.show().unwrap();
            *r = s.output().last_frame()[0].r;
        }
        assert_eq!(reds, [100, 101, 101, 101, 100, 101, 101, 101]);
    }
    #[test]
    fn rgbw_strip_accepts_rgb_colors() {
        let mut s: Strip<Sk6812Rgbw, 1, MemoryOutput<Rgbw, 1>> = Strip::new(MemoryOutput::new());
        s.set(0, Rgb::RED);
        assert_eq!(s.get(0), Some(Rgbw::new(255, 0, 0, 0)));
    }
    /// An output that fails while `fail` is set, and otherwise remembers
    /// frames like `MemoryOutput`.
    struct Flaky<const N: usize> {
        fail: bool,
        memory: Mem<N>,
    }

    #[derive(Debug, PartialEq)]
    struct SendFailed;

    impl<const N: usize> Output<Ws2812b> for Flaky<N> {
        type Error = SendFailed;
        fn write<I>(&mut self, pixels: I) -> Result<(), SendFailed>
        where
            I: Iterator<Item = Rgb> + ExactSizeIterator,
        {
            if self.fail {
                return Err(SendFailed);
            }
            let Ok(()) = Output::<Ws2812b>::write(&mut self.memory, pixels);
            Ok(())
        }
    }

    impl<const N: usize> OutputAsync<Ws2812b> for Flaky<N> {
        type Error = SendFailed;
        #[allow(clippy::unused_async_trait_impl)] // the trait requires an async fn
        async fn write<I>(&mut self, pixels: I) -> Result<(), SendFailed>
        where
            I: Iterator<Item = Rgb> + ExactSizeIterator,
        {
            Output::<Ws2812b>::write(self, pixels)
        }
    }

    fn flaky<const N: usize>() -> Strip<Ws2812b, N, Flaky<N>> {
        Strip::new(Flaky {
            fail: true,
            memory: MemoryOutput::new(),
        })
    }

    #[test]
    fn failed_show_keeps_effective_brightness() {
        let mut s = flaky::<10>();
        s.fill(Rgb::WHITE);
        s.set_power_limit(Milliamps(300));
        s.set_dithering(false);
        assert_eq!(s.show(), Err(SendFailed));
        assert_eq!(s.effective_brightness(), Brightness::exact(255));
        s.output_mut().fail = false;
        s.show().unwrap();
        assert_eq!(s.effective_brightness(), Brightness::exact(122));
        s.output_mut().fail = true;
        s.clear_power_limit();
        assert_eq!(s.show(), Err(SendFailed));
        assert_eq!(s.effective_brightness(), Brightness::exact(122));
    }
    #[test]
    fn failed_show_keeps_the_dither_phase() {
        let mut s = flaky::<1>();
        s.set(0, Rgb::new(200, 0, 0));
        s.set_brightness(Brightness::exact(128));
        // Frame 0 rounds red to 100, frames 1 to 3 to 101.
        assert_eq!(s.show(), Err(SendFailed));
        assert_eq!(s.show(), Err(SendFailed));
        s.output_mut().fail = false;
        let mut reds = [0u8; 8];
        for r in &mut reds {
            s.show().unwrap();
            *r = s.output().memory.last_frame()[0].r;
        }
        assert_eq!(reds, [100, 101, 101, 101, 100, 101, 101, 101]);
        assert_eq!(s.output().memory.frames_written, 8);
    }
    #[test]
    fn failed_async_show_keeps_state() {
        let mut s = flaky::<1>();
        s.set(0, Rgb::new(200, 0, 0));
        s.set_brightness(Brightness::exact(128));
        s.set_power_limit(Milliamps(5));
        let failed = embassy_futures::block_on(s.show_async());
        assert_eq!(failed, Err(SendFailed));
        assert_eq!(s.effective_brightness(), Brightness::exact(128));
        s.output_mut().fail = false;
        s.clear_power_limit();
        embassy_futures::block_on(s.show_async()).unwrap();
        // Frame 0, not frame 1: red 100.
        assert_eq!(s.output().memory.last_frame()[0].r, 100);
        assert_eq!(s.effective_brightness(), Brightness::exact(128));
        embassy_futures::block_on(s.show_async()).unwrap();
        assert_eq!(s.output().memory.last_frame()[0].r, 101);
    }
    #[test]
    fn async_show() {
        let mut s = strip::<1>();
        s.set(0, Rgb::BLUE);
        embassy_futures::block_on(s.show_async()).unwrap();
        assert_eq!(s.output().last_frame(), &[Rgb::BLUE]);
    }

    /// A DotStar output that takes a global brightness and remembers it.
    struct Hd {
        global: u8,
        memory: MemoryOutput<Rgb, 1>,
    }

    impl Output<Apa102> for Hd {
        type Error = Infallible;
        fn set_global_brightness(&mut self, level: u8) -> bool {
            self.global = level;
            true
        }
        fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
        where
            I: Iterator<Item = Rgb> + ExactSizeIterator,
        {
            Output::<Apa102>::write(&mut self.memory, pixels)
        }
    }

    impl OutputAsync<Apa102> for Hd {
        type Error = Infallible;
        fn set_global_brightness(&mut self, level: u8) -> bool {
            Output::<Apa102>::set_global_brightness(self, level)
        }
        #[allow(clippy::unused_async_trait_impl)] // the trait requires an async fn
        async fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
        where
            I: Iterator<Item = Rgb> + ExactSizeIterator,
        {
            Output::<Apa102>::write(self, pixels)
        }
    }

    fn hd_strip() -> Strip<Apa102, 1, Hd> {
        let mut s = Strip::new(Hd {
            global: 0,
            memory: MemoryOutput::new(),
        });
        s.set_dithering(false);
        s.set_brightness(Brightness::exact(3));
        s.fill(Rgb::new(255, 128, 0));
        s
    }

    #[test]
    fn hd_off_sends_full_global_brightness() {
        let mut s = hd_strip();
        s.set_hd_brightness(false);
        s.show().unwrap();
        assert_eq!(s.output().global, 31);
        assert_eq!(s.output().memory.last_frame(), &[Rgb::new(4, 2, 0)]);
    }
    #[test]
    fn hd_on_moves_the_dimming_into_the_global_level() {
        let mut s = hd_strip();
        s.set_hd_brightness(true);
        s.show().unwrap();
        // Level 3 is global 2 with the channels at 61: the same light on
        // paper (62 * 2 / 31 = 4), with about 15 times as many steps.
        assert_eq!(s.output().global, 2);
        assert_eq!(s.output().memory.last_frame(), &[Rgb::new(62, 31, 0)]);
        assert_eq!(s.effective_brightness(), Brightness::exact(3));
        s.set_hd_brightness(false);
        embassy_futures::block_on(s.show_async()).unwrap();
        assert_eq!(s.output().global, 31);
        assert_eq!(s.output().memory.last_frame(), &[Rgb::new(4, 2, 0)]);
    }
    #[test]
    fn hd_power_limit_counts_what_hd_sends() {
        // A limit of just the idle current: without HD the dim pixel rounds
        // to black; HD must not send it at a lower global level instead.
        let mut s = hd_strip();
        s.set_brightness(Brightness::FULL);
        s.fill(Rgb::new(4, 4, 4));
        s.set_power_limit(Milliamps(1));
        s.show().unwrap();
        assert_eq!(s.output().memory.last_frame(), &[Rgb::BLACK]);
        embassy_futures::block_on(s.show_async()).unwrap();
        assert_eq!(s.output().memory.last_frame(), &[Rgb::BLACK]);
    }
    #[test]
    fn hd_on_does_nothing_without_a_global_level() {
        let mut s: Strip<Apa102, 1, MemoryOutput<Rgb, 1>> = Strip::new(MemoryOutput::new());
        s.set_dithering(false);
        s.set_brightness(Brightness::exact(3));
        s.set_hd_brightness(true);
        s.fill(Rgb::new(255, 128, 0));
        s.show().unwrap();
        assert_eq!(s.output().last_frame(), &[Rgb::new(4, 2, 0)]);
    }
}
