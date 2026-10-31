//! `embedded-graphics` support: a [`Canvas`] is a draw target.
//!
//! Drawing outside the canvas, including at negative coordinates, is ignored,
//! so text and shapes can hang off any edge.

use crate::{Canvas, Layout};
use core::convert::Infallible;
use embedded_graphics_core::draw_target::DrawTarget;
use embedded_graphics_core::geometry::{OriginDimensions, Size};
use embedded_graphics_core::pixelcolor::{Rgb888, RgbColor};
use embedded_graphics_core::Pixel;
use ledkit_color::Rgb;

/// Turns an `embedded-graphics` [`Rgb888`] into a ledkit [`Rgb`].
///
/// This is a function, not a `From` impl, because a `From` impl isn't
/// possible in this crate: both [`Rgb888`] and [`Rgb`] come from other crates,
/// and Rust only lets a crate implement a trait when it owns one of the types.
///
/// # Example
///
/// ```
/// use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
/// use ledkit_color::Rgb;
/// use ledkit_layout::rgb_from_rgb888;
///
/// assert_eq!(rgb_from_rgb888(Rgb888::RED), Rgb::RED);
/// ```
pub fn rgb_from_rgb888(color: Rgb888) -> Rgb {
    Rgb {
        r: color.r(),
        g: color.g(),
        b: color.b(),
    }
}

/// Turns a ledkit [`Rgb`] into an `embedded-graphics` [`Rgb888`].
///
/// This is a function, not a `From` impl, because a `From` impl isn't
/// possible in this crate (see [`rgb_from_rgb888`]).
///
/// # Example
///
/// ```
/// use embedded_graphics::pixelcolor::{Rgb888, RgbColor};
/// use ledkit_color::Rgb;
/// use ledkit_layout::rgb888_from_rgb;
///
/// assert_eq!(rgb888_from_rgb(Rgb::GREEN), Rgb888::GREEN);
/// ```
pub fn rgb888_from_rgb(color: Rgb) -> Rgb888 {
    Rgb888::new(color.r, color.g, color.b)
}

impl<P, L: Layout> OriginDimensions for Canvas<'_, P, L> {
    fn size(&self) -> Size {
        Size::new(u32::from(self.width()), u32::from(self.height()))
    }
}

impl<P, L: Layout> DrawTarget for Canvas<'_, P, L>
where
    Rgb: Into<P>,
{
    type Color = Rgb888;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            // Negative or huge coordinates have no LED; skip them.
            if let (Ok(x), Ok(y)) = (u16::try_from(point.x), u16::try_from(point.y)) {
                self.set(x, y, rgb_from_rgb888(color));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Canvas, MatrixLayout};
    use embedded_graphics::mono_font::ascii::FONT_6X10;
    use embedded_graphics::mono_font::MonoTextStyle;
    use embedded_graphics::pixelcolor::Rgb888;
    use embedded_graphics::prelude::*;
    use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
    use embedded_graphics::text::Text;
    use embedded_graphics::Pixel;
    use ledkit_color::Rgb;

    #[test]
    fn draw_target_clear_blacks_out_the_canvas() {
        let mut px = [Rgb::RED; 14];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 3)).unwrap();
        c.clear(Rgb888::BLACK).unwrap();
        assert!(px[..12].iter().all(|p| *p == Rgb::BLACK));
        assert_eq!(px[12], Rgb::RED);
    }

    #[test]
    fn draws_a_rectangle_outline() {
        let mut px = [Rgb::BLACK; 16];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 4)).unwrap();
        Rectangle::new(Point::new(0, 0), Size::new(4, 4))
            .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
            .draw(&mut c)
            .unwrap();
        assert_eq!(c.get(0, 0), Some(Rgb::RED));
        assert_eq!(c.get(1, 1), Some(Rgb::BLACK));
    }

    #[test]
    fn text_hanging_off_every_edge_does_not_panic() {
        let mut px = [Rgb::BLACK; 64];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(8, 8).serpentine()).unwrap();
        let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
        for x in [-20, -3, 0, 5, 30] {
            for y in [-12, 0, 9, 40] {
                Text::new("Hi!", Point::new(x, y), style)
                    .draw(&mut c)
                    .unwrap();
            }
        }
        assert!(px.iter().any(|p| *p != Rgb::BLACK));
    }

    #[test]
    fn coordinates_do_not_wrap_onto_other_leds() {
        // 65538 is 2 plus 65536, so a truncating cast would land on LED 2.
        let mut px = [Rgb::BLACK; 4];
        let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 1)).unwrap();
        c.draw_iter([
            Pixel(Point::new(65538, 0), Rgb888::WHITE),
            Pixel(Point::new(-1, 0), Rgb888::WHITE),
        ])
        .unwrap();
        assert_eq!(px, [Rgb::BLACK; 4]);
    }
}
