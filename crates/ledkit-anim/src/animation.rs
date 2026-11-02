//! The [`Animation`] trait and what it is given: [`Position`] and [`Frame`].

use ledkit_color::Rgb;

use crate::{Surface, Time};

/// Something that can say what color each LED should be at each moment.
///
/// That's the whole idea: an animation is a recipe "the LED at this position,
/// at this time, is this color". It keeps no memory between frames, so you can
/// ask about any LED at any time, and combining animations (fading between them,
/// stacking them in layers) is just mixing two colors.
///
/// To draw an animation onto LEDs, call [`render`](Animation::render) with a
/// [`Surface`] (a strip, a matrix `Canvas`, a `Segment` of a strip, or a plain
/// array of colors) and the time since the animation started.
///
/// # Writing your own
///
/// ```
/// use ledkit_anim::{Animation, Frame, Position};
/// use ledkit_color::Rgb;
///
/// /// Every LED blinks red once a second, with each LED a little later
/// /// than the one before.
/// struct Ripple;
///
/// impl Animation for Ripple {
///     fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
///         let t = frame.millis + u32::from(at.index) * 50;
///         if t % 1000 < 200 { Rgb::RED } else { Rgb::BLACK }
///     }
/// }
///
/// let mut leds = [Rgb::BLACK; 10];
/// Ripple.render(&mut leds, ledkit_anim::Time::from_millis(0));
/// assert_eq!(leds[0], Rgb::RED);
/// assert_eq!(leds[9], Rgb::BLACK);
/// ```
pub trait Animation {
    /// The color of the LED at `at`, at the moment described by `frame`.
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb;

    /// Draws the animation onto every LED of `surface`, as it looks at time `now`.
    ///
    /// This only sets the colors. Call `show` on your strip afterwards to send
    /// them; brightness, gamma and the power limit are applied there.
    ///
    /// To render a `&dyn Animation`, write `(&animation).render(..)`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_anim::prelude::*;
    ///
    /// struct AllRed;
    /// impl Animation for AllRed {
    ///     fn color_at(&self, _at: Position, _frame: &Frame) -> Rgb {
    ///         Rgb::RED
    ///     }
    /// }
    ///
    /// let mut leds = [Rgb::BLACK; 30];
    /// AllRed.render(&mut leds, Time::from_millis(1500));
    /// assert_eq!(leds[29], Rgb::RED);
    /// ```
    fn render(&self, surface: &mut (impl Surface + ?Sized), now: impl Into<Time>)
    where
        Self: Sized,
    {
        render(self, surface, now.into());
    }
}

impl<A: Animation + ?Sized> Animation for &A {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        (**self).color_at(at, frame)
    }
}

/// Draws `animation` onto `surface` at `now`. Works for `&dyn Animation` too.
fn render<A: Animation + ?Sized, S: Surface + ?Sized>(animation: &A, surface: &mut S, now: Time) {
    let frame = Frame::new(now.as_millis(), surface.width(), surface.height());
    for y in 0..frame.height {
        for x in 0..frame.width {
            if let Some(index) = surface.index(x, y) {
                let at = Position {
                    x,
                    y,
                    index: u16::try_from(index).unwrap_or(u16::MAX),
                };
                let color = animation.color_at(at, &frame);
                surface.set(x, y, color);
            }
        }
    }
}

/// Where an LED is: its column and row, and its place along the chain.
///
/// On a strip, `x` and `index` are the same and `y` is 0. On a matrix, `(x, y)`
/// is the column and row as you look at it (`(0, 0)` is the top-left), and
/// `index` is how far along the wiring the LED is.
///
/// # Example
///
/// ```
/// use ledkit_anim::Position;
///
/// let third_led = Position { x: 2, y: 0, index: 2 };
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Position {
    /// The column, counting from 0 at the left (or the start of a strip).
    pub x: u16,
    /// The row, counting from 0 at the top. Always 0 on a strip.
    pub y: u16,
    /// The LED's place along the chain, from 0.
    pub index: u16,
}

/// What an animation knows about the frame being drawn: the time, and the size
/// of what it's drawn on.
///
/// A strip is a surface one LED tall: `width` is its length and `height` is 1.
///
/// # Example
///
/// ```
/// use ledkit_anim::Frame;
///
/// let frame = Frame::new(1500, 8, 8);
/// assert_eq!(frame.len(), 64);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct Frame {
    /// Milliseconds since the animation started.
    pub millis: u32,
    /// How many LEDs wide the surface is (the length, for a strip).
    pub width: u16,
    /// How many LEDs tall the surface is (1 for a strip).
    pub height: u16,
}

