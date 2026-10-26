//! Helpers for testing code that uses ledkit, with no hardware needed.

use crate::{Chip, Output, OutputAsync, Pixel};
use core::convert::Infallible;

/// An output that just remembers the last frame it was given.
///
/// Handy in tests, or to try a program before wiring up any LEDs. `N` is
/// the most pixels it remembers.
///
/// # Example
///
/// ```
/// use ledkit_core::testing::MemoryOutput;
/// use ledkit_core::{Rgb, Strip, Ws2812b};
///
/// let mut strip: Strip<Ws2812b, 2, MemoryOutput<Rgb, 2>> = Strip::new(MemoryOutput::new());
/// strip.fill(Rgb::RED);
/// strip.show().unwrap();
/// assert_eq!(strip.output().last_frame(), &[Rgb::RED, Rgb::RED]);
/// assert_eq!(strip.output().frames_written, 1);
/// ```
#[derive(Debug, Clone)]
pub struct MemoryOutput<P: Pixel, const N: usize> {
    /// How many frames have been written so far.
    pub frames_written: usize,
    frame: [P; N],
    len: usize,
}

impl<P: Pixel, const N: usize> MemoryOutput<P, N> {
    /// Creates an empty output.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::testing::MemoryOutput;
    /// use ledkit_core::Rgb;
    ///
    /// let out: MemoryOutput<Rgb, 8> = MemoryOutput::new();
    /// assert_eq!(out.frames_written, 0);
    /// assert!(out.last_frame().is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            frames_written: 0,
            frame: [P::default(); N],
            len: 0,
        }
    }

    /// The pixels of the last frame written (at most `N` of them).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::testing::MemoryOutput;
    /// use ledkit_core::{Rgb, Strip, Ws2812b};
    ///
    /// let mut strip: Strip<Ws2812b, 2, MemoryOutput<Rgb, 2>> = Strip::new(MemoryOutput::new());
    /// strip.set(1, Rgb::GREEN);
    /// strip.show().unwrap();
    /// assert_eq!(strip.output().last_frame(), &[Rgb::BLACK, Rgb::GREEN]);
    /// ```
    #[must_use]
    pub fn last_frame(&self) -> &[P] {
        &self.frame[..self.len]
    }

    fn store<I: Iterator<Item = P>>(&mut self, pixels: I) {
        let mut n = 0;
        for (slot, p) in self.frame.iter_mut().zip(pixels) {
            *slot = p;
            n += 1;
        }
        self.len = n;
        self.frames_written += 1;
    }
}

impl<P: Pixel, const N: usize> Default for MemoryOutput<P, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Chip, const N: usize> Output<C> for MemoryOutput<C::Pixel, N> {
    type Error = Infallible;

    fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.store(pixels);
        Ok(())
    }
}

impl<C: Chip, const N: usize> OutputAsync<C> for MemoryOutput<C::Pixel, N> {
    type Error = Infallible;

    #[allow(clippy::unused_async_trait_impl)] // the trait requires an async fn; nothing here can wait
    async fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.store(pixels);
        Ok(())
    }
}
