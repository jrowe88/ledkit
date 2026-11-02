//! [`TextArea`]: still text in a box on the panel, with [`Across`] and
//! [`UpDown`] to line it up.

use ledkit_anim::{Surface, Time};

use crate::style::Style;
use crate::{Font, Paint};

/// Where text sits across its box: against the left edge, in the centre, or
/// against the right edge.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// let area = TextArea::new(Font::X11_5X8).align(Across::Left, UpDown::Top);
/// assert_eq!(Across::default(), Across::Centre);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Across {
    /// Against the left edge of the box.
    Left,
    /// In the middle (the default).
    #[default]
    Centre,
    /// Against the right edge of the box.
    Right,
}

/// Where text sits up and down in its box: at the top, in the middle, or at
/// the bottom.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// let area = TextArea::new(Font::X11_5X8).align(Across::Centre, UpDown::Bottom);
/// assert_eq!(UpDown::default(), UpDown::Middle);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum UpDown {
    /// Against the top edge of the box.
    Top,
    /// In the middle (the default).
    #[default]
    Middle,
    /// Against the bottom edge of the box.
    Bottom,
}

/// Text that stays still, drawn in a box on your panel: a clock, a score, a
/// label.
///
/// Set it up once with the builder methods, then each time round your loop
/// change the text with [`set_text`](TextArea::set_text) whenever you like and
/// draw it with [`render`](TextArea::render). It only touches LEDs inside its
/// box, so two areas stacked one above the other make two lines of text, and
/// anything you drew outside the box stays as it was.
///
/// Text that doesn't fit is cut off at the edges of the box. The area keeps
/// its own copy of the text, up to 32 characters (or `LEN`, see
/// [`with_capacity`](TextArea::with_capacity)).
///
/// ```
/// use ledkit_anim::Time;
/// use ledkit_layout::{Canvas, MatrixLayout};
/// use ledkit_text::prelude::*;
///
/// let mut pixels = [Rgb::BLACK; 32 * 8];
/// let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(32, 8)).unwrap();
///
/// let mut score = TextArea::new(Font::X11_5X8)
///     .letters(Rgb::YELLOW)
///     .outline(Rgb::RED);
/// score.set_text("12-7");
/// score.render(&mut panel, Time::from_millis(0));
/// ```
#[derive(Copy, Clone)]
#[must_use]
pub struct TextArea<const LEN: usize = 32> {
    style: Style,
    /// False until `at` is called: then the box is the whole surface.
    placed: bool,
    across: Across,
    up_down: UpDown,
    text: [u8; LEN],
    len: usize,
}

impl TextArea {
    /// A text area with room for 32 characters, with white letters, centred
    /// both ways. Until you call [`at`](TextArea::at), it covers the whole
    /// surface it is drawn on.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let area = TextArea::new(Font::X11_5X8);
    /// ```
    pub const fn new(font: Font) -> TextArea {
        TextArea::with_capacity(font)
    }
}

impl<const LEN: usize> TextArea<LEN> {
    /// A text area with room for `LEN` characters. Use it when 32 is more than
    /// you need (to save memory) or not enough.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Just enough for "12:34".
    /// let clock = TextArea::<5>::with_capacity(Font::seven_segment(8));
    /// ```
    pub const fn with_capacity(font: Font) -> Self {
        Self {
            style: Style::new(font),
            placed: false,
            across: Across::Centre,
            up_down: UpDown::Middle,
            text: [0; LEN],
            len: 0,
        }
    }

    /// Puts the box on the panel: its top-left LED at (`left`, `top`), and
    /// how many LEDs wide and tall it is. The box may hang off the edge of the
    /// panel; only the LEDs that exist are drawn.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // The bottom half of a 64x32 panel.
    /// let area = TextArea::new(Font::X11_6X10).at(0, 16, 64, 16);
    /// ```
    pub const fn at(mut self, left: i32, top: i32, width: u16, height: u16) -> Self {
        self.style.left = left;
        self.style.top = top;
        self.style.width = width;
        self.style.height = height;
        self.placed = true;
        self
    }

