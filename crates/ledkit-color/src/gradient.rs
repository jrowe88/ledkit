use crate::Rgb;

/// A smooth blend between color stops placed along a 0-255 scale.
///
/// Each stop is a `(position, color)` pair. Asking for a position before the
/// first stop gives the first color, after the last stop gives the last color,
/// and anywhere in between gives a blend of the two neighbouring stops.
///
/// # Examples
///
/// ```
/// use ledkit_color::{Gradient, Rgb};
///
/// // A sunset: deep purple, then orange, then gold.
/// const SUNSET: Gradient<3> = Gradient::new([
///     (0, Rgb::PURPLE),
///     (128, Rgb::ORANGE),
///     (255, Rgb::GOLD),
/// ]);
///
/// assert_eq!(SUNSET.color_at(0), Rgb::PURPLE);
/// assert_eq!(SUNSET.color_at(128), Rgb::ORANGE);
/// // Spread 8 pixels across the whole gradient.
/// for pixel in 0..8u8 {
///     let _color = SUNSET.color_at(pixel * 36);
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Gradient<const N: usize> {
    stops: [(u8, Rgb); N],
}

impl<const N: usize> Gradient<N> {
    /// Makes a gradient from color stops, listed from lowest position to highest.
    ///
    /// Two neighbouring stops may share a position, which makes a hard edge
    /// (the later stop wins at that exact position).
    ///
    /// # Panics
    ///
    /// Panics if there are no stops, or if the positions are not in ascending
    /// order. When used to build a `const`, this is a compile error instead.
    pub const fn new(stops: [(u8, Rgb); N]) -> Self {
        assert!(N > 0, "a Gradient needs at least one stop");
        let mut i = 1;
        while i < N {
            assert!(
                stops[i - 1].0 <= stops[i].0,
                "Gradient stop positions must be in ascending order"
            );
            i += 1;
        }
        Self { stops }
    }

    /// Returns the color at `position` (0-255) along the gradient.
    pub const fn color_at(&self, position: u8) -> Rgb {
        // Find the last stop at or before `position`.
        let mut i = 0;
        while i < N && self.stops[i].0 <= position {
            i += 1;
        }
        if i == 0 {
            return self.stops[0].1;
        }
        if i == N {
            return self.stops[N - 1].1;
        }
        let (p0, c0) = self.stops[i - 1];
        let (p1, c1) = self.stops[i];
        // p1 > position >= p0, so the span is never zero.
        let span = (p1 - p0) as u16;
        let into = (position - p0) as u16;
        #[allow(clippy::cast_possible_truncation)] // into < span, so the result is at most 255
        let t = (into * 255 / span) as u8;
        c0.lerp(c1, t)
    }
}

/// A ring of evenly spaced colors that wraps around smoothly.
///
/// Position 0 is the first color, and as the position climbs towards 255 the
/// palette blends through each color and then back towards the first one.
///
/// # Examples
///
/// ```
/// use ledkit_color::{Palette, Rgb};
///
/// let palette = Palette::new([Rgb::RED, Rgb::GREEN, Rgb::BLUE]);
/// // Cycle the palette along a strip of 10 pixels.
/// for pixel in 0..10u8 {
///     let position = pixel * 25;
///     let _color = palette.color_at(position);
/// }
/// assert_eq!(palette.color_at(0), Rgb::RED);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Palette<const N: usize> {
    colors: [Rgb; N],
}

impl<const N: usize> Palette<N> {
    /// Makes a palette from a list of colors.
    ///
    /// # Panics
    ///
    /// Panics if the list is empty. When used to build a `const`, this is a
    /// compile error instead.
    pub const fn new(colors: [Rgb; N]) -> Self {
        assert!(N > 0, "a Palette needs at least one color");
        Self { colors }
    }

    /// Returns the color at `position` (0-255) around the palette's circle.
    pub const fn color_at(&self, position: u8) -> Rgb {
        #[allow(clippy::cast_possible_truncation)] // palettes are far smaller than u32::MAX
        let scaled = position as u32 * N as u32;
        let i = (scaled >> 8) as usize;
        #[allow(clippy::cast_possible_truncation)] // masked to 8 bits
        let t = (scaled & 0xFF) as u8;
        self.colors[i].lerp(self.colors[(i + 1) % N], t)
    }
}

impl Palette<6> {
    /// The classic rainbow: red, orange, yellow, green, blue, purple.
    pub const RAINBOW: Self = Self {
        colors: [
            Rgb::RED,
            Rgb::ORANGE,
            Rgb::YELLOW,
            Rgb::GREEN,
            Rgb::BLUE,
            Rgb::PURPLE,
        ],
    };
}

