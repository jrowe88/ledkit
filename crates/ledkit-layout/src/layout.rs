//! The `Layout` trait and `MatrixLayout`.

/// Maps `(x, y)` positions on a grid of LEDs onto positions along the strip.
///
/// `(0, 0)` is the top-left of the picture as you look at the panel; `x`
/// grows to the right and `y` grows down.
///
/// Implementations must map the grid one-to-one onto `0..len()`: every
/// `(x, y)` inside the grid gives a different index, and together they give
/// every number from `0` up to (not including) [`len`](Layout::len).
///
/// # Example
///
/// ```
/// use ledkit_layout::{Layout, MatrixLayout};
///
/// fn corner_count(layout: &impl Layout) -> usize {
///     [(0, 0), (layout.width() - 1, layout.height() - 1)]
///         .into_iter()
///         .filter(|&(x, y)| layout.index(x, y).is_some())
///         .count()
/// }
///
/// assert_eq!(corner_count(&MatrixLayout::new(4, 3)), 2);
/// ```
pub trait Layout {
    /// How many LEDs wide the grid is.
    fn width(&self) -> u16;

    /// How many LEDs tall the grid is.
    fn height(&self) -> u16;

    /// The strip position of the LED at `(x, y)`, or `None` if `(x, y)` is
    /// outside the grid.
    fn index(&self, x: u16, y: u16) -> Option<usize>;

    /// How many LEDs the grid has: `width * height`.
    fn len(&self) -> usize {
        usize::from(self.width()) * usize::from(self.height())
    }

    /// True if the grid has no LEDs.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The corner of the panel where the data wire goes in (LED 0).
///
/// # Example
///
/// ```
/// use ledkit_layout::{Corner, MatrixLayout};
///
/// let layout = MatrixLayout::new(4, 3).starting_at(Corner::BottomRight);
/// assert_eq!(layout.index(3, 2), Some(0));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Corner {
    /// Top-left, as you look at the panel. The default.
    TopLeft,
    /// Top-right.
    TopRight,
    /// Bottom-left.
    BottomLeft,
    /// Bottom-right.
    BottomRight,
}

/// How each line of the chain continues into the next.
///
/// # Example
///
/// ```
/// use ledkit_layout::{MatrixLayout, Wiring};
///
/// assert_eq!(MatrixLayout::new(4, 3).serpentine().wiring(), Wiring::Serpentine);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Wiring {
    /// Every line runs the same way, so the chain jumps back to the start
    /// side at the end of each line. The default.
    Progressive,
    /// Lines alternate direction, so the chain zig-zags and never jumps.
    Serpentine,
}

/// Whether the chain runs along rows or down columns.
///
/// # Example
///
/// ```
/// use ledkit_layout::{MatrixLayout, Order};
///
/// assert_eq!(MatrixLayout::new(4, 3).columns_first().order(), Order::ColumnsFirst);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Order {
    /// The chain fills a row, then moves to the next row. The default.
    RowsFirst,
    /// The chain fills a column, then moves to the next column.
    ColumnsFirst,
}

/// A rectangular LED matrix wired as one chain.
///
/// A new layout is progressive, rows first, starting at the top-left. Chain
/// [`serpentine`](Self::serpentine), [`columns_first`](Self::columns_first)
/// and [`starting_at`](Self::starting_at) to match your panel. Every method
/// is a `const fn`, so a layout can be a `const`.
///
/// # Example
///
/// ```
/// use ledkit_layout::{Corner, Layout, MatrixLayout};
///
/// const LAYOUT: MatrixLayout = MatrixLayout::new(16, 16).serpentine();
/// assert_eq!(LAYOUT.len(), 256);
///
/// let panel = MatrixLayout::new(8, 32)
///     .columns_first()
///     .serpentine()
///     .starting_at(Corner::BottomLeft);
/// assert_eq!(panel.index(0, 31), Some(0));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MatrixLayout {
    width: u16,
    height: u16,
    wiring: Wiring,
    order: Order,
    corner: Corner,
}

impl MatrixLayout {
    /// A `width` by `height` matrix: progressive, rows first, starting at
    /// the top-left.
    ///
    /// # Panics
    ///
    /// Panics if `width` or `height` is zero. In a `const` that is a build
    /// error.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Layout, MatrixLayout};
    ///
    /// let layout = MatrixLayout::new(4, 3);
    /// assert_eq!((layout.width(), layout.height()), (4, 3));
    /// ```
    #[must_use]
    pub const fn new(width: u16, height: u16) -> Self {
        assert!(
            width > 0 && height > 0,
            "a matrix needs at least one LED in each direction"
        );
        Self {
            width,
            height,
            wiring: Wiring::Progressive,
            order: Order::RowsFirst,
            corner: Corner::TopLeft,
        }
    }

