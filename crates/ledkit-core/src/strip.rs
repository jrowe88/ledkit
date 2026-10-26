//! The [`Strip`] pixel buffer.

use crate::pipeline::{process, PipelineSettings, DITHER_SEQUENCE};
use crate::power::limited_brightness;
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
    frame: u8,
    last_raw: Option<u8>,
    chip: PhantomData<C>,
}

impl<C: Chip, const N: usize, O> Strip<C, N, O> {
    /// Creates an all-black strip that sends through `output`.
    ///
    /// It starts at full brightness, with no color gamma, no color
    /// correction and no power limit. Dithering is on.
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
            frame: 0,
            last_raw: None,
            chip: PhantomData,
        }
    }

    /// Sets one LED. An `index` past the end is ignored.
    pub fn set(&mut self, index: usize, color: impl Into<C::Pixel>) {
        if let Some(slot) = self.pixels.get_mut(index) {
            *slot = color.into();
        }
    }

    /// Reads one LED's color, or `None` if `index` is past the end.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<C::Pixel> {
        self.pixels.get(index).copied()
    }

    /// Sets every LED to one color.
    pub fn fill(&mut self, color: impl Into<C::Pixel>) {
        self.pixels = [color.into(); N];
    }

    /// Sets a range of LEDs to one color, for example `strip.fill_range(2..5, color)`.
    ///
    /// Parts of the range outside the strip are ignored; an empty or
    /// backwards range does nothing.
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
    pub fn clear(&mut self) {
        self.pixels = [C::Pixel::default(); N];
    }

    /// How many LEDs the strip has.
    #[must_use]
    pub const fn len(&self) -> usize {
        N
    }

    /// True if the strip has no LEDs.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        N == 0
    }

    /// All the colors, as stored (before brightness and the other settings).
    #[must_use]
    pub fn pixels(&self) -> &[C::Pixel; N] {
        &self.pixels
    }

    /// All the colors, for changing in place.
    pub fn pixels_mut(&mut self) -> &mut [C::Pixel; N] {
        &mut self.pixels
    }

    /// Sets the overall brightness, for example `strip.set_brightness(0.5)`.
    ///
    /// A fraction or [`Brightness::from_percent`] is "perceived" brightness:
    /// half looks about half as bright. [`Brightness::exact`] skips that curve.
    pub fn set_brightness(&mut self, b: impl Into<Brightness>) {
        self.brightness = b.into();
    }

    /// The brightness you asked for.
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
    pub fn set_color_gamma(&mut self, table: &'static GammaTable) {
        self.gamma = Some(table);
    }

    /// Turns the color gamma off again (the default).
    pub fn clear_color_gamma(&mut self) {
        self.gamma = None;
    }

    /// Sets a white-balance correction, such as `correction::TYPICAL_LED_STRIP`.
    pub fn set_correction(&mut self, correction: Rgb) {
        self.correction = correction;
    }

    /// Keeps the estimated current under `limit` by lowering the brightness when needed.
    ///
    /// The current is an estimate for your chip, not a measurement.
    pub fn set_power_limit(&mut self, limit: Milliamps) {
        self.power_limit = Some(limit);
    }

    /// Removes the power limit (the default).
    pub fn clear_power_limit(&mut self) {
        self.power_limit = None;
    }

    /// Turns dithering on or off (it is on by default).
    ///
    /// Dithering varies the rounding a little from frame to frame so dim
    /// colors look smoother over time.
    pub fn set_dithering(&mut self, on: bool) {
        self.dithering = on;
    }

    /// The brightness the last `show` really used, after power limiting.
    ///
    /// Before the first `show` this is the brightness you set. It is always
    /// an exact output level.
    #[must_use]
    pub fn effective_brightness(&self) -> Brightness {
        Brightness::exact(
            self.last_raw
                .unwrap_or_else(|| self.brightness.output_level()),
        )
    }

    /// The output (driver), for reading.
    #[must_use]
    pub fn output(&self) -> &O {
        &self.output
    }

    /// The output (driver), for changing.
    pub fn output_mut(&mut self) -> &mut O {
        &mut self.output
    }

    /// Works out the settings for this frame, then advances the frame counter.
    fn prepare(&mut self) -> PipelineSettings {
        let mut settings = PipelineSettings {
            correction: self.correction,
            gamma: self.gamma,
            brightness: self.brightness.output_level(),
            dither: if self.dithering {
                DITHER_SEQUENCE[usize::from(self.frame % 8)]
            } else {
                128
            },
        };
        if let Some(limit) = self.power_limit {
            settings.brightness = limited_brightness(&self.pixels, &settings, C::POWER, limit);
        }
        self.last_raw = Some(settings.brightness);
        self.frame = self.frame.wrapping_add(1);
        settings
    }

    /// Sends the strip to the LEDs.
    ///
    /// Brightness, gamma, correction and the power limit are applied on the
    /// way out; the stored colors stay as you set them.
    ///
    /// # Errors
    ///
    /// Returns the output's error if sending failed.
    pub fn show(&mut self) -> Result<(), O::Error>
    where
        O: Output<C>,
    {
        let settings = self.prepare();
        self.output
            .write(self.pixels.iter().map(|p| process(*p, &settings)))
    }

    /// Like [`show`](Strip::show), but `async`.
    ///
    /// # Errors
    ///
    /// Returns the output's error if sending failed.
    pub async fn show_async(&mut self) -> Result<(), O::Error>
    where
        O: OutputAsync<C>,
    {
        let settings = self.prepare();
        self.output
            .write(self.pixels.iter().map(|p| process(*p, &settings)))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MemoryOutput;
    use crate::{Brightness, Milliamps, Rgb, Rgbw, Sk6812Rgbw, Ws2812b};

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
    #[test]
    fn async_show() {
        let mut s = strip::<1>();
        s.set(0, Rgb::BLUE);
        embassy_futures::block_on(s.show_async()).unwrap();
        assert_eq!(s.output().last_frame(), &[Rgb::BLUE]);
    }
}
