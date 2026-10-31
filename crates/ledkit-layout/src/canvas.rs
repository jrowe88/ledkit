//! The 2D canvas.

use crate::Layout;
use core::fmt;

/// The layout needs more LEDs than the strip has.
///
/// `needed` is how many LEDs the layout (or segment) reaches, `available` is
/// how many the pixel slice holds. Nothing was drawn.
///
/// # Example
///
/// ```
/// use ledkit_color::Rgb;
/// use ledkit_layout::{Canvas, LayoutTooBig, MatrixLayout};
///
/// let mut pixels = [Rgb::BLACK; 10];
/// let result = Canvas::new(&mut pixels, MatrixLayout::new(4, 3));
/// assert_eq!(result.err(), Some(LayoutTooBig { needed: 12, available: 10 }));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutTooBig {
    /// How many LEDs the layout needs.
    pub needed: usize,
    /// How many LEDs the strip has.
    pub available: usize,
}

impl fmt::Display for LayoutTooBig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "this needs {} LEDs but the strip has {}",
            self.needed, self.available
        )
    }
}

impl core::error::Error for LayoutTooBig {}

/// A drawing surface: a strip's pixels seen through a [`Layout`], addressed
/// by `(x, y)`.
///
/// A canvas borrows the pixels, so you draw in a block and call
/// `strip.show()` after it. If the slice is longer than the layout, the extra
/// LEDs are not part of the canvas and are never touched.
///
/// Drawing outside the canvas is ignored, so shapes can run off the edge.
///
/// # Example
///
/// ```
/// use ledkit_color::Rgb;
/// use ledkit_layout::{Canvas, MatrixLayout};
///
/// let mut pixels = [Rgb::BLACK; 12];
/// let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
/// canvas.set(1, 2, Rgb::RED);
/// assert_eq!(canvas.get(1, 2), Some(Rgb::RED));
/// canvas.set(9, 9, Rgb::RED); // off the canvas: ignored
/// assert_eq!(canvas.get(9, 9), None);
/// ```
#[derive(Debug)]
pub struct Canvas<'a, P, L: Layout> {
    pixels: &'a mut [P],
    layout: L,
}

impl<'a, P, L: Layout> Canvas<'a, P, L> {
    /// Makes a canvas over `pixels` using `layout`.
    ///
    /// Fails with [`LayoutTooBig`] if the layout needs more LEDs than
    /// `pixels` holds. A longer slice is fine.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutTooBig`] when `pixels` is shorter than the layout.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 14]; // 12 for the matrix, 2 spare
    /// assert!(Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).is_ok());
    /// ```
    pub fn new(pixels: &'a mut [P], layout: L) -> Result<Self, LayoutTooBig> {
        let needed = layout.len();
        let available = pixels.len();
        if needed > available {
            return Err(LayoutTooBig { needed, available });
        }
        Ok(Self { pixels, layout })
    }

    /// How many LEDs wide the canvas is.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 12];
    /// let canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// assert_eq!(canvas.width(), 4);
    /// ```
    pub fn width(&self) -> u16 {
        self.layout.width()
    }

    /// How many LEDs tall the canvas is.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 12];
    /// let canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// assert_eq!(canvas.height(), 3);
    /// ```
    pub fn height(&self) -> u16 {
        self.layout.height()
    }

    /// The layout this canvas draws through.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, Layout, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 12];
    /// let canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// assert_eq!(canvas.layout().len(), 12);
    /// ```
    pub fn layout(&self) -> &L {
        &self.layout
    }

    /// Sets the LED at `(x, y)`. Positions outside the canvas are ignored.
    ///
    /// `color` can be anything that converts into the strip's pixel type, so
    /// an `Rgb` works on an RGBW strip too.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 12];
    /// let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// canvas.set(3, 0, Rgb::GREEN);
    /// assert_eq!(pixels[3], Rgb::GREEN);
    /// ```
    pub fn set(&mut self, x: u16, y: u16, color: impl Into<P>) {
        if let Some(slot) = self.layout.index(x, y).and_then(|i| self.pixels.get_mut(i)) {
            *slot = color.into();
        }
    }

