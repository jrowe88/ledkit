//! Rotating a layout and tiling panels into a bigger one.

use crate::layout::{Corner, Layout, MatrixLayout};

/// How far to turn a layout, clockwise, as you look at the panel.
///
/// # Example
///
/// ```
/// use ledkit_layout::{Layout, MatrixLayout, Rotation};
///
/// let sideways = MatrixLayout::new(8, 32).rotated(Rotation::Quarter);
/// assert_eq!((sideways.width(), sideways.height()), (32, 8));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rotation {
    /// Not turned.
    None,
    /// A quarter turn clockwise (90 degrees). Width and height swap.
    Quarter,
    /// A half turn (180 degrees), upside down.
    Half,
    /// Three quarter turns clockwise (270 degrees, or a quarter turn the
    /// other way). Width and height swap.
    ThreeQuarter,
}

/// Another layout, turned clockwise.
///
/// Use it when a panel is mounted sideways or upside down: the picture keeps
/// its `(0, 0)` at the top-left of what you see. A quarter or three-quarter
/// turn swaps the width and height.
///
/// # Example
///
/// ```
/// use ledkit_layout::{Layout, MatrixLayout, Rotated, Rotation};
///
/// // A tall 8x32 panel lying on its side is a wide 32x8 picture.
/// let panel = MatrixLayout::new(8, 32);
/// let turned = Rotated::new(panel, Rotation::Quarter);
/// assert_eq!((turned.width(), turned.height()), (32, 8));
/// // The picture's top-left is the panel's bottom-left.
/// assert_eq!(turned.index(0, 0), panel.index(0, 31));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rotated<L: Layout> {
    inner: L,
    rotation: Rotation,
}

impl<L: Layout> Rotated<L> {
    /// Turns `inner` clockwise by `rotation`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Rotated, Rotation};
    ///
    /// let upside_down = Rotated::new(MatrixLayout::new(4, 3), Rotation::Half);
    /// assert_eq!(upside_down.index(0, 0), MatrixLayout::new(4, 3).index(3, 2));
    /// ```
    #[must_use]
    pub const fn new(inner: L, rotation: Rotation) -> Self {
        Self { inner, rotation }
    }
}

impl<L: Layout> Layout for Rotated<L> {
    fn width(&self) -> u16 {
        match self.rotation {
            Rotation::None | Rotation::Half => self.inner.width(),
            Rotation::Quarter | Rotation::ThreeQuarter => self.inner.height(),
        }
    }

    fn height(&self) -> u16 {
        match self.rotation {
            Rotation::None | Rotation::Half => self.inner.height(),
            Rotation::Quarter | Rotation::ThreeQuarter => self.inner.width(),
        }
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width() || y >= self.height() {
            return None;
        }
        let inner_width = self.inner.width();
        let inner_height = self.inner.height();
        let (inner_x, inner_y) = match self.rotation {
            Rotation::None => (x, y),
            Rotation::Quarter => (y, inner_height - 1 - x),
            Rotation::Half => (inner_width - 1 - x, inner_height - 1 - y),
            Rotation::ThreeQuarter => (inner_width - 1 - y, x),
        };
        self.inner.index(inner_x, inner_y)
    }
}

impl MatrixLayout {
    /// This matrix, turned clockwise by `rotation`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Rotation};
    ///
    /// let turned = MatrixLayout::new(8, 32).rotated(Rotation::ThreeQuarter);
    /// assert_eq!((turned.width(), turned.height()), (32, 8));
    /// ```
    #[must_use]
    pub const fn rotated(self, rotation: Rotation) -> Rotated<Self> {
        Rotated::new(self, rotation)
    }
}

/// Several identical panels joined into one bigger picture.
///
/// The panels' data wires are chained one after another: panel 1's LEDs
/// follow panel 0's, and so on. A new `Tiled` runs across each row of panels,
/// starting at the top-left panel. Chain [`serpentine`](Self::serpentine),
/// [`columns_first`](Self::columns_first) and
/// [`starting_at`](Self::starting_at) to describe how the *panels* are
/// chained; each panel keeps its own wiring from the `MatrixLayout` you pass
/// in.
///
/// # Example
///
/// ```
/// use ledkit_layout::{Layout, MatrixLayout, Tiled};
///
/// // Two 4x3 panels side by side make an 8x3 picture.
/// let wide = Tiled::new(MatrixLayout::new(4, 3), 2, 1);
/// assert_eq!((wide.width(), wide.height()), (8, 3));
/// // The first LED of the second panel comes right after the first panel's 12.
/// assert_eq!(wide.index(4, 0), Some(12));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tiled {
    panel: MatrixLayout,
    panel_order: MatrixLayout,
}