    /// Where the text sits in the box, across and up-and-down. Centred both
    /// ways unless you say otherwise.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let label = TextArea::new(Font::X11_5X8).align(Across::Left, UpDown::Top);
    /// ```
    pub const fn align(mut self, across: Across, up_down: UpDown) -> Self {
        self.across = across;
        self.up_down = up_down;
        self
    }

    /// What colours the letters: an `Rgb`, or any effect. White unless you
    /// say otherwise.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let red = TextArea::new(Font::X11_5X8).letters(Rgb::RED);
    /// let rainbow = TextArea::new(Font::X11_5X8).letters(Rainbow::new());
    /// ```
    pub fn letters(mut self, paint: impl Into<Paint>) -> Self {
        self.style.letters = paint.into();
        self
    }

    /// Draws a line one LED wide around the letters, in this colour or
    /// effect. Outlined letters usually look best with a gap of 2 between
    /// them ([`Font::spacing`]).
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::X11_5X8.spacing(2);
    /// let area = TextArea::new(font).letters(Rgb::WHITE).outline(Rgb::BLUE);
    /// ```
    pub fn outline(mut self, paint: impl Into<Paint>) -> Self {
        self.style.outline = Some(paint.into());
        self
    }

    /// Lights the rest of the box (behind the letters) with this colour or
    /// effect. Without it, the rest of the box is off (black).
    ///
    /// Behind gets the colour of each LED's place on the whole panel, so the
    /// same effect behind the text and on the rest of the panel joins up
    /// without a seam.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let area = TextArea::new(Font::X11_5X8).letters(Rgb::WHITE).behind(Plasma::new());
    /// ```
    pub fn behind(mut self, paint: impl Into<Paint>) -> Self {
        self.style.behind = Some(paint.into());
        self
    }

    /// True to swap round: the box is lit with the letter colour, and the
    /// letters themselves are dark (or show what's behind).
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Black letters on a green box.
    /// let area = TextArea::new(Font::X11_5X8).letters(Rgb::GREEN).reverse(true);
    /// ```
    pub const fn reverse(mut self, reverse: bool) -> Self {
        self.style.reverse = reverse;
        self
    }

    /// Changes the text. Call it as often as you like, for example once a
    /// second for a clock.
    ///
    /// Only the first `LEN` characters are kept (32 unless you chose
    /// otherwise). Letters other than plain ASCII (such as `é`) show as `?`.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let mut clock = TextArea::new(Font::seven_segment(8));
    /// clock.set_text("12:34");
    /// ```
    pub fn set_text(&mut self, text: &str) {
        self.len = 0;
        for (slot, character) in self.text.iter_mut().zip(text.chars()) {
            *slot = if character.is_ascii() {
                character as u8
            } else {
                b'?'
            };
            self.len += 1;
        }
    }

    /// How many LEDs wide the text is, in this area's font.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let mut area = TextArea::new(Font::X11_5X8);
    /// area.set_text("Hi");
    /// assert_eq!(area.text_width(), 10);
    /// ```
    pub fn text_width(&self) -> u16 {
        self.style.font.text_width(self.text())
    }