    /// Lines alternate direction (a zig-zag chain).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// // Row 1 runs right to left.
    /// assert_eq!(MatrixLayout::new(4, 3).serpentine().index(3, 1), Some(4));
    /// ```
    #[must_use]
    pub const fn serpentine(mut self) -> Self {
        self.wiring = Wiring::Serpentine;
        self
    }

    /// Every line runs the same way. This is the default.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// let layout = MatrixLayout::new(4, 3).serpentine().progressive();
    /// assert_eq!(layout.index(0, 1), Some(4));
    /// ```
    #[must_use]
    pub const fn progressive(mut self) -> Self {
        self.wiring = Wiring::Progressive;
        self
    }

    /// The chain runs down columns instead of along rows.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// assert_eq!(MatrixLayout::new(4, 3).columns_first().index(0, 2), Some(2));
    /// ```
    #[must_use]
    pub const fn columns_first(mut self) -> Self {
        self.order = Order::ColumnsFirst;
        self
    }

    /// The chain runs along rows. This is the default.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// let layout = MatrixLayout::new(4, 3).columns_first().rows_first();
    /// assert_eq!(layout.index(3, 0), Some(3));
    /// ```
    #[must_use]
    pub const fn rows_first(mut self) -> Self {
        self.order = Order::RowsFirst;
        self
    }

    /// Puts LED 0 (where the data wire goes in) at `corner`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{Corner, MatrixLayout};
    ///
    /// let layout = MatrixLayout::new(4, 3).starting_at(Corner::TopRight);
    /// assert_eq!(layout.index(3, 0), Some(0));
    /// ```
    #[must_use]
    pub const fn starting_at(mut self, corner: Corner) -> Self {
        self.corner = corner;
        self
    }

    /// How many LEDs wide the matrix is. The same as [`Layout::width`], but
    /// usable in a `const`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// const WIDTH: u16 = MatrixLayout::new(8, 32).width();
    /// assert_eq!(WIDTH, 8);
    /// ```
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    /// How many LEDs tall the matrix is. The same as [`Layout::height`], but
    /// usable in a `const`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// const HEIGHT: u16 = MatrixLayout::new(8, 32).height();
    /// assert_eq!(HEIGHT, 32);
    /// ```
    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    /// The wiring set so far: [`Wiring::Progressive`] (the default) or
    /// [`Wiring::Serpentine`]. Usable in a `const`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{MatrixLayout, Wiring};
    ///
    /// const WIRING: Wiring = MatrixLayout::new(8, 8).serpentine().wiring();
    /// assert_eq!(WIRING, Wiring::Serpentine);
    /// ```
    #[must_use]
    pub const fn wiring(&self) -> Wiring {
        self.wiring
    }

    /// Whether the chain runs along rows or down columns: [`Order::RowsFirst`]
    /// (the default) or [`Order::ColumnsFirst`]. Usable in a `const`, so you can
    /// size things from it.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::{MatrixLayout, Order};
    ///
    /// const LAYOUT: MatrixLayout = MatrixLayout::new(8, 32).columns_first();
    /// // How many LEDs make one line of the chain.
    /// const LINE: u16 = match LAYOUT.order() {
    ///     Order::RowsFirst => LAYOUT.width(),
    ///     Order::ColumnsFirst => LAYOUT.height(),
    /// };
    /// assert_eq!(LINE, 32);
    /// ```
    #[must_use]
    pub const fn order(&self) -> Order {
        self.order
    }

    /// The corner LED 0 sits in.
    #[must_use]
    pub const fn corner(&self) -> Corner {
        self.corner
    }

    /// The strip position of the LED at `(x, y)`, or `None` outside the
    /// matrix. The same as [`Layout::index`], but usable in a `const`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_layout::MatrixLayout;
    ///
    /// const CENTER: Option<usize> = MatrixLayout::new(3, 3).index(1, 1);
    /// assert_eq!(CENTER, Some(4));
    /// ```
    #[must_use]
    pub const fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        // Measure from the corner the chain starts in.
        let from_left = match self.corner {
            Corner::TopLeft | Corner::BottomLeft => x,
            Corner::TopRight | Corner::BottomRight => self.width - 1 - x,
        };
        let from_top = match self.corner {
            Corner::TopLeft | Corner::TopRight => y,
            Corner::BottomLeft | Corner::BottomRight => self.height - 1 - y,
        };
        // A "line" is a row or a column; `along` is the place within it.
        let (line, along, line_len) = match self.order {
            Order::RowsFirst => (from_top, from_left, self.width),
            Order::ColumnsFirst => (from_left, from_top, self.height),
        };
        let reversed = matches!(self.wiring, Wiring::Serpentine) && line % 2 == 1;
        let along = if reversed {
            line_len - 1 - along
        } else {
            along
        };
        Some(line as usize * line_len as usize + along as usize)
    }
}