    /// The color at `(x, y)`, or `None` outside the canvas.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 12];
    /// let canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// assert_eq!(canvas.get(0, 0), Some(Rgb::BLACK));
    /// assert_eq!(canvas.get(4, 0), None);
    /// ```
    pub fn get(&self, x: u16, y: u16) -> Option<P>
    where
        P: Copy,
    {
        self.layout
            .index(x, y)
            .and_then(|i| self.pixels.get(i))
            .copied()
    }

    /// Sets every LED of the canvas to `color`. Spare LEDs beyond the layout
    /// are left alone.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::BLACK; 14];
    /// let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// canvas.fill(Rgb::BLUE);
    /// assert_eq!(pixels[11], Rgb::BLUE);
    /// assert_eq!(pixels[12], Rgb::BLACK);
    /// ```
    pub fn fill(&mut self, color: impl Into<P>)
    where
        P: Clone,
    {
        let color: P = color.into();
        let count = self.layout.len();
        for slot in &mut self.pixels[..count] {
            *slot = color.clone();
        }
    }

    /// Turns every LED of the canvas off (the pixel type's default, which is
    /// black).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_color::Rgb;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    ///
    /// let mut pixels = [Rgb::RED; 12];
    /// let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3)).unwrap();
    /// canvas.clear();
    /// assert_eq!(pixels[0], Rgb::BLACK);
    /// ```
    pub fn clear(&mut self)
    where
        P: Default + Clone,
    {
        self.fill(P::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MatrixLayout;
    use ledkit_color::{Rgb, Rgbw};
    use ledkit_core::testing::MemoryOutput;
    use ledkit_core::Strip;
    use ledkit_core::Ws2812b;

    #[test]
    fn canvas_draws_into_the_strip_buffer() {
        let mut strip = Strip::<Ws2812b, 12, _>::new(MemoryOutput::<Rgb, 12>::new());
        {
            let mut c =
                Canvas::new(strip.pixels_mut(), MatrixLayout::new(4, 3).serpentine()).unwrap();
            c.set(0, 1, Rgb::RED);
        }
        assert_eq!(strip.pixels()[7], Rgb::RED);
    }

    #[test]
    fn off_canvas_writes_are_ignored() {
        let mut px = [Rgb::BLACK; 12];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 3)).unwrap();
        c.set(4, 0, Rgb::RED);
        c.set(0, 3, Rgb::RED);
        c.set(u16::MAX, u16::MAX, Rgb::RED);
        assert_eq!(c.get(4, 0), None);
        assert!(px.iter().all(|p| *p == Rgb::BLACK));
    }

    #[test]
    fn layout_too_big_is_an_error() {
        let mut px = [Rgb::BLACK; 10];
        assert_eq!(
            Canvas::new(&mut px, MatrixLayout::new(4, 3)).err(),
            Some(LayoutTooBig {
                needed: 12,
                available: 10
            })
        );
    }

    #[test]
    fn fill_leaves_the_tail_alone() {
        let mut px = [Rgb::BLACK; 14];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 3)).unwrap();
        c.fill(Rgb::RED);
        assert!(px[..12].iter().all(|p| *p == Rgb::RED));
        assert_eq!(px[12], Rgb::BLACK);
        assert_eq!(px[13], Rgb::BLACK);
    }

    #[test]
    fn clear_and_get_work() {
        let mut px = [Rgb::BLACK; 4];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(2, 2)).unwrap();
        c.set(1, 1, Rgb::RED);
        assert_eq!(c.get(1, 1), Some(Rgb::RED));
        c.clear();
        assert_eq!(c.get(1, 1), Some(Rgb::BLACK));
        assert_eq!((c.width(), c.height()), (2, 2));
        assert_eq!(c.layout().len(), 4);
    }

    #[test]
    fn rgbw_canvas_takes_rgb() {
        let mut px = [Rgbw::default(); 4];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(2, 2)).unwrap();
        c.set(1, 0, Rgb::RED);
        assert_eq!(px[1], Rgbw::from(Rgb::RED));
    }

    #[test]
    fn error_message_is_readable() {
        let e = LayoutTooBig {
            needed: 12,
            available: 10,
        };
        assert_eq!(format!("{e}"), "this needs 12 LEDs but the strip has 10");
    }
}