    /// Draws the text onto `surface` (a matrix `Canvas`, a strip, ...) at the
    /// time `now`. Only LEDs inside the box change.
    ///
    /// ```
    /// use ledkit_anim::Time;
    /// use ledkit_layout::{Canvas, MatrixLayout};
    /// use ledkit_text::prelude::*;
    ///
    /// let mut pixels = [Rgb::BLACK; 16 * 8];
    /// let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(16, 8)).unwrap();
    ///
    /// let mut area = TextArea::new(Font::X11_5X8).letters(Rainbow::new());
    /// area.set_text("Hi");
    /// area.render(&mut panel, Time::from_millis(250));
    /// ```
    pub fn render(&self, surface: &mut (impl Surface + ?Sized), now: impl Into<Time>) {
        let mut style = self.style;
        if !self.placed {
            style.left = 0;
            style.top = 0;
            style.width = surface.width();
            style.height = surface.height();
        }
        let text = self.text();
        let spare_width = i32::from(style.width) - i32::from(style.font.text_width(text));
        let spare_height = i32::from(style.height) - i32::from(style.font.height());
        // Rounded down, so text wider than the box still sits in the middle.
        let text_left = match self.across {
            Across::Left => 0,
            Across::Centre => spare_width.div_euclid(2),
            Across::Right => spare_width,
        };
        let text_top = match self.up_down {
            UpDown::Top => 0,
            UpDown::Middle => spare_height.div_euclid(2),
            UpDown::Bottom => spare_height,
        };
        style.draw(surface, now.into(), text, text_left, text_top);
    }

    /// The text as it is kept.
    fn text(&self) -> &[u8] {
        &self.text[..self.len]
    }
}

#[cfg(test)]
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::type_complexity
)]
mod tests {
    use super::*;
    use crate::Edges;
    use ledkit_anim::{Animation, Frame, Position, Rainbow};
    use ledkit_color::Rgb;
    use ledkit_layout::{Canvas, MatrixLayout};

    /// A colour no text area makes in these tests.
    const SENTINEL: Rgb = Rgb::new(1, 2, 3);

    /// Calls `check` with a `width` x `height` canvas filled with `SENTINEL`.
    fn panel<const N: usize>(
        width: u16,
        height: u16,
        check: impl FnOnce(&mut Canvas<'_, Rgb, MatrixLayout>),
    ) {
        let mut pixels = [SENTINEL; N];
        let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(width, height)).unwrap();
        check(&mut canvas);
    }

    /// The colour at (`x`, `y`).
    fn at(canvas: &Canvas<'_, Rgb, MatrixLayout>, x: i32, y: i32) -> Rgb {
        canvas.get(x as u16, y as u16).unwrap()
    }

    /// White where the font covers fully, black where it doesn't (1x bitmap
    /// faces are only ever fully on or off).
    fn expected(font: Font, text: &[u8], tx: i32, ty: i32) -> Rgb {
        match font.coverage(text, tx, ty) {
            255 => Rgb::WHITE,
            0 => Rgb::BLACK,
            other => panic!("partial coverage {other}"),
        }
    }

    #[test]
    fn exact_pixels() {
        // The BDF glyphs of '1' and '2' in X11 5x8.
        // The BDF rows are 00 20 60 20 20 20 70 00 and 00 60 90 10 60 80 F0 00.
        let one = [
            ".....", "..#..", ".##..", "..#..", "..#..", "..#..", ".###.", ".....",
        ];
        let two = [
            ".....", ".##..", "#..#.", "...#.", ".##..", "#....", "####.", ".....",
        ];
        panel::<96>(12, 8, |canvas| {
            let mut area = TextArea::new(Font::X11_5X8);
            area.set_text("12");
            area.render(canvas, Time::from_millis(0));
            for y in 0..8 {
                let mut row = [b'.'; 12];
                for (x, led) in row.iter_mut().enumerate() {
                    *led = match at(canvas, x as i32, y) {
                        Rgb::WHITE => b'#',
                        Rgb::BLACK => b'.',
                        other => panic!("unexpected {other:?}"),
                    };
                }
                let want = [".", one[y as usize], two[y as usize], "."].concat();
                assert_eq!(core::str::from_utf8(&row).unwrap(), want, "row {y}");
            }
        });
    }

    #[test]
    fn alignment() {
        let font = Font::X11_5X8;
        let cases = [
            (Across::Left, UpDown::Top, (0, 0)),
            (Across::Right, UpDown::Bottom, (5, 2)),
            (Across::Centre, UpDown::Middle, (2, 1)),
        ];
        for (across, up_down, (left, top)) in cases {
            panel::<100>(10, 10, |canvas| {
                let mut area = TextArea::new(font).at(0, 0, 10, 10).align(across, up_down);
                area.set_text("1");
                area.render(canvas, Time::from_millis(0));
                for y in 0..10 {
                    for x in 0..10 {
                        assert_eq!(
                            at(canvas, x, y),
                            expected(font, b"1", x - left, y - top),
                            "{across:?} {up_down:?} at ({x}, {y})"
                        );
                    }
                }
            });
        }
    }

