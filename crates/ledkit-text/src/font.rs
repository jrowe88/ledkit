//! [`Font`] and [`Size`]: which letters look like what, and how big.

use crate::edges::{sharpen, Edges};
use crate::faces::bitmap::{BakedSize, BitmapFace};
use crate::faces::data;
use crate::faces::fourteen::Fourteen;
use crate::faces::seven::Seven;

/// How big to draw a font. `X1` is the font's own size; the others are bigger
/// and are drawn fresh (not stretched), so letters stay as clean as the LEDs
/// allow.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// let small = Font::X11_5X8;
/// let big = Font::X11_5X8.size(Size::X2);
/// assert!(big.height() > small.height());
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Size {
    /// The font's own size (the default).
    #[default]
    X1,
    /// One and a quarter times as big.
    X1_25,
    /// One and a half times as big.
    X1_5,
    /// Twice as big.
    X2,
}

impl Size {
    /// Which of a face's four baked sizes this is.
    const fn index(self) -> usize {
        match self {
            Size::X1 => 0,
            Size::X1_25 => 1,
            Size::X1_5 => 2,
            Size::X2 => 3,
        }
    }
}

/// Which kind of letters: a baked bitmap font, or a segment display drawn at
/// any height.
#[derive(Copy, Clone)]
enum Face {
    Bitmap(&'static BitmapFace),
    Seven(Seven),
    Fourteen(Fourteen),
}

/// A font: the shape of the letters, plus a [`Size`], the sharpness of their
/// [`Edges`] and the gap between letters.
///
/// Pick a font, then adjust it. Every method returns a new `Font`, so you can
/// keep one in a `const`.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// const FONT: Font = Font::X11_5X8.size(Size::X1_25).edges(Edges::Crisp).spacing(2);
/// assert_eq!(FONT.height(), 10);
/// // Each letter takes its width (6) plus the gap (2).
/// assert_eq!(FONT.text_width("AB"), 16);
/// ```
#[derive(Copy, Clone)]
pub struct Font {
    face: Face,
    size: Size,
    edges: Edges,
    spacing: u8,
}

/// The first character the baked faces hold (space).
const FIRST_CHAR: u8 = 32;
/// The last character the baked faces hold (`~`).
const LAST_CHAR: u8 = 126;

macro_rules! bitmap_fonts {
    ($($(#[$doc:meta])* $name:ident => $data:ident,)*) => {
        impl Font {
            $(
                $(#[$doc])*
                pub const $name: Font = Font::bitmap(&data::$data);
            )*
        }
    };
}

bitmap_fonts! {
    /// X11 "4x6": the smallest readable font, 4 LEDs wide and 6 tall.
    X11_4X6 => X11_4X6,
    /// X11 "5x7": 5 wide, 7 tall.
    X11_5X7 => X11_5X7,
    /// X11 "5x8": 5 wide, 8 tall.
    X11_5X8 => X11_5X8,
    /// X11 "6x10": 6 wide, 10 tall.
    X11_6X10 => X11_6X10,
    /// X11 "6x13": 6 wide, 13 tall, with thin strokes.
    X11_6X13 => X11_6X13,
    /// X11 "6x13" bold: 6 wide, 13 tall.
    X11_6X13_BOLD => X11_6X13_BOLD,
    /// X11 "7x13" bold: 7 wide, 13 tall.
    X11_7X13_BOLD => X11_7X13_BOLD,
    /// X11 "7x14" bold: 7 wide, 14 tall.
    X11_7X14_BOLD => X11_7X14_BOLD,
    /// X11 "8x13" bold: 8 wide, 13 tall.
    X11_8X13_BOLD => X11_8X13_BOLD,
    /// X11 "9x15" bold: 9 wide, 15 tall.
    X11_9X15_BOLD => X11_9X15_BOLD,
    /// X11 "9x18" bold: 9 wide, 18 tall.
    X11_9X18_BOLD => X11_9X18_BOLD,
    /// X11 "10x20": 10 wide, 20 tall.
    X11_10X20 => X11_10X20,
    /// A tiny 3x5 font. It has capitals only: lower case shows as capitals.
    TINY_3X5 => TINY_3X5,
}

impl Font {
    /// A bitmap font at its own size, medium edges, no extra gap between
    /// letters (the glyphs already have a blank column built in).
    const fn bitmap(face: &'static BitmapFace) -> Font {
        Font {
            face: Face::Bitmap(face),
            size: Size::X1,
            edges: Edges::Medium,
            spacing: 0,
        }
    }

    /// A seven-segment display font, like a digital clock: `height` LEDs tall
    /// (at least 5; anything smaller is drawn 5 tall).
    ///
    /// It shows the digits, `-`, `_`, `=`, `:`, `.`, a space and the letters a
    /// seven-segment display can make. Some letters change shape with case
    /// (A/a, C/c, E/e, G/g, H/h, I/i, J/j, L/l, O/o, U/u); the rest have one
    /// form. `K`, `M`, `V`, `W`, `X` and anything else it can't draw show as
    /// a blank. [`TextArea`](crate::TextArea) and [`Ticker`](crate::Ticker)
    /// keep any character other than plain ASCII (such as `°`) as one `?`,
    /// which this font can't draw, so it shows as one blank.
    ///
    /// The letters are drawn finely and shrunk onto your LEDs, so their
    /// [`Edges`] work as for other fonts. For fully-on-or-off LEDs, use
    /// [`Font::hard_edges`]. [`Font::size`] has no effect: pick the height.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// const CLOCK: Font = Font::seven_segment(8);
    /// assert_eq!(CLOCK.height(), 8);
    /// assert_eq!(CLOCK.text_width("12:34"), 28);
    /// ```
    pub const fn seven_segment(height: u16) -> Font {
        Font::seven(Seven::new(height, false, false))
    }

    /// A seven-segment font like [`Font::seven_segment`], leaning to the right
    /// like many real displays. The lean makes the text a little wider.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::seven_segment_slanted(8);
    /// assert_eq!(font.height(), 8);
    /// assert_eq!(font.text_width("12:34"), 29);
    /// ```
    pub const fn seven_segment_slanted(height: u16) -> Font {
        Font::seven(Seven::new(height, true, false))
    }

    /// A fourteen-segment ("starburst") display font: `height` LEDs tall (at
    /// least 7; anything smaller is drawn 7 tall).
    ///
    /// Unlike [`Font::seven_segment`] it has every capital letter, as well as
    /// the digits and basic punctuation (`- + / * ' " _ = < > ? ! $ % : . ,`
    /// and a space). Lower case shows as capitals. Anything else shows as a
    /// blank. [`TextArea`](crate::TextArea) and [`Ticker`](crate::Ticker)
    /// keep any character other than plain ASCII (such as `°`) as one `?`,
    /// which this font draws as a `?`.
    ///
    /// The letters are drawn finely and shrunk onto your LEDs, so their
    /// [`Edges`] work as for other fonts. For fully-on-or-off LEDs, use
    /// [`Font::hard_edges`]. [`Font::size`] has no effect: pick the height.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// const SIGN: Font = Font::fourteen_segment(8);
    /// assert_eq!(SIGN.height(), 8);
    /// assert_eq!(SIGN.text_width("HELLO"), 34);
    /// assert_eq!(SIGN.text_width("12:34"), 32);
    /// ```
    pub const fn fourteen_segment(height: u16) -> Font {
        Font::segments(Face::Fourteen(Fourteen::new(height, false)))
    }

    /// A seven-segment font with medium edges and no extra gap between letters.
    const fn seven(face: Seven) -> Font {
        Font::segments(Face::Seven(face))
    }

    /// A segment font with medium edges and no extra gap between letters.
    const fn segments(face: Face) -> Font {
        Font {
            face,
            size: Size::X1,
            edges: Edges::Medium,
            spacing: 0,
        }
    }

    /// The same segment font with hard edges: each LED is either fully on or
    /// off, never in between, like a real segment display; [`Font::edges`]
    /// then has no effect. Seven-segment letters are drawn straight onto your
    /// LEDs instead of finely and shrunk, so they can come out a little
    /// narrower or wider. Fourteen-segment letters are drawn finely as usual,
    /// and an LED is on when at least half of it is covered.
    ///
    /// This has no effect on bitmap fonts (such as [`Font::X11_5X8`]): use
    /// [`Edges::VeryCrisp`] for those.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// const CLOCK: Font = Font::seven_segment(8).hard_edges();
    /// assert_eq!(CLOCK.height(), 8);
    /// assert_eq!(CLOCK.text_width("12:34"), 29);
    /// ```
    #[must_use]
    pub const fn hard_edges(mut self) -> Font {
        match self.face {
            Face::Seven(face) => self.face = Face::Seven(face.hardened()),
            Face::Fourteen(face) => self.face = Face::Fourteen(face.hardened()),
            Face::Bitmap(_) => {}
        }
        self
    }

    /// The same font at another [`Size`].
    ///
    /// Segment fonts ([`Font::seven_segment`], [`Font::fourteen_segment`]) are
    /// sized by their height instead, so this has no effect on them.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::X11_5X8.size(Size::X2);
    /// ```
    #[must_use]
    pub const fn size(mut self, size: Size) -> Font {
        self.size = size;
        self
    }

    /// The same font with sharper or softer [`Edges`].
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::X11_5X8.edges(Edges::Soft);
    /// ```
    #[must_use]
    pub const fn edges(mut self, edges: Edges) -> Font {
        self.edges = edges;
        self
    }

    /// The same font with `leds` extra empty LEDs between letters (0 to start
    /// with, since the letters already leave a little room; 0 to 3 is typical).
    /// Each letter gets the extra LEDs after it, for segment fonts too.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::X11_5X8.spacing(2);
    /// assert_eq!(font.text_width("AB"), 14);
    /// ```
    #[must_use]
    pub const fn spacing(mut self, leds: u8) -> Font {
        self.spacing = leds;
        self
    }

    /// The baked size of a bitmap face's letters.
    const fn baked(&self, face: &'static BitmapFace) -> &'static BakedSize {
        &face.sizes[self.size.index()]
    }

    /// How many LEDs tall the letters are.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// assert_eq!(Font::X11_5X8.height(), 8);
    /// assert_eq!(Font::seven_segment(12).height(), 12);
    /// ```
    pub const fn height(&self) -> u16 {
        match self.face {
            Face::Bitmap(face) => self.baked(face).height as u16,
            Face::Seven(face) => face.height(),
            Face::Fourteen(face) => face.height(),
        }
    }

    /// How many LEDs wide `text` is. Each letter takes its width plus the
    /// gap after it, so the last letter's gap is counted too.
    ///
    /// Characters are counted as [`TextArea::set_text`](crate::TextArea::set_text)
    /// keeps them: each one other than plain ASCII (such as `é`) counts as
    /// one `?`.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Two letters, each 5 wide.
    /// assert_eq!(Font::X11_5X8.text_width("AB"), 10);
    /// // With a 1 LED gap after each: 2 * (5 + 1).
    /// assert_eq!(Font::X11_5X8.spacing(1).text_width("AB"), 12);
    /// // Segment fonts: each extra LED of spacing adds one per letter.
    /// let clock = Font::seven_segment(8);
    /// assert_eq!(clock.spacing(1).text_width("12:34"), clock.text_width("12:34") + 5);
    /// ```
    pub fn text_width(&self, text: &str) -> u16 {
        let bytes = text.chars().map(|character| {
            if character.is_ascii() {
                character as u8
            } else {
                b'?'
            }
        });
        self.width_of(bytes)
    }

    /// [`Font::text_width`] of text already kept as bytes.
    pub(crate) fn bytes_width(&self, text: &[u8]) -> u16 {
        self.width_of(text.iter().copied())
    }

    /// How many LEDs wide the bytes `text` gives are.
    fn width_of(&self, text: impl Iterator<Item = u8>) -> u16 {
        match self.face {
            Face::Bitmap(face) => {
                let advance = u64::from(self.baked(face).width) + u64::from(self.spacing);
                let width = text.count() as u64 * advance;
                u16::try_from(width).unwrap_or(u16::MAX)
            }
            Face::Seven(face) => face.width_of(text, self.spacing),
            Face::Fourteen(face) => face.width_of(text, self.spacing),
        }
    }

    /// The part of `text` that can light LED columns `first..=last` (in the
    /// text's own columns, where 0 is its left edge), so that drawing only
    /// that part looks exactly like drawing the whole text there.
    ///
    /// Bitmap letters are all the same width, so the part is worked out
    /// straight away. Segment letters differ in width and are laid out in
    /// drawing pixels (8 per LED, or 1 for hard-edged seven-segment), so the
    /// part's first letter may start part of the way into an LED: that is
    /// [`Visible::origin`], which goes to [`Font::coverage_from`].
    pub(crate) fn visible(&self, text: &[u8], first: i64, last: i64) -> Visible {
        let scale = match self.face {
            Face::Bitmap(face) => {
                let advance = i64::from(self.baked(face).width) + i64::from(self.spacing);
                let advance = advance.max(1);
                let start = usize::try_from(first.max(0) / advance)
                    .unwrap_or(usize::MAX)
                    .min(text.len());
                let end = usize::try_from(last.div_euclid(advance) + 1)
                    .unwrap_or(0)
                    .clamp(start, text.len());
                // `start` letters are at most a u16 of LEDs wide, so this fits.
                let skipped = i64::try_from(start).unwrap_or(0) * advance;
                return Visible {
                    start,
                    end,
                    shift: skipped,
                    origin: 0,
                };
            }
            Face::Seven(face) => face.scale(),
            Face::Fourteen(_) => 8,
        };
        let scale = i64::from(scale);
        // The drawing pixel columns the LEDs cover.
        let (low, high) = (
            first.saturating_mul(scale),
            (last + 1).saturating_mul(scale),
        );
        let mut start = None;
        let mut end = text.len();
        let mut at = 0i64;
        for (index, &byte) in text.iter().enumerate() {
            // Letters only reach right of where they start.
            if at >= high {
                end = index;
                break;
            }
            let (advance, reach) = self.segment_steps(byte);
            if start.is_none() && at + i64::from(reach) > low {
                start = Some((index, at));
            }
            at += i64::from(advance);
        }
        let (start, at) = start.unwrap_or((end, 0));
        Visible {
            start,
            end,
            // `at` is not negative.
            shift: at / scale,
            origin: i32::try_from(at % scale).unwrap_or(0),
        }
    }

    /// A segment letter's advance (where the next letter starts) and reach
    /// (how far right its lit pixels can go), in drawing pixels from where it
    /// starts, as the face's coverage lays letters out.
    fn segment_steps(&self, byte: u8) -> (i32, i32) {
        match self.face {
            Face::Seven(face) => (face.advance(byte, self.spacing), face.reach(byte)),
            Face::Fourteen(face) => (face.advance(byte, self.spacing), face.reach(byte)),
            Face::Bitmap(_) => (1, 1),
        }
    }

    /// How much of the LED at (`x`, `y`) the letters cover, 0 to 255, where
    /// (0, 0) is the top left of `text`. 0 outside the text.
    #[cfg(test)]
    pub(crate) fn coverage(&self, text: &[u8], x: i32, y: i32) -> u8 {
        self.coverage_from(text, 0, x, y)
    }

    /// [`Font::coverage`] for part of a text, whose first letter starts
    /// `origin` drawing pixels into LED column 0 (see [`Font::visible`]; 0
    /// for a whole text).
    pub(crate) fn coverage_from(&self, text: &[u8], origin: i32, x: i32, y: i32) -> u8 {
        match self.face {
            Face::Bitmap(face) => self.bitmap_coverage(face, text, x, y),
            Face::Seven(face) => {
                let covered = face.coverage_from(text, origin, self.spacing, x, y);
                if face.hard() {
                    if covered > 0 {
                        255
                    } else {
                        0
                    }
                } else {
                    sharpen(covered, 64, self.edges)
                }
            }
            Face::Fourteen(face) => {
                let covered = face.coverage_from(text, origin, self.spacing, x, y);
                if face.hard() {
                    // As the preview: on when at least half covered.
                    if covered >= 32 {
                        255
                    } else {
                        0
                    }
                } else {
                    sharpen(covered, 64, self.edges)
                }
            }
        }
    }

    /// [`Font::coverage`] for a bitmap face.
    fn bitmap_coverage(&self, face: &'static BitmapFace, text: &[u8], x: i32, y: i32) -> u8 {
        let baked = self.baked(face);
        let width = usize::from(baked.width);
        let height = usize::from(baked.height);
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return 0;
        };
        let advance = width + usize::from(self.spacing);
        let (letter, column) = (x / advance, x % advance);
        if column >= width || y >= height {
            return 0;
        }
        let Some(&code) = text.get(letter) else {
            return 0;
        };
        let code = if (FIRST_CHAR..=LAST_CHAR).contains(&code) {
            code
        } else {
            b'?'
        };
        let glyph = usize::from(code - FIRST_CHAR);
        let covered = baked.data[glyph * width * height + y * width + column];
        sharpen(u16::from(covered), 16, self.edges)
    }

    /// The name of the font's face, for example `"5x8"`.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// assert_eq!(Font::X11_5X8.name(), "5x8");
    /// assert_eq!(Font::seven_segment(8).name(), "7-segment");
    /// assert_eq!(Font::fourteen_segment(8).name(), "14-segment");
    /// ```
    pub const fn name(&self) -> &'static str {
        match self.face {
            Face::Bitmap(face) => face.name,
            Face::Seven(face) => {
                if face.slanted() {
                    "7-segment slanted"
                } else {
                    "7-segment"
                }
            }
            Face::Fourteen(_) => "14-segment",
        }
    }
}

/// Which letters of a text to draw, from [`Font::visible`]: `text[start..end]`,
/// drawn `shift` LEDs right of where the whole text starts, with its first
/// letter `origin` drawing pixels into that LED.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct Visible {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) shift: i64,
    pub(crate) origin: i32,
}

#[cfg(test)]
#[allow(clippy::cast_possible_wrap)]
mod tests {
    use super::*;

    #[test]
    fn one_x_is_exact() {
        // Rows 1, 2 and 4 of 'A' in X11 5x8.
        let rows = [(1, "01100"), (2, "10010"), (4, "11110")];
        for (y, row) in rows {
            for (x, bit) in row.bytes().enumerate() {
                let expected = if bit == b'1' { 255 } else { 0 };
                assert_eq!(Font::X11_5X8.coverage(b"A", x as i32, y), expected);
            }
        }
    }

    #[test]
    fn sizes() {
        let font = Font::X11_5X8.size(Size::X1_25);
        assert_eq!(font.height(), 10);
        assert_eq!(font.text_width("AB"), 12);
        assert_eq!(font.spacing(2).text_width("AB"), 16);
    }

    #[test]
    fn missing_and_case() {
        let font = Font::TINY_3X5;
        for (x, y) in [(0, 0), (1, 0), (1, 1), (0, 2), (2, 3), (1, 4)] {
            assert_eq!(font.coverage(b"a", x, y), font.coverage(b"A", x, y));
            assert_eq!(font.coverage(b"@", x, y), font.coverage(b"?", x, y));
            assert_eq!(font.coverage(&[200], x, y), font.coverage(b"?", x, y));
        }
    }

    #[test]
    fn zero_outside_the_text() {
        let font = Font::X11_5X8;
        // Negative, past the end, below the letters.
        assert_eq!(font.coverage(b"A", -1, 4), 0);
        assert_eq!(font.coverage(b"A", 1, -1), 0);
        assert_eq!(font.coverage(b"A", 5, 4), 0);
        assert_eq!(font.coverage(b"A", 1, 8), 0);
        // The second letter starts at x = 5 (no gap) or x = 6 (a gap of 1).
        assert_eq!(font.coverage(b"AA", 5, 4), 255);
        assert_eq!(font.spacing(1).coverage(b"AA", 6, 4), 255);
        assert_eq!(font.spacing(1).coverage(b"AA", 5, 4), 0);
    }

    #[test]
    fn seven_segment_through_the_font() {
        let soft = Font::seven_segment(10);
        let face = Seven::new(10, false, false);
        let text = b"12:34";
        for edges in [Edges::Soft, Edges::Medium, Edges::Crisp, Edges::VeryCrisp] {
            let font = soft.edges(edges);
            for y in -1..11 {
                for x in -1..=i32::from(font.bytes_width(text)) {
                    let expected = sharpen(face.coverage(text, 0, x, y), 64, edges);
                    assert_eq!(font.coverage(text, x, y), expected);
                }
            }
        }
        // Hard edges: fully on or off, and the same LEDs as the plain drawing.
        let hard = Font::seven_segment_slanted(10).hard_edges();
        let face = Seven::new(10, true, true);
        assert_eq!(hard.bytes_width(text), face.width(text, 0));
        for y in 0..10 {
            for x in 0..i32::from(hard.bytes_width(text)) {
                let expected = if face.coverage(text, 0, x, y) == 64 {
                    255
                } else {
                    0
                };
                assert_eq!(hard.edges(Edges::Soft).coverage(text, x, y), expected);
            }
        }
    }

    #[test]
    fn segment_options() {
        // Too short is raised to 5.
        assert_eq!(Font::seven_segment(3).height(), 5);
        assert_eq!(Font::seven_segment_slanted(0).height(), 5);
        // Size has no effect on segment fonts.
        let font = Font::seven_segment(8);
        assert_eq!(font.size(Size::X2).height(), 8);
        assert_eq!(font.size(Size::X2).text_width("88"), font.text_width("88"));
        // Spacing adds blank LEDs after each letter.
        assert_eq!(font.spacing(2).text_width("88"), font.text_width("88") + 4);
        assert_eq!(
            font.hard_edges().spacing(1).text_width("88"),
            font.hard_edges().text_width("88") + 2
        );
        // Hard edges leave bitmap fonts alone.
        let bitmap = Font::X11_5X8.edges(Edges::Soft);
        for (x, y) in [(0, 3), (1, 2), (2, 5)] {
            assert_eq!(
                bitmap.hard_edges().coverage(b"A", x, y),
                bitmap.coverage(b"A", x, y)
            );
        }
        assert_eq!(Font::seven_segment_slanted(8).name(), "7-segment slanted");
    }

    #[test]
    fn fourteen_segment_through_the_font() {
        let soft = Font::fourteen_segment(10);
        let face = Fourteen::new(10, false);
        let text = b"Hi 12:34";
        for edges in [Edges::Soft, Edges::Medium, Edges::Crisp, Edges::VeryCrisp] {
            let font = soft.edges(edges);
            for y in -1..11 {
                for x in -1..=i32::from(font.bytes_width(text)) {
                    let expected = sharpen(face.coverage(text, 0, x, y), 64, edges);
                    assert_eq!(font.coverage(text, x, y), expected);
                }
            }
        }
        // Hard edges: the same drawing, on where at least half covered.
        let hard = soft.hard_edges();
        assert_eq!(hard.bytes_width(text), soft.bytes_width(text));
        let mut on = 0;
        for y in 0..10 {
            for x in 0..i32::from(hard.bytes_width(text)) {
                let expected = if face.coverage(text, 0, x, y) >= 32 {
                    on += 1;
                    255
                } else {
                    0
                };
                assert_eq!(hard.edges(Edges::Soft).coverage(text, x, y), expected);
            }
        }
        assert!(on > 0);
        // Too short is raised to 7; size has no effect; spacing adds blank
        // LEDs after each letter.
        assert_eq!(Font::fourteen_segment(3).height(), 7);
        assert_eq!(soft.size(Size::X2).text_width("88"), soft.text_width("88"));
        assert_eq!(soft.spacing(2).text_width("88"), soft.text_width("88") + 4);
        assert_eq!(soft.name(), "14-segment");
    }

    #[test]
    fn question_mark_has_ink() {
        let any = (0..5).any(|x| (0..8).any(|y| Font::X11_5X8.coverage(b"?", x, y) > 0));
        assert!(any);
    }
}