impl Layout for MatrixLayout {
    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        MatrixLayout::index(self, x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::assert_bijection;

    #[test]
    fn getters_report_the_builder_settings() {
        let default = MatrixLayout::new(4, 3);
        assert_eq!(default.wiring(), Wiring::Progressive);
        assert_eq!(default.order(), Order::RowsFirst);
        let m = default.serpentine().columns_first();
        assert_eq!(m.wiring(), Wiring::Serpentine);
        assert_eq!(m.order(), Order::ColumnsFirst);
        assert_eq!(m.rows_first().order(), Order::RowsFirst);
    }

    #[test]
    fn progressive_rows_from_top_left() {
        let m = MatrixLayout::new(4, 3);
        assert_eq!(m.index(0, 0), Some(0));
        assert_eq!(m.index(3, 0), Some(3));
        assert_eq!(m.index(0, 1), Some(4));
        assert_eq!(m.index(4, 0), None);
        assert_eq!(m.index(0, 3), None);
    }

    #[test]
    fn serpentine_rows_reverse_every_other_row() {
        let m = MatrixLayout::new(4, 3).serpentine();
        assert_eq!(m.index(3, 1), Some(4)); // row 1 runs right to left
        assert_eq!(m.index(0, 1), Some(7));
        assert_eq!(m.index(0, 2), Some(8));
    }

    #[test]
    fn columns_first_from_bottom_right() {
        let m = MatrixLayout::new(4, 3)
            .columns_first()
            .starting_at(Corner::BottomRight);
        assert_eq!(m.index(3, 2), Some(0));
        assert_eq!(m.index(3, 0), Some(2));
        assert_eq!(m.index(2, 2), Some(3));
    }

    #[test]
    fn columns_first_serpentine_from_bottom_right_explicit_values() {
        let m = MatrixLayout::new(4, 3)
            .columns_first()
            .serpentine()
            .starting_at(Corner::BottomRight);
        assert_eq!(m.index(3, 2), Some(0));
        assert_eq!(m.index(3, 1), Some(1));
        assert_eq!(m.index(3, 0), Some(2));
        assert_eq!(m.index(2, 0), Some(3));
        assert_eq!(m.index(2, 1), Some(4));
        assert_eq!(m.index(2, 2), Some(5));
    }

    #[test]
    fn layout_trait_delegates_and_counts() {
        let m = MatrixLayout::new(8, 32);
        assert_eq!(Layout::len(&m), 256);
        assert!(!Layout::is_empty(&m));
        assert_eq!(Layout::index(&m, 7, 31), Some(255));
    }

    #[test]
    #[should_panic(expected = "at least one LED")]
    fn zero_size_panics() {
        let _ = MatrixLayout::new(0, 4);
    }

    /// Review Focus 3: all 16 combinations on 4x3 and 8x32 are one-to-one onto 0..len.
    #[test]
    fn every_wiring_is_a_bijection() {
        let wirings = [Wiring::Progressive, Wiring::Serpentine];
        let orders = [Order::RowsFirst, Order::ColumnsFirst];
        let corners = [
            Corner::TopLeft,
            Corner::TopRight,
            Corner::BottomLeft,
            Corner::BottomRight,
        ];
        for (w, h) in [(4, 3), (8, 32)] {
            for wiring in wirings {
                for order in orders {
                    for corner in corners {
                        let mut m = MatrixLayout::new(w, h).starting_at(corner);
                        m = if wiring == Wiring::Serpentine {
                            m.serpentine()
                        } else {
                            m.progressive()
                        };
                        m = if order == Order::ColumnsFirst {
                            m.columns_first()
                        } else {
                            m.rows_first()
                        };
                        assert_bijection(
                            w,
                            h,
                            |x, y| m.index(x, y),
                            &format!("{w}x{h} {wiring:?} {order:?} {corner:?}"),
                        );
                    }
                }
            }
        }
    }
}