    #[test]
    fn clips_to_box() {
        panel::<64>(8, 8, |canvas| {
            let mut area = TextArea::new(Font::X11_5X8)
                .at(2, 2, 4, 4)
                .behind(Rgb::BLUE)
                .outline(Rgb::RED);
            area.set_text("88");
            area.render(canvas, Time::from_millis(0));
            for y in 0..8 {
                for x in 0..8 {
                    let inside = (2..6).contains(&x) && (2..6).contains(&y);
                    assert_eq!(at(canvas, x, y) != SENTINEL, inside, "({x}, {y})");
                }
            }
        });
    }

    #[test]
    fn off_panel_and_empty() {
        // Each box, and which LEDs of an 8x8 panel it may touch.
        let boxes: [(i32, i32, u16, u16, fn(i32, i32) -> bool); 6] = [
            (-3, -3, 6, 6, |x, y| x < 3 && y < 3),
            (100, 100, 6, 6, |_, _| false),
            (2, 2, 0, 0, |_, _| false),
            (i32::MAX, i32::MAX, u16::MAX, u16::MAX, |_, _| false),
            (i32::MIN, i32::MIN, u16::MAX, u16::MAX, |_, _| false),
            (-70_000, 0, u16::MAX, 8, |x, _| x < 70_000 - 65_535),
        ];
        for (left, top, width, height, may_touch) in boxes {
            panel::<64>(8, 8, |canvas| {
                let mut area = TextArea::new(Font::X11_5X8)
                    .at(left, top, width, height)
                    .behind(Rgb::BLUE)
                    .outline(Rgb::RED);
                area.set_text("8");
                area.render(canvas, Time::from_millis(0));
                for y in 0..8 {
                    for x in 0..8 {
                        if !may_touch(x, y) {
                            assert_eq!(at(canvas, x, y), SENTINEL, "box at ({left}, {top})");
                        }
                    }
                }
            });
        }
        // A strip is a surface one LED tall; the text is clipped to it.
        let mut strip = [SENTINEL; 10];
        let mut area = TextArea::new(Font::X11_5X8).behind(Rgb::BLUE);
        area.set_text("8");
        area.render(&mut strip, Time::from_millis(0));
        assert!(strip.iter().all(|&led| led != SENTINEL));
        // No `at`: the whole surface, however small.
        let mut empty: [Rgb; 0] = [];
        area.render(&mut empty, Time::from_millis(0));
    }

    #[test]
    fn overflow_centres() {
        let font = Font::X11_5X8;
        assert_eq!(font.text_width(b"88888"), 25);
        panel::<64>(8, 8, |canvas| {
            let mut area = TextArea::new(font).at(0, 0, 8, 8);
            area.set_text("88888");
            area.render(canvas, Time::from_millis(0));
            // floor((8 - 25) / 2) = -9.
            for y in 0..8 {
                for x in 0..8 {
                    assert_eq!(at(canvas, x, y), expected(font, b"88888", x + 9, y));
                }
            }
        });
    }

    #[test]
    fn capacity_and_ascii() {
        let mut area = TextArea::<4>::with_capacity(Font::X11_5X8);
        area.set_text("héllo!");
        assert_eq!(area.text(), b"h?ll");
        area.set_text("ab");
        assert_eq!(area.text(), b"ab");
        assert_eq!(area.text_width(), 10);
        let mut none = TextArea::<0>::with_capacity(Font::X11_5X8);
        none.set_text("anything");
        assert_eq!(none.text(), b"");
    }