/// Anything that gives a color for a position from 0 to 255: a [`Gradient`],
/// a [`Palette`], or one of the ready-made [`palettes`](crate::palettes).
///
/// Animations take a color map so you can swap the colors of an effect without
/// changing the effect. `wraps` says whether the end joins back to the start
/// smoothly (a [`Palette`] does, a [`Gradient`] does not), so an effect that
/// scrolls through the colors knows whether it can go round and round or should
/// go back and forth instead.
///
/// # Example
///
/// ```
/// use ledkit_color::{palettes, ColorMap, Rgb};
///
/// fn first_color(map: &dyn ColorMap) -> Rgb {
///     map.color_at(0)
/// }
///
/// assert_eq!(first_color(&palettes::FIRE), Rgb::BLACK);
/// assert!(palettes::RAINBOW.wraps());
/// assert!(!palettes::JET.wraps());
/// ```
pub trait ColorMap {
    /// The color at `position`, from 0 (the start) to 255 (the end).
    fn color_at(&self, position: u8) -> Rgb;

    /// True if position 255 blends smoothly back into position 0.
    fn wraps(&self) -> bool {
        false
    }
}

impl<const N: usize> ColorMap for Gradient<N> {
    fn color_at(&self, position: u8) -> Rgb {
        Gradient::color_at(self, position)
    }
}

impl<const N: usize> ColorMap for Palette<N> {
    fn color_at(&self, position: u8) -> Rgb {
        Palette::color_at(self, position)
    }

    fn wraps(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Rgb;

    #[test]
    fn gradient_endpoints_and_clamping() {
        let g = Gradient::new([(64, Rgb::RED), (192, Rgb::BLUE)]);
        assert_eq!(g.color_at(0), Rgb::RED);
        assert_eq!(g.color_at(64), Rgb::RED);
        assert_eq!(g.color_at(192), Rgb::BLUE);
        assert_eq!(g.color_at(255), Rgb::BLUE);
    }
    #[test]
    fn gradient_midpoint() {
        let g = Gradient::new([(0, Rgb::BLACK), (255, Rgb::WHITE)]);
        assert_eq!(g.color_at(128), Rgb::new(128, 128, 128));
    }
    #[test]
    fn gradient_three_stops() {
        let g = Gradient::new([(0, Rgb::RED), (128, Rgb::GREEN), (255, Rgb::BLUE)]);
        assert_eq!(g.color_at(128), Rgb::GREEN);
    }
    #[test]
    fn single_stop_gradient_is_solid() {
        let g = Gradient::new([(100, Rgb::GOLD)]);
        assert_eq!(g.color_at(0), Rgb::GOLD);
        assert_eq!(g.color_at(255), Rgb::GOLD);
    }
    #[test]
    fn equal_positions_later_stop_wins() {
        let g = Gradient::new([(0, Rgb::RED), (100, Rgb::GREEN), (100, Rgb::BLUE)]);
        assert_eq!(g.color_at(100), Rgb::BLUE);
    }
    #[test]
    fn palette_hits_each_color() {
        let p = Palette::new([Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE]);
        assert_eq!(p.color_at(0), Rgb::RED);
        assert_eq!(p.color_at(64), Rgb::GREEN);
        assert_eq!(p.color_at(128), Rgb::BLUE);
        assert_eq!(p.color_at(192), Rgb::WHITE);
    }
    #[test]
    fn palette_wraps_back_toward_first_color() {
        let p = Palette::new([Rgb::RED, Rgb::BLUE]);
        let near_end = p.color_at(255);
        assert!(
            near_end.r > 200 && near_end.b < 10,
            "should be almost back to red: {near_end:?}"
        );
    }
    #[test]
    fn color_map_matches_inherent() {
        let g = Gradient::new([(0, Rgb::RED), (255, Rgb::BLUE)]);
        let p = Palette::new([Rgb::RED, Rgb::GREEN]);
        for i in [0u8, 1, 77, 128, 254, 255] {
            assert_eq!(ColorMap::color_at(&g, i), g.color_at(i));
            assert_eq!(ColorMap::color_at(&p, i), p.color_at(i));
        }
        assert!(!ColorMap::wraps(&g));
        assert!(ColorMap::wraps(&p));
    }
    #[test]
    fn rainbow_palette_starts_red() {
        assert_eq!(Palette::RAINBOW.color_at(0), Rgb::RED);
    }
}
