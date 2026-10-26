//! The traits that send finished pixels to the LEDs.

use crate::Chip;

/// Sends a frame of pixels to LEDs, waiting until the send is done.
///
/// You rarely call this yourself: [`Strip::show`](crate::Strip::show) does
/// it for you. You implement it when writing a driver for a new board or
/// peripheral.
///
/// The `pixels` arrive already processed (brightness, gamma, color
/// correction and power limiting applied), in plain RGB(W) form. A backend
/// only has to turn them into bytes (see [`wire::clockless_bytes`] and
/// [`wire::clocked_frame`], and the [`encode`](crate::encode) module) and
/// push them out.
///
/// [`wire::clockless_bytes`]: crate::wire::clockless_bytes
/// [`wire::clocked_frame`]: crate::wire::clocked_frame
///
/// # Example
///
/// ```
/// use core::convert::Infallible;
/// use ledkit_core::{Output, Rgb, Ws2812b};
///
/// /// Counts the pixels it is given.
/// struct Counter(usize);
///
/// impl Output<Ws2812b> for Counter {
///     type Error = Infallible;
///     fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
///     where
///         I: Iterator<Item = Rgb> + ExactSizeIterator,
///     {
///         self.0 += pixels.len();
///         Ok(())
///     }
/// }
///
/// let mut out = Counter(0);
/// out.write([Rgb::RED; 3].into_iter()).unwrap();
/// assert_eq!(out.0, 3);
/// ```
pub trait Output<C: Chip> {
    /// What can go wrong while sending.
    type Error;

    /// Sends one frame of already processed pixels.
    ///
    /// # Errors
    ///
    /// Returns the backend's error if the frame could not be sent.
    fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator;
}

/// Like [`Output`], but `async`: other tasks can run while the frame is sent.
///
/// The `pixels` arrive already processed, exactly as for [`Output`].
///
/// # Example
///
/// ```
/// use core::convert::Infallible;
/// use ledkit_core::{OutputAsync, Rgb, Ws2812b};
///
/// struct Counter(usize);
///
/// impl OutputAsync<Ws2812b> for Counter {
///     type Error = Infallible;
///     async fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
///     where
///         I: Iterator<Item = Rgb> + ExactSizeIterator,
///     {
///         self.0 += pixels.len();
///         Ok(())
///     }
/// }
///
/// let mut out = Counter(0);
/// embassy_futures::block_on(out.write([Rgb::RED; 2].into_iter())).unwrap();
/// assert_eq!(out.0, 2);
/// ```
#[allow(async_fn_in_trait)] // drivers are used with concrete types, not boxed futures
pub trait OutputAsync<C: Chip> {
    /// What can go wrong while sending.
    type Error;

    /// Sends one frame of already processed pixels.
    ///
    /// # Errors
    ///
    /// Returns the backend's error if the frame could not be sent.
    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator;
}
