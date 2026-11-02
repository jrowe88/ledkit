//! [`Style`]: how text is drawn in a box (the font, the colours and the box
//! itself). Shared by [`TextArea`](crate::TextArea) and the ticker.

use ledkit_anim::{Frame, Position, Surface, Time};
use ledkit_color::Rgb;

use crate::{Font, Paint};

/// The font, the three colour sources (letters, outline, behind), reverse,
/// and the box on the panel that drawing is kept inside.
#[derive(Copy, Clone)]
pub(crate) struct Style {
    pub(crate) font: Font,
    pub(crate) letters: Paint,
    pub(crate) outline: Option<Paint>,
    pub(crate) behind: Option<Paint>,
    pub(crate) reverse: bool,
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl Style {
    /// White letters, no outline, nothing behind, in an empty box at (0, 0).
    pub(crate) const fn new(font: Font) -> Self {
        Self {
            font,
            letters: Paint::Color(Rgb::WHITE),
            outline: None,
            behind: None,
            reverse: false,
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    }

    /// Draws `text` with its top left at (`text_left`, `text_top`) inside the
    /// box. Only LEDs inside both the box and the surface are touched.
    ///
    /// Per LED, as the preview's `draw`: start from behind (black if none),
    /// mix toward the outline by the outline amount, then toward the letters
    /// by the letter coverage.
    pub(crate) fn draw(
        &self,
        surface: &mut (impl Surface + ?Sized),
        now: Time,
        text: &[u8],
        text_left: i32,
        text_top: i32,
    ) {
        let frame = Frame::new(now.as_millis(), surface.width(), surface.height());
        // i64 so that a box near i32::MAX can't overflow.
        let left = i64::from(self.left);
        let top = i64::from(self.top);
        let x_start = left.max(0);
        let x_end = (left + i64::from(self.width)).min(i64::from(frame.width));
        let y_start = top.max(0);
        let y_end = (top + i64::from(self.height)).min(i64::from(frame.height));
        // Now all four are between 0 and the surface size, or the box is empty.
        let (Ok(x_start), Ok(x_end), Ok(y_start), Ok(y_end)) = (
            u16::try_from(x_start),
            u16::try_from(x_end),
            u16::try_from(y_start),
            u16::try_from(y_end),
        ) else {
            return;
        };
        // Where the first LED of each row is, in the text's own coordinates.
        let first_x = clamp(i64::from(x_start) - left - i64::from(text_left));
        for y in y_start..y_end {
            let ty = clamp(i64::from(y) - top - i64::from(text_top));
            if self.outline.is_some() {
                self.draw_outlined_row(surface, frame, text, y, x_start..x_end, first_x, ty);
            } else {
                let mut tx = first_x;
                for x in x_start..x_end {
                    let a = self.cover(text, tx, ty);
                    self.set(surface, frame, x, y, a, 0);
                    tx = tx.saturating_add(1);
                }
            }
        }
    }

    /// One row with an outline. The outline needs the most-lit of the 3x3
    /// LEDs around each LED; walking along the row, each column of three is
    /// worked out once and kept for the next two LEDs, so it costs three
    /// coverage lookups per LED rather than nine.
    #[allow(clippy::too_many_arguments)]
    fn draw_outlined_row(
        &self,
        surface: &mut (impl Surface + ?Sized),
        frame: Frame,
        text: &[u8],
        y: u16,
        columns: core::ops::Range<u16>,
        first_x: i32,
        ty: i32,
    ) {
        let mut tx = first_x;
        let (_, mut before) = self.column(text, tx.saturating_sub(1), ty);
        let (mut a, mut here) = self.column(text, tx, ty);
        for x in columns {
            let (next_a, after) = self.column(text, tx.saturating_add(1), ty);
            let outline = before.max(here).max(after) - a;
            self.set(surface, frame, x, y, a, outline);
            (before, here, a) = (here, after, next_a);
            tx = tx.saturating_add(1);
        }
    }

    /// The coverage at (`tx`, `ty`), and the most of the three LEDs from
    /// `ty - 1` to `ty + 1` in that column.
    fn column(&self, text: &[u8], tx: i32, ty: i32) -> (u8, u8) {
        let middle = self.cover(text, tx, ty);
        let above = self.cover(text, tx, ty.saturating_sub(1));
        let below = self.cover(text, tx, ty.saturating_add(1));
        (middle, middle.max(above).max(below))
    }

    /// How much the LED at (`tx`, `ty`) of the text is "letter", 0 to 255,
    /// swapped round when reversed (so outside the text is fully letter).
    fn cover(&self, text: &[u8], tx: i32, ty: i32) -> u8 {
        let a = self.font.coverage(text, tx, ty);
        if self.reverse {
            255 - a
        } else {
            a
        }
    }

    /// Mixes and sets the panel LED at (`x`, `y`), if there is one.
    fn set(
        &self,
        surface: &mut (impl Surface + ?Sized),
        frame: Frame,
        x: u16,
        y: u16,
        letter: u8,
        outline: u8,
    ) {
        let Some(index) = surface.index(x, y) else {
            return;
        };
        let at = Position {
            x,
            y,
            index: u16::try_from(index).unwrap_or(u16::MAX),
        };
        // Fully letter: the letter paint alone, so skip asking the others.
        let color = if letter == 255 {
            self.letters.color_at(at, &frame)
        } else {
            let mut color = match self.behind {
                Some(behind) => behind.color_at(at, &frame),
                None => Rgb::BLACK,
            };
            if let (Some(paint), 1..) = (self.outline, outline) {
                color = color.lerp(paint.color_at(at, &frame), outline);
            }
            if letter > 0 {
                color = color.lerp(self.letters.color_at(at, &frame), letter);
            }
            color
        };
        surface.set(x, y, color);
    }
}

/// `value`, held inside the range of an `i32`.
fn clamp(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(if value < 0 { i32::MIN } else { i32::MAX })
}