    #[test]
    fn reverse_and_outline() {
        let font = Font::X11_5X8;
        // Reversed: the glyph's lit LEDs are black, the rest of the box green.
        panel::<100>(10, 10, |canvas| {
            let mut area = TextArea::new(font).letters(Rgb::GREEN).reverse(true);
            area.set_text("1");
            area.render(canvas, Time::from_millis(0));
            for y in 0..10 {
                for x in 0..10 {
                    let want = match font.coverage(b"1", x - 2, y - 1) {
                        255 => Rgb::BLACK,
                        _ => Rgb::GREEN,
                    };
                    assert_eq!(at(canvas, x, y), want, "({x}, {y})");
                }
            }
        });
        // Outlined: letters white, LEDs touching a letter red, the rest blue.
        panel::<100>(10, 10, |canvas| {
            let mut area = TextArea::new(font).outline(Rgb::RED).behind(Rgb::BLUE);
            area.set_text("1");
            area.render(canvas, Time::from_millis(0));
            let mut outlined = 0;
            for y in 0..10 {
                for x in 0..10 {
                    let lit = |dx: i32, dy: i32| font.coverage(b"1", x - 2 + dx, y - 1 + dy) == 255;
                    let near = (-1..=1).any(|dy| (-1..=1).any(|dx| lit(dx, dy)));
                    let want = if lit(0, 0) {
                        Rgb::WHITE
                    } else if near {
                        outlined += 1;
                        Rgb::RED
                    } else {
                        Rgb::BLUE
                    };
                    assert_eq!(at(canvas, x, y), want, "({x}, {y})");
                }
            }
            assert!(outlined > 0);
        });
    }

    #[test]
    fn outline_matches_the_preview() {
        // Soft (partly covered) LEDs, reversed and not, against the preview's
        // way: nine lookups per LED.
        let font = Font::seven_segment(8).edges(Edges::Soft);
        for reverse in [false, true] {
            panel::<160>(20, 8, |canvas| {
                let mut area = TextArea::new(font)
                    .at(1, -1, 18, 9)
                    .outline(Rgb::RED)
                    .behind(Rgb::BLUE)
                    .reverse(reverse);
                area.set_text("12:4");
                area.render(canvas, Time::from_millis(0));
                let left = 1 + (18 - i32::from(font.text_width(b"12:4"))).div_euclid(2);
                let top = -1; // (9 - 8) / 2 rounds down to 0.
                let cover = |x: i32, y: i32| {
                    let a = font.coverage(b"12:4", x - left, y - top);
                    if reverse {
                        255 - a
                    } else {
                        a
                    }
                };
                for y in 0..8 {
                    for x in 0..20 {
                        if !(1..19).contains(&x) {
                            assert_eq!(at(canvas, x, y), SENTINEL);
                            continue;
                        }
                        let a = cover(x, y);
                        let most = (-1..=1)
                            .flat_map(|j| (-1..=1).map(move |i| (i, j)))
                            .map(|(i, j)| cover(x + i, y + j))
                            .max()
                            .unwrap();
                        let want = Rgb::BLUE.lerp(Rgb::RED, most - a).lerp(Rgb::WHITE, a);
                        assert_eq!(at(canvas, x, y), want, "({x}, {y}) reverse {reverse}");
                    }
                }
            });
        }
    }

    #[test]
    fn panel_coordinates() {
        panel::<96>(12, 8, |canvas| {
            let mut area = TextArea::new(Font::X11_5X8)
                .at(4, 0, 6, 8)
                .letters(Rainbow::new())
                .reverse(true);
            area.set_text(" ");
            area.render(canvas, Time::from_millis(700));
            // Reversed space: the whole box is letter.
            let frame = Frame::new(700, 12, 8);
            for y in 0..8u16 {
                for x in 4..10u16 {
                    let index = ledkit_anim::Surface::index(canvas, x, y).unwrap() as u16;
                    let want = Rainbow::new().color_at(Position { x, y, index }, &frame);
                    assert_eq!(canvas.get(x, y).unwrap(), want, "({x}, {y})");
                }
            }
        });
    }
}