impl Tiled {
    /// `across` by `down` copies of `panel`, chained left to right, top to
    /// bottom, starting at the top-left.
    ///
    /// # Panics
    ///
    /// Panics if `across` or `down` is zero, or if the whole picture would be
    /// wider or taller than 65535 LEDs.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Tiled};
    ///
    /// let wall = Tiled::new(MatrixLayout::new(8, 8), 4, 2);
    /// assert_eq!((wall.width(), wall.height()), (32, 16));
    /// assert_eq!(wall.len(), 512);
    /// ```
    #[must_use]
    pub const fn new(panel: MatrixLayout, across: u16, down: u16) -> Self {
        assert!(
            across > 0 && down > 0,
            "a tiled picture needs at least one panel each way"
        );
        assert!(
            panel.width() as u32 * across as u32 <= u16::MAX as u32
                && panel.height() as u32 * down as u32 <= u16::MAX as u32,
            "the tiled picture is too big"
        );
        Self {
            panel,
            panel_order: MatrixLayout::new(across, down),
        }
    }

    /// The panels' chain zig-zags: every other line of panels runs the
    /// other way.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Tiled};
    ///
    /// // Panel row 1 runs right to left, so its right panel is panel 2.
    /// let wall = Tiled::new(MatrixLayout::new(2, 2), 2, 2).serpentine();
    /// assert_eq!(wall.index(2, 2), Some(8));
    /// ```
    #[must_use]
    pub const fn serpentine(mut self) -> Self {
        self.panel_order = self.panel_order.serpentine();
        self
    }

    /// Every line of panels runs the same way. This is the default.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Tiled};
    ///
    /// let wall = Tiled::new(MatrixLayout::new(2, 2), 2, 2)
    ///     .serpentine()
    ///     .progressive();
    /// assert_eq!(wall.index(0, 2), Some(8));
    /// ```
    #[must_use]
    pub const fn progressive(mut self) -> Self {
        self.panel_order = self.panel_order.progressive();
        self
    }

    /// The panels are chained down columns instead of along rows.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Tiled};
    ///
    /// let wall = Tiled::new(MatrixLayout::new(2, 2), 2, 2).columns_first();
    /// // The panel under the first one is panel 1.
    /// assert_eq!(wall.index(0, 2), Some(4));
    /// ```
    #[must_use]
    pub const fn columns_first(mut self) -> Self {
        self.panel_order = self.panel_order.columns_first();
        self
    }

    /// The panels are chained along rows. This is the default.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout, Tiled};
    ///
    /// let wall = Tiled::new(MatrixLayout::new(2, 2), 2, 2)
    ///     .columns_first()
    ///     .rows_first();
    /// assert_eq!(wall.index(2, 0), Some(4));
    /// ```
    #[must_use]
    pub const fn rows_first(mut self) -> Self {
        self.panel_order = self.panel_order.rows_first();
        self
    }

    /// The chain starts at the panel in `corner`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Corner, Layout, MatrixLayout, Tiled};
    ///
    /// let wall = Tiled::new(MatrixLayout::new(2, 2), 2, 1).starting_at(Corner::TopRight);
    /// // The right-hand panel is first in the chain.
    /// assert_eq!(wall.index(2, 0), Some(0));
    /// ```
    #[must_use]
    pub const fn starting_at(mut self, corner: Corner) -> Self {
        self.panel_order = self.panel_order.starting_at(corner);
        self
    }

    /// The strip position of the LED at `(x, y)`, or `None` outside the
    /// picture. The same as [`Layout::index`], but usable in a `const`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{MatrixLayout, Tiled};
    ///
    /// const SECOND_PANEL: Option<usize> = Tiled::new(MatrixLayout::new(4, 3), 2, 1).index(4, 0);
    /// assert_eq!(SECOND_PANEL, Some(12));
    /// ```
    #[must_use]
    pub const fn index(&self, x: u16, y: u16) -> Option<usize> {
        let panel_width = self.panel.width();
        let panel_height = self.panel.height();
        let Some(panel_number) = self.panel_order.index(x / panel_width, y / panel_height) else {
            return None;
        };
        // Inside the panel, the panel's own wiring decides.
        match self.panel.index(x % panel_width, y % panel_height) {
            Some(inside) => {
                Some(panel_number * (panel_width as usize * panel_height as usize) + inside)
            }
            None => None,
        }
    }
}

impl Layout for Tiled {
    fn width(&self) -> u16 {
        self.panel.width() * self.panel_order.width()
    }

