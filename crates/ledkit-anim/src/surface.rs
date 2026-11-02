//! [`Surface`]: anything an animation can be drawn onto.

use ledkit_color::Rgb;
use ledkit_core::{Chip, Strip};
use ledkit_layout::{Canvas, Layout, SegmentMut};

/// Something with LEDs to draw on: a whole [`Strip`], a matrix [`Canvas`], a
/// [`SegmentMut`] (part of a strip), or a plain array or slice of colors.
///
/// You rarely need this trait yourself; it's what lets
/// [`Animation::render`](crate::Animation::render) draw on all of them. A strip
/// (or segment, or array) is a surface one LED tall.
///
/// # Example
///
/// ```
/// use ledkit_anim::Surface;
/// use ledkit_color::Rgb;
///
/// let leds = [Rgb::BLACK; 30];
/// assert_eq!((leds.width(), leds.height()), (30, 1));
/// ```
pub trait Surface {
    /// How many LEDs wide (the length, for a strip).
    fn width(&self) -> u16;

    /// How many LEDs tall (1 for a strip).
    fn height(&self) -> u16;

    /// The place along the chain of the LED at `(x, y)`, or `None` if there is
    /// no LED there.
    fn index(&self, x: u16, y: u16) -> Option<usize>;

    /// Sets the LED at `(x, y)` to `color`. Does nothing if there is no LED there.
    fn set(&mut self, x: u16, y: u16, color: Rgb);
}

/// The width of a strip-like surface of `len` LEDs, capped at `u16::MAX`.
fn strip_width(len: usize) -> u16 {
    u16::try_from(len).unwrap_or(u16::MAX)
}

/// The index of `(x, y)` on a strip-like surface `width` LEDs long.
fn strip_index(width: u16, x: u16, y: u16) -> Option<usize> {
    (y == 0 && x < width).then_some(usize::from(x))
}

impl<P: From<Rgb>> Surface for [P] {
    fn width(&self) -> u16 {
        strip_width(self.len())
    }

    fn height(&self) -> u16 {
        1
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        strip_index(self.width(), x, y)
    }

    fn set(&mut self, x: u16, y: u16, color: Rgb) {
        if let Some(i) = self.index(x, y) {
            self[i] = color.into();
        }
    }
}

impl<P: From<Rgb>, const N: usize> Surface for [P; N] {
    fn width(&self) -> u16 {
        self.as_slice().width()
    }

    fn height(&self) -> u16 {
        1
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        self.as_slice().index(x, y)
    }

    fn set(&mut self, x: u16, y: u16, color: Rgb) {
        self.as_mut_slice().set(x, y, color);
    }
}

impl<P: From<Rgb>> Surface for SegmentMut<'_, P> {
    fn width(&self) -> u16 {
        strip_width(self.len())
    }

    fn height(&self) -> u16 {
        1
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        strip_index(self.width(), x, y)
    }

    fn set(&mut self, x: u16, y: u16, color: Rgb) {
        if let Some(i) = self.index(x, y) {
            SegmentMut::set(self, i, color);
        }
    }
}

impl<P: From<Rgb>, L: Layout> Surface for Canvas<'_, P, L> {
    fn width(&self) -> u16 {
        Canvas::width(self)
    }

    fn height(&self) -> u16 {
        Canvas::height(self)
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        self.layout().index(x, y)
    }

    fn set(&mut self, x: u16, y: u16, color: Rgb) {
        Canvas::set(self, x, y, color);
    }
}

impl<C: Chip, const N: usize, O> Surface for Strip<C, N, O> {
    fn width(&self) -> u16 {
        strip_width(N)
    }

    fn height(&self) -> u16 {
        1
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        strip_index(Surface::width(self), x, y)
    }

    fn set(&mut self, x: u16, y: u16, color: Rgb) {
        if let Some(i) = Surface::index(self, x, y) {
            Strip::set(self, i, color);
        }
    }
}