impl Frame {
    /// A frame at `millis` on a surface `width` by `height` LEDs.
    ///
    /// ```
    /// use ledkit_anim::Frame;
    ///
    /// let strip_frame = Frame::new(0, 30, 1);
    /// ```
    pub const fn new(millis: u32, width: u16, height: u16) -> Self {
        Self {
            millis,
            width,
            height,
        }
    }

    /// How many LEDs the surface has.
    ///
    /// ```
    /// use ledkit_anim::Frame;
    ///
    /// assert_eq!(Frame::new(0, 30, 1).len(), 30);
    /// ```
    pub const fn len(&self) -> usize {
        self.width as usize * self.height as usize
    }

    /// True if the surface has no LEDs.
    ///
    /// ```
    /// use ledkit_anim::Frame;
    ///
    /// assert!(Frame::new(0, 0, 1).is_empty());
    /// ```
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The same frame at a different time. Used by animations that run
    /// others on their own clock.
    ///
    /// ```
    /// use ledkit_anim::Frame;
    ///
    /// assert_eq!(Frame::new(500, 8, 8).at_millis(0).millis, 0);
    /// ```
    #[must_use]
    pub const fn at_millis(self, millis: u32) -> Self {
        Self { millis, ..self }
    }
}

/// Which way an effect moves.
///
/// Along a strip, `Forward` moves away from the first LED. For things that
/// spin, `Forward` is clockwise as you look at the panel.
///
/// # Example
///
/// ```
/// use ledkit_anim::prelude::*;
///
/// let backwards = Direction::Backward;
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub enum Direction {
    /// Away from the first LED, or clockwise.
    #[default]
    Forward,
    /// Towards the first LED, or counter-clockwise.
    Backward,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledkit_color::Rgbw;
    use ledkit_core::testing::MemoryOutput;
    use ledkit_core::{Strip, Ws2812b};
    use ledkit_layout::{Canvas, MatrixLayout, Segment};

    /// Reports where each LED is as its color.
    struct Where;

    impl Animation for Where {
        fn color_at(&self, at: Position, _frame: &Frame) -> Rgb {
            Rgb::new(at.x as u8, at.y as u8, at.index as u8)
        }
    }

    #[test]
    fn render_strip_slice() {
        let mut pixels = [Rgb::BLACK; 5];
        Where.render(&mut pixels, Time::from_millis(0));
        assert_eq!(pixels[3], Rgb::new(3, 0, 3));
    }

    #[test]
    fn render_canvas_uses_layout() {
        let mut pixels = [Rgb::BLACK; 12];
        let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(4, 3).serpentine()).unwrap();
        Where.render(&mut canvas, Time::from_millis(0));
        // Serpentine: row 1 runs right to left, so (3, 1) is LED 4.
        assert_eq!(pixels[4], Rgb::new(3, 1, 4));
    }

    #[test]
    fn render_segment() {
        let mut pixels = [Rgb::BLACK; 6];
        let mut segment = Segment::new(2, 3).of(&mut pixels).unwrap();
        Where.render(&mut segment, Time::from_millis(0));
        assert_eq!(pixels[0], Rgb::BLACK);
        assert_eq!(pixels[3], Rgb::new(1, 0, 1));
        assert_eq!(pixels[4], Rgb::new(2, 0, 2));
        assert_eq!(pixels[5], Rgb::BLACK);
    }

    #[test]
    fn render_rgbw() {
        let mut pixels = [Rgbw::default(); 2];
        Where.render(&mut pixels, Time::from_millis(0));
        assert_eq!(pixels[1], Rgbw::from(Rgb::new(1, 0, 1)));
    }

    #[test]
    fn render_strip() {
        let mut strip: Strip<Ws2812b, 4, MemoryOutput<Rgb, 4>> = Strip::new(MemoryOutput::new());
        Where.render(&mut strip, Time::from_millis(0));
        assert_eq!(strip.get(2), Some(Rgb::new(2, 0, 2)));
    }

    #[test]
    fn dyn_render() {
        let animation: &dyn Animation = &Where;
        let mut pixels = [Rgb::BLACK; 3];
        (&animation).render(&mut pixels, Time::from_millis(0));
        assert_eq!(pixels[2], Rgb::new(2, 0, 2));
    }

    #[test]
    fn empty_surfaces() {
        let mut none: [Rgb; 0] = [];
        Where.render(&mut none, Time::from_millis(0));
        let mut pixels = [Rgb::BLACK; 3];
        let mut segment = Segment::new(1, 0).of(&mut pixels).unwrap();
        Where.render(&mut segment, Time::from_millis(0));
        assert_eq!(pixels, [Rgb::BLACK; 3]);
    }
}