    fn height(&self) -> u16 {
        self.panel.height() * self.panel_order.height()
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        Tiled::index(self, x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Order, Wiring};
    use crate::test_support::assert_bijection;

    #[test]
    fn quarter_turn_swaps_size_and_moves_origin() {
        let r = MatrixLayout::new(8, 32).rotated(Rotation::Quarter);
        assert_eq!((r.width(), r.height()), (32, 8));
        // After a clockwise quarter turn, the picture's top-left is the panel's bottom-left.
        assert_eq!(r.index(0, 0), MatrixLayout::new(8, 32).index(0, 31));
        // One step right in the picture is one step up the panel's left edge.
        assert_eq!(r.index(1, 0), MatrixLayout::new(8, 32).index(0, 30));
    }

    #[test]
    fn half_turn_maps_corners() {
        let m = MatrixLayout::new(4, 3);
        let r = m.rotated(Rotation::Half);
        assert_eq!(r.index(0, 0), m.index(3, 2));
        let tall = MatrixLayout::new(8, 32);
        let r = tall.rotated(Rotation::Half);
        assert_eq!(r.index(0, 0), tall.index(7, 31));
        // One step right in the picture is one step left along the panel's bottom row.
        assert_eq!(r.index(1, 0), tall.index(6, 31));
    }

    #[test]
    fn three_quarter_turn_moves_origin_to_top_right() {
        let m = MatrixLayout::new(8, 32);
        let r = m.rotated(Rotation::ThreeQuarter);
        assert_eq!((r.width(), r.height()), (32, 8));
        assert_eq!(r.index(0, 0), m.index(7, 0));
        // One step right in the picture is one step down the panel's right edge.
        assert_eq!(r.index(1, 0), m.index(7, 1));
    }

    #[test]
    #[should_panic(expected = "at least one panel")]
    fn tiled_with_zero_panels_across_panics() {
        let _ = Tiled::new(MatrixLayout::new(4, 3), 0, 1);
    }

    #[test]
    #[should_panic(expected = "at least one panel")]
    fn tiled_with_zero_panels_down_panics() {
        let _ = Tiled::new(MatrixLayout::new(4, 3), 1, 0);
    }

    #[test]
    #[should_panic(expected = "too big")]
    fn tiled_wider_than_u16_panics() {
        let _ = Tiled::new(MatrixLayout::new(1000, 1), 100, 1);
    }

    #[test]
    #[should_panic(expected = "too big")]
    fn tiled_taller_than_u16_panics() {
        let _ = Tiled::new(MatrixLayout::new(1, 1000), 1, 100);
    }

    #[test]
    fn no_rotation_is_the_same_layout() {
        let m = MatrixLayout::new(4, 3).serpentine();
        let r = m.rotated(Rotation::None);
        for y in 0..3 {
            for x in 0..4 {
                assert_eq!(r.index(x, y), m.index(x, y));
            }
        }
    }

    /// Review Focus 3 + 4: every rotation of 8x32, and tiled 2x2 of 4x3 under every
    /// panel-order choice, are one-to-one onto 0..len.
    #[test]
    fn transforms_are_bijections() {
        let rotations = [
            Rotation::None,
            Rotation::Quarter,
            Rotation::Half,
            Rotation::ThreeQuarter,
        ];
        let panel = MatrixLayout::new(8, 32).columns_first().serpentine();
        for rotation in rotations {
            let r = panel.rotated(rotation);
            assert_bijection(
                r.width(),
                r.height(),
                |x, y| r.index(x, y),
                &format!("8x32 {rotation:?}"),
            );
        }

        let corners = [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ];
        for wiring in [Wiring::Progressive, Wiring::Serpentine] {
            for order in [Order::RowsFirst, Order::ColumnsFirst] {
                for corner in corners {
                    let mut t =
                        Tiled::new(MatrixLayout::new(4, 3).serpentine(), 2, 2).starting_at(corner);
                    t = if wiring == Wiring::Serpentine {
                        t.serpentine()
                    } else {
                        t.progressive()
                    };
                    t = if order == Order::ColumnsFirst {
                        t.columns_first()
                    } else {
                        t.rows_first()
                    };
                    assert_bijection(
                        t.width(),
                        t.height(),
                        |x, y| t.index(x, y),
                        &format!("tiled {wiring:?} {order:?} {corner:?}"),
                    );
                }
            }
        }
    }

    /// A grid that is not square (3 panels across, 2 down) is still one-to-one
    /// under every panel-order choice.
    #[test]
    fn non_square_tiled_grid_is_a_bijection() {
        let corners = [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ];
        for wiring in [Wiring::Progressive, Wiring::Serpentine] {
            for order in [Order::RowsFirst, Order::ColumnsFirst] {
                for corner in corners {
                    let mut t =
                        Tiled::new(MatrixLayout::new(4, 3).serpentine(), 3, 2).starting_at(corner);
                    t = if wiring == Wiring::Serpentine {
                        t.serpentine()
                    } else {
                        t.progressive()
                    };
                    t = if order == Order::ColumnsFirst {
                        t.columns_first()
                    } else {
                        t.rows_first()
                    };
                    assert_eq!((t.width(), t.height()), (12, 6));
                    assert_bijection(
                        t.width(),
                        t.height(),
                        |x, y| t.index(x, y),
                        &format!("tiled 3x2 {wiring:?} {order:?} {corner:?}"),
                    );
                }
            }
        }
    }

    #[test]
    fn tiled_panels_follow_each_other() {
        let t = Tiled::new(MatrixLayout::new(4, 3), 2, 1);
        assert_eq!((t.width(), t.height()), (8, 3));
        assert_eq!(t.index(4, 0), Some(12)); // first pixel of the second panel
    }

    #[test]
    fn tiled_panels_keep_their_own_wiring() {
        let t = Tiled::new(MatrixLayout::new(4, 3).serpentine(), 2, 1);
        // Second panel, row 1 runs right to left: (7, 1) is its 5th LED.
        assert_eq!(t.index(7, 1), Some(12 + 4));
        assert_eq!(t.index(8, 0), None);
    }
}
