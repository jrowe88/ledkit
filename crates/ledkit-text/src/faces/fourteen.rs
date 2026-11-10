//! The fourteen-segment ("starburst") face: every capital letter has a shape.
//!
//! An integer port of the preview page's `renderSeg14`
//! (`site/text-preview/glyphs.js`). Each segment is a capsule: every pixel
//! within half a stroke of a line segment. The preview draws the capsules at
//! 8 pixels per LED and each LED shows how many of its 8 x 8 pixels are lit.
//! Nothing is drawn into a buffer here: each LED finds the letter it is over,
//! skips the capsules whose box misses it, and lights its pixels row by row.
//!
//! Like the preview, a pixel `(x, y)` (its integer corner, not its centre) is
//! lit when its distance to a capsule's line segment is at most half a
//! stroke. The straight segments and dots are exact in integers. The
//! diagonals' ends are pulled back by an irrational distance; their sides are
//! tested exactly and their round ends in 1/256 of a pixel.

// A glyph's segments are bits 0 to 13 of a mask, for a (top), b and c (right),
// d (bottom), e and f (left), g1 and g2 (middle halves), h and j (upper
// diagonals), i and l (centre verticals), k and m (lower diagonals).
// `Fourteen::capsule` gives each one's line.

/// The preview's `SEG14`: which segments each character lights.
const FORMS: [(u8, &str); 54] = [
    (b'A', "a b c e f g1 g2"),
    (b'B', "a b c d i l g2"),
    (b'C', "a d e f"),
    (b'D', "a b c d i l"),
    (b'E', "a d e f g1"),
    (b'F', "a e f g1"),
    (b'G', "a c d e f g2"),
    (b'H', "b c e f g1 g2"),
    (b'I', "a d i l"),
    (b'J', "b c d e"),
    (b'K', "e f g1 j m"),
    (b'L', "d e f"),
    (b'M', "b c e f h j"),
    (b'N', "b c e f h m"),
    (b'O', "a b c d e f"),
    (b'P', "a b e f g1 g2"),
    (b'Q', "a b c d e f m"),
    (b'R', "a b e f g1 g2 m"),
    (b'S', "a c d f g1 g2"),
    (b'T', "a i l"),
    (b'U', "b c d e f"),
    (b'V', "e f k j"),
    (b'W', "b c e f k m"),
    (b'X', "h j k m"),
    (b'Y', "h j l"),
    (b'Z', "a d j k"),
    (b'0', "a b c d e f j k"),
    (b'1', "b c j"),
    (b'2', "a b d e g1 g2"),
    (b'3', "a b c d g2"),
    (b'4', "b c f g1 g2"),
    (b'5', "a c d f g1 g2"),
    (b'6', "a c d e f g1 g2"),
    (b'7', "a b c"),
    (b'8', "a b c d e f g1 g2"),
    (b'9', "a b c d f g1 g2"),
    (b'-', "g1 g2"),
    (b'+', "g1 g2 i l"),
    (b'/', "j k"),
    (b'*', "g1 g2 h i j k l m"),
    (b'\'', "i"),
    (b'"', "f i"),
    (b'_', "d"),
    (b'=', "d g1 g2"),
    (b'<', "j m"),
    (b'>', "h k"),
    (b'?', "a b g2 l"),
    (b'!', "b c"),
    (b'$', "a c d f g1 g2 i l"),
    (b'%', "c f j k g1 g2"),
    (b' ', ""),
    // Not in the preview's table, but blank there too (`?? ''`).
    (b'#', ""),
    (b'@', ""),
    (b'&', ""),
];

/// Segment numbers by name.
const A: u32 = 0;
const B: u32 = 1;
const C: u32 = 2;
const D: u32 = 3;
const E: u32 = 4;
const F: u32 = 5;
const G1: u32 = 6;
const G2: u32 = 7;
const H: u32 = 8;
const I: u32 = 9;
const J: u32 = 10;
const K: u32 = 11;
const L: u32 = 12;
const M: u32 = 13;

/// Builds a segment mask from the preview's names, such as `"a b g1"`.
/// An unknown name is a build error (the table is built at compile time),
/// so a typo in [`FORMS`] can't silently draw the wrong glyph.
const fn mask(names: &str) -> u16 {
    let names = names.as_bytes();
    let mut mask = 0;
    let mut i = 0;
    while i < names.len() {
        let segment = match names[i] {
            b' ' => {
                i += 1;
                continue;
            }
            b'a' => A,
            b'b' => B,
            b'c' => C,
            b'd' => D,
            b'e' => E,
            b'f' => F,
            b'g' => {
                // g1 or g2: the digit is part of the name.
                i += 1;
                assert!(i < names.len(), "segment g needs a 1 or 2");
                match names[i] {
                    b'1' => G1,
                    b'2' => G2,
                    _ => panic!("segment g needs a 1 or 2"),
                }
            }
            b'h' => H,
            b'i' => I,
            b'j' => J,
            b'k' => K,
            b'l' => L,
            b'm' => M,
            _ => panic!("unknown segment name"),
        };
        i += 1;
        // A name ends at a space or the end.
        assert!(i == names.len() || names[i] == b' ', "unknown segment name");
        mask |= 1 << segment;
    }
    mask
}

/// [`FORMS`] as a table by ASCII code: 0 (blank) where a character has none.
const TABLE: [u16; 128] = {
    let mut table = [0; 128];
    let mut i = 0;
    while i < FORMS.len() {
        table[FORMS[i].0 as usize] = mask(FORMS[i].1);
        i += 1;
    }
    table
};

/// What one character draws.
#[derive(Copy, Clone)]
enum Glyph {
    /// A letter-sized cell lighting these segments (none: blank).
    Segments(u16),
    /// A narrow cell with two dots.
    Colon,
    /// A narrow cell with one dot at the bottom (`.` and `,`).
    Dot,
}

impl Glyph {
    /// As the preview: upper case first; characters with no form (and
    /// non-ASCII bytes) are blank.
    const fn of(byte: u8) -> Glyph {
        match byte {
            b':' => Glyph::Colon,
            b'.' | b',' => Glyph::Dot,
            128.. => Glyph::Segments(0),
            _ => Glyph::Segments(TABLE[byte.to_ascii_uppercase() as usize]),
        }
    }
}

/// One capsule, in drawing pixels from the top left of its glyph's cell.
#[derive(Copy, Clone)]
enum Capsule {
    /// From (`from`, `y`) to (`to`, `y`), `from <= to`.
    Across { y: i32, from: i32, to: i32 },
    /// From (`x`, `from`) to (`x`, `to`), `from <= to`.
    Down { x: i32, from: i32, to: i32 },
    /// A diagonal: segment h, flipped left to right (j), top to bottom (k)
    /// or both (m).
    Slope { flip_x: bool, flip_y: bool },
}

// The preview pulls back both ends of a segment; on short segments (g1 and
// g2 at small heights) the ends cross, which draws the same capsule as the
// other way round. `across` and `down` put the ends in order.

/// A horizontal capsule at row `y` between columns `a` and `b`.
const fn across(y: i32, a: i32, b: i32) -> Capsule {
    if a <= b {
        Capsule::Across { y, from: a, to: b }
    } else {
        Capsule::Across { y, from: b, to: a }
    }
}

/// A vertical capsule at column `x` between rows `a` and `b`.
const fn down(x: i32, a: i32, b: i32) -> Capsule {
    if a <= b {
        Capsule::Down { x, from: a, to: b }
    } else {
        Capsule::Down { x, from: b, to: a }
    }
}

/// `round(numerator / denominator)` with halves rounded up, like JavaScript's
/// `Math.round`, for a non-negative numerator.
const fn round_div(numerator: i32, denominator: i32) -> i32 {
    (2 * numerator + denominator) / (2 * denominator)
}

/// The integer square root (rounded down) of a `u128`, for the measurements
/// worked out once per face.
const fn isqrt_wide(n: u128) -> u128 {
    let mut root = 0;
    let mut rest = n;
    let mut bit = 1u128 << 126;
    while bit > n {
        bit >>= 2;
    }
    while bit != 0 {
        if rest >= root + bit {
            rest -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// The integer square root (rounded down) of a `u32`, per pixel row.
fn isqrt(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut root = 0;
    let mut rest = n;
    // The highest power of four not above `n`.
    let mut bit = 1u32 << (n.ilog2() & !1);
    while bit != 0 {
        if rest >= root + bit {
            rest -= root + bit;
            root = (root >> 1) + bit;
        } else {
            root >>= 1;
        }
        bit >>= 2;
    }
    root
}

/// One pixel row of an LED: the bits of columns `low..=high` (from the LED's
/// left column `left`), as the low 8 bits.
fn span(low: i32, high: i32, left: i32) -> u64 {
    let from = if low > left { low - left } else { 0 };
    let to = if high < left + 7 { high - left } else { 7 };
    if from > to {
        return 0;
    }
    u64::from((0xFFu8 >> (7 - to)) & (0xFFu8 << from))
}

/// Every pixel of an LED lit.
const FULL: u64 = u64::MAX;

/// A fourteen-segment face at one height, with its drawing measurements
/// worked out once.
///
/// All measurements are in drawing pixels, 8 per LED: the preview's
/// `renderSeg14` draws at 8 times the height and shrinks.
#[derive(Copy, Clone)]
pub(crate) struct Fourteen {
    /// LEDs tall, at least 7.
    height: u16,
    hard: bool,
    /// The drawing's height in pixels.
    drawn: i32,
    /// Half a stroke: a capsule's radius.
    half: i32,
    /// A letter cell's width.
    wide: i32,
    /// The gap on each side of a cell, and how far segments are pulled back
    /// from their ends.
    gap: i32,
    /// Segment h, from (half, half) to (wide / 2, drawn / 2) before it is
    /// pulled back: across and down.
    run: i64,
    rise: i64,
    /// `isqrt(run^2 + rise^2)`: its length, rounded down.
    length: i64,
    /// `floor(half * length)`: a pixel is within `half` of the diagonal's
    /// line when `|cross| <= band`, where `cross` is its cross product with
    /// (run, rise) from the corner.
    band: i64,
    /// Where the pulled-back segment starts and ends along the diagonal, as
    /// `5 * dot`, where `dot` is a pixel's dot product with (run, rise): a
    /// pixel with `5 * dot` below `low5` is past one end, above `high5` past
    /// the other. The diagonals are pulled back by `pull = 1.6 * gap` at both
    /// ends: the two are `5 * pull * length` (rounded up) and
    /// `5 * (run^2 + rise^2 - pull * length)`.
    low5: i64,
    high5: i64,
    /// Whether the pulled-back ends crossed over (at some small heights,
    /// such as 12, `2 * pull` is more than the length). The preview then
    /// draws the capsule between the crossed ends; `low5` is the far end's.
    crossed: bool,
    /// The pulled-back near end, from the corner, in 1/256 of a pixel. The
    /// far end is the same distance back from (run, rise).
    end_x: i64,
    end_y: i64,
    /// The pulled-back segment's box grown by half a stroke, in pixels from
    /// the corner: columns `box_left..=box_right`, rows `box_top..=box_bottom`.
    box_left: i64,
    box_right: i64,
    box_top: i64,
    box_bottom: i64,
}

impl Fourteen {
    /// A face `height` LEDs tall (below 7 counts as 7).
    // The measurements are positive, and their square roots well below
    // `i64::MAX`.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    pub(crate) const fn new(height: u16, hard: bool) -> Fourteen {
        let height = if height < 7 { 7 } else { height };
        let rows = height as i32;
        let drawn = 8 * rows;
        // The preview's measurements: t = round(H / 8) * 8,
        // W = round(0.62 H) * 8, gap = round(0.9 t). H >= 7 keeps t >= 8.
        let thick = round_div(rows, 8) * 8;
        let half = thick / 2;
        let wide = round_div(62 * rows, 100) * 8;
        let gap = round_div(9 * thick, 10);

        let run = (wide / 2 - half) as i64;
        let rise = (drawn / 2 - half) as i64;
        let length2 = run * run + rise * rise;
        let wide2 = length2 as u128;
        let length = isqrt_wide(wide2) as i64;
        let band = isqrt_wide((half as u128) * (half as u128) * wide2) as i64;
        // 5 * pull * length = 8 * gap * sqrt(length2), rounded up.
        let squared = 64 * (gap as u128) * (gap as u128) * wide2;
        let root = isqrt_wide(squared);
        let near5 = (root + (root * root != squared) as u128) as i64;
        let far5 = 5 * length2 - near5;
        let crossed = near5 > far5;
        // The near end moves (run, rise) * pull / length; in 1/256 of a pixel
        // that is sqrt((2048 * gap * run)^2 / (25 * length2)), rounded.
        let end_x = Fourteen::pulled(gap, run, wide2);
        let end_y = Fourteen::pulled(gap, rise, wide2);
        // The far end, in 1/256 of a pixel.
        let (far_x, far_y) = (256 * run - end_x, 256 * rise - end_y);
        let h = half as i64;
        Fourteen {
            height,
            hard,
            drawn,
            half,
            wide,
            gap,
            run,
            rise,
            length,
            band,
            low5: if crossed { far5 } else { near5 },
            high5: if crossed { near5 } else { far5 },
            crossed,
            end_x,
            end_y,
            box_left: Fourteen::min(end_x, far_x) / 256 - h,
            box_right: (Fourteen::max(end_x, far_x) + 255) / 256 + h,
            box_top: Fourteen::min(end_y, far_y) / 256 - h,
            box_bottom: (Fourteen::max(end_y, far_y) + 255) / 256 + h,
        }
    }

    /// The smaller of `a` and `b` (`Ord::min` isn't `const`).
    const fn min(a: i64, b: i64) -> i64 {
        if a < b {
            a
        } else {
            b
        }
    }

    /// The larger of `a` and `b`.
    const fn max(a: i64, b: i64) -> i64 {
        if a > b {
            a
        } else {
            b
        }
    }

    /// `round(256 * 1.6 * gap * along / sqrt(length2))`.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    const fn pulled(gap: i32, along: i64, length2: u128) -> i64 {
        let scaled = 2048 * gap as u128 * along as u128;
        let twice = isqrt_wide(4 * scaled * scaled / (25 * length2));
        twice.div_ceil(2) as i64
    }

    /// LEDs tall.
    pub(crate) const fn height(&self) -> u16 {
        self.height
    }

    /// The same face, hard-edged.
    pub(crate) const fn hardened(&self) -> Fourteen {
        let mut face = *self;
        face.hard = true;
        face
    }

    /// Whether each LED is fully on or off (on when at least half covered).
    pub(crate) const fn hard(&self) -> bool {
        self.hard
    }

    /// How wide a glyph's cell is, in drawing pixels.
    const fn cell(&self, glyph: Glyph) -> i32 {
        match glyph {
            Glyph::Segments(_) => self.wide,
            Glyph::Colon | Glyph::Dot => 2 * self.half + 2 * self.gap,
        }
    }

    /// How far past its cell the next glyph starts, in drawing pixels.
    const fn padding(&self, spacing: u8) -> i32 {
        2 * self.gap + 8 * spacing as i32
    }

    /// How many LEDs wide `text` is: each glyph's cell, its two gaps and
    /// `spacing` blank LEDs. 0 for no text.
    #[cfg(test)]
    pub(crate) fn width(&self, text: &[u8], spacing: u8) -> u16 {
        self.width_of(text.iter().copied(), spacing)
    }

    /// [`Fourteen::width`] of the bytes `text` gives.
    pub(crate) fn width_of(&self, text: impl Iterator<Item = u8>, spacing: u8) -> u16 {
        let pixels: i64 = text
            .map(|byte| i64::from(self.advance(byte, spacing)))
            .sum();
        u16::try_from((pixels + 7) / 8).unwrap_or(u16::MAX)
    }

    /// How far the glyph after `byte` starts from `byte`'s own start, in
    /// drawing pixels (8 per LED): its cell, its two gaps and `spacing`
    /// blank LEDs. This is the step [`Fourteen::coverage`] takes from one
    /// glyph to the next.
    pub(crate) const fn advance(&self, byte: u8, spacing: u8) -> i32 {
        self.cell(Glyph::of(byte)) + self.padding(spacing)
    }

    /// How far right of its start `byte`'s lit pixels can reach, in drawing
    /// pixels: its cell's last column, which the capsules can touch.
    pub(crate) const fn reach(&self, byte: u8) -> i32 {
        self.cell(Glyph::of(byte)) + 1
    }

    /// Segment number `index`'s capsule, as the preview's `add` makes it:
    /// from one point of the cell to another, pulled back by `gap` at both
    /// ends (straight segments) or `1.6 * gap` (diagonals).
    const fn capsule(&self, index: u32) -> Capsule {
        let (h, g) = (self.half, self.gap);
        let (left, right, centre) = (h, self.wide - h, self.wide / 2);
        let (top, middle, bottom) = (h, self.drawn / 2, self.drawn - h);
        match index {
            A => across(top, left + g, right - g),
            B => down(right, top + g, middle - g),
            C => down(right, middle + g, bottom - g),
            D => across(bottom, left + g, right - g),
            E => down(left, middle + g, bottom - g),
            F => down(left, top + g, middle - g),
            G1 => across(middle, left + g, centre - g),
            G2 => across(middle, centre + g, right - g),
            I => down(centre, top + g, middle - g),
            L => down(centre, middle + g, bottom - g),
            H => Capsule::Slope {
                flip_x: false,
                flip_y: false,
            },
            J => Capsule::Slope {
                flip_x: true,
                flip_y: false,
            },
            K => Capsule::Slope {
                flip_x: false,
                flip_y: true,
            },
            _ => Capsule::Slope {
                flip_x: true,
                flip_y: true,
            },
        }
    }

    /// The lit pixels of the LED whose top left pixel is (`left`, `top`) in
    /// `glyph`'s cell: bit `8 * row + column`.
    fn glyph_pixels(&self, glyph: Glyph, left: i32, top: i32) -> u64 {
        let centre = self.half + self.gap;
        match glyph {
            // The preview's dots sit at 0.3 and 0.7 of the drawing's height,
            // that is 12 / 5 and 28 / 5 of the height in LEDs.
            Glyph::Colon => {
                let rows = i32::from(self.height);
                self.dot(centre, 12 * rows, left, top) | self.dot(centre, 28 * rows, left, top)
            }
            Glyph::Dot => self.dot(centre, 5 * (self.drawn - self.half), left, top),
            Glyph::Segments(form) => {
                let mut lit = 0;
                let mut segments = form;
                while segments != 0 {
                    let index = segments.trailing_zeros();
                    segments &= segments - 1;
                    lit |= match self.capsule(index) {
                        Capsule::Across { y, from, to } => self.across(y, from, to, left, top),
                        Capsule::Down { x, from, to } => self.down(x, from, to, left, top),
                        Capsule::Slope { flip_x, flip_y } => self.slope(flip_x, flip_y, left, top),
                    };
                    if lit == FULL {
                        break;
                    }
                }
                lit
            }
        }
    }

    /// The rows `first..=last` (0 to 7) of the LED starting at pixel row
    /// `top` that lie in pixel rows `low..=high`; empty when `first > last`.
    fn rows(low: i32, high: i32, top: i32) -> (i32, i32) {
        ((low - top).max(0), (high - top).min(7))
    }

    /// `half^2 - offset^2` as a `u32`, for an `|offset| <= half`.
    #[allow(clippy::cast_sign_loss)]
    fn room(&self, offset: i32) -> u32 {
        (self.half * self.half - offset * offset) as u32
    }

    /// Pixels of a horizontal capsule.
    #[allow(clippy::cast_possible_wrap)]
    fn across(&self, y: i32, from: i32, to: i32, left: i32, top: i32) -> u64 {
        let h = self.half;
        if left > to + h || left + 7 < from - h {
            return 0;
        }
        let (first, last) = Fourteen::rows(y - h, y + h, top);
        if first > last {
            return 0;
        }
        // Columns all within the straight part: whole rows.
        if from <= left && left + 7 <= to {
            if first == 0 && last == 7 {
                return FULL;
            }
            let rows = (1u64 << (8 * (last - first + 1))) - 1;
            return rows << (8 * first);
        }
        let mut lit = 0;
        for row in first..=last {
            let reach = isqrt(self.room(top + row - y)) as i32;
            lit |= span(from - reach, to + reach, left) << (8 * row);
        }
        lit
    }

    /// Pixels of a vertical capsule.
    #[allow(clippy::cast_possible_wrap)]
    fn down(&self, x: i32, from: i32, to: i32, left: i32, top: i32) -> u64 {
        let h = self.half;
        if left > x + h || left + 7 < x - h {
            return 0;
        }
        let (first, last) = Fourteen::rows(from - h, to + h, top);
        if first > last {
            return 0;
        }
        let straight = span(x - h, x + h, left);
        // Rows all within the straight part: the same columns in each.
        if from <= top + first && top + last <= to {
            if straight == 0xFF && first == 0 && last == 7 {
                return FULL;
            }
            let mut copies = straight | straight << 8;
            copies |= copies << 16;
            copies |= copies << 32;
            let rows = if last - first == 7 {
                FULL
            } else {
                ((1u64 << (8 * (last - first + 1))) - 1) << (8 * first)
            };
            return copies & rows;
        }
        let mut lit = 0;
        for row in first..=last {
            let y = top + row;
            // Beyond an end, the round end's width at this row.
            let bits = if y < from || y > to {
                let end = if y < from { from } else { to };
                let reach = isqrt(self.room(y - end)) as i32;
                span(x - reach, x + reach, left)
            } else {
                straight
            };
            lit |= bits << (8 * row);
        }
        lit
    }

    /// Pixels of a round dot at (`x`, `y5 / 5`): those with
    /// `25 (px - x)^2 + (5 py - y5)^2 <= 25 half^2`.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn dot(&self, x: i32, y5: i32, left: i32, top: i32) -> u64 {
        let h = self.half;
        if left > x + h || left + 7 < x - h {
            return 0;
        }
        // Rows within half of y5 / 5, generously; the test below is exact.
        let (first, last) = Fourteen::rows(y5 / 5 - h, y5 / 5 + h + 1, top);
        let radius = 25 * i64::from(h) * i64::from(h);
        let mut lit = 0;
        for row in first..=last {
            let offset = i64::from(5 * (top + row) - y5);
            let room = radius - offset * offset;
            if room < 0 {
                continue;
            }
            // The largest r with 25 r^2 <= room.
            let reach = isqrt((room / 25) as u32) as i32;
            lit |= span(x - reach, x + reach, left) << (8 * row);
        }
        lit
    }

    /// Whether the pixel (`x`, `y`), from the corner of segment h, is lit by
    /// segment h: within `half` of its pulled-back line segment.
    fn on_slope(&self, x: i64, y: i64) -> bool {
        let dot = x * self.run + y * self.rise;
        let cross = x * self.rise - y * self.run;
        self.on_slope_with(x, y, dot, cross)
    }

    /// [`Fourteen::on_slope`] with the pixel's `dot = x * run + y * rise` and
    /// `cross = x * rise - y * run` already worked out (a row steps them by
    /// additions).
    ///
    /// Where a pixel lies along the diagonal is `dot / length`. The ends are
    /// pulled back by `1.6 * gap`, which is `8 * gap / 5`, so they lie at
    /// `dot = 8 * gap * length / 5` and `length^2` minus that. Multiplying by
    /// 5 keeps this exact in integers: `5 * dot` is compared with `low5` and
    /// `high5`, the two ends' `5 * dot` (`8 * gap * length` rounded up, and
    /// `5 * length^2` minus it). Below `low5` or above `high5` a pixel is past
    /// an end and is tested against that end's round cap; in between, against
    /// the sides, `|cross| <= half * length` (`band`). Normally the low end is
    /// the near one. When the pulled-back ends cross (`crossed`), the far end
    /// is the low one, and the capsule runs between the crossed ends, as the
    /// preview draws it.
    fn on_slope_with(&self, x: i64, y: i64, dot: i64, cross: i64) -> bool {
        let radius = 256 * i64::from(self.half);
        let (low, high) = (5 * dot < self.low5, 5 * dot > self.high5);
        if low || high {
            // Past an end: within its round end. The low end is the near
            // one unless the ends crossed.
            let (dx, dy) = if low == self.crossed {
                (
                    256 * (x - self.run) + self.end_x,
                    256 * (y - self.rise) + self.end_y,
                )
            } else {
                (256 * x - self.end_x, 256 * y - self.end_y)
            };
            dx * dx + dy * dy <= radius * radius
        } else {
            cross.abs() <= self.band
        }
    }

    /// Pixels of a diagonal: segment h mirrored by `flip_x` and `flip_y`.
    fn slope(&self, flip_x: bool, flip_y: bool, left: i32, top: i32) -> u64 {
        let h = self.half;
        // Pixel columns and rows in segment h's frame, from its corner
        // (half, half): mirrored about the cell's centre lines as needed.
        let column = |c: i32| -> i64 {
            let x = left + c;
            i64::from(if flip_x { self.wide - x } else { x } - h)
        };
        let line = |r: i32| -> i64 {
            let y = top + r;
            i64::from(if flip_y { self.drawn - y } else { y } - h)
        };
        let (x0, x7, y0, y7) = (column(0), column(7), line(0), line(7));
        let (x_low, x_high) = (x0.min(x7), x0.max(x7));
        let (y_low, y_high) = (y0.min(y7), y0.max(y7));
        if x_high < self.box_left
            || x_low > self.box_right
            || y_high < self.box_top
            || y_low > self.box_bottom
        {
            return 0;
        }
        // The LED's centre (doubled) further than half a stroke and half the
        // LED's diagonal (under 5 pixels) from the line: nothing lit.
        let cross2 = (x_low + x_high) * self.rise - (y_low + y_high) * self.run;
        if cross2.abs() > 2 * (self.band + 5 * (self.length + 1)) {
            return 0;
        }
        // All four corners lit: the capsule is convex, so every pixel is.
        if self.on_slope(x0, y0)
            && self.on_slope(x7, y0)
            && self.on_slope(x0, y7)
            && self.on_slope(x7, y7)
        {
            return FULL;
        }
        // Row by row: one dot and cross product per row, then each column
        // steps them by an addition. A row's lit pixels are one run (the
        // capsule is convex), so a row stops at the first unlit pixel after
        // a lit one.
        let step = if flip_x { -1 } else { 1 };
        let (dot_step, cross_step) = (step * self.run, step * self.rise);
        let mut lit = 0;
        for r in 0..8 {
            let y = line(r);
            let mut x = x0;
            let mut dot = x * self.run + y * self.rise;
            let mut cross = x * self.rise - y * self.run;
            let mut bits = 0u32;
            for c in 0..8 {
                if self.on_slope_with(x, y, dot, cross) {
                    bits |= 1 << c;
                } else if bits != 0 {
                    break;
                }
                x += step;
                dot += dot_step;
                cross += cross_step;
            }
            lit |= u64::from(bits) << (8 * r);
        }
        lit
    }

    /// How many of the LED at (`x`, `y`)'s 64 pixels are lit. (0, 0) is the
    /// top left of `text`.
    #[cfg(test)]
    pub(crate) fn coverage(&self, text: &[u8], spacing: u8, x: i32, y: i32) -> u16 {
        self.coverage_from(text, 0, spacing, x, y)
    }

    /// [`Fourteen::coverage`] with the first glyph starting `origin` drawing
    /// pixels (0 or more) right of LED column 0, so that part of a longer
    /// text can be drawn exactly where it would be in the whole.
    pub(crate) fn coverage_from(
        &self,
        text: &[u8],
        origin: i32,
        spacing: u8,
        x: i32,
        y: i32,
    ) -> u16 {
        // Past 2^27 LEDs is far beyond any text's width (at most `u16::MAX`),
        // and keeps the pixel positions below from overflowing.
        if x < 0 || y < 0 || y >= i32::from(self.height) || x >= 1 << 27 {
            return 0;
        }
        let (left, top) = (8 * x, 8 * y);
        let padding = self.padding(spacing);
        let mut lit = 0;
        let mut start = origin;
        for &byte in text {
            if start > left + 7 {
                break;
            }
            let glyph = Glyph::of(byte);
            let cell = self.cell(glyph);
            // A glyph's capsules reach from column 0 to column `cell` of its
            // cell, both included.
            if start + cell >= left {
                lit |= self.glyph_pixels(glyph, left - start, top);
                if lit == FULL {
                    break;
                }
            }
            start += cell + padding;
        }
        // Two 32-bit counts: cheaper than a 64-bit one on 32-bit chips.
        ((lit as u32).count_ones() + ((lit >> 32) as u32).count_ones()) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One `seg14` case of `segments.fixtures`.
    struct Case {
        height: u16,
        text: String,
        width: usize,
        rows: Vec<Vec<u16>>,
    }

    fn fixtures() -> Vec<Case> {
        let file = include_str!("segments.fixtures");
        let mut cases = Vec::new();
        for block in file.split("\n\n") {
            let mut lines = block.lines();
            let Some(header) = lines.next() else { continue };
            let Some(rest) = header.strip_prefix("case seg14 height=") else {
                continue;
            };
            let (height, text) = rest.split_once(" text=").unwrap();
            let size: Vec<usize> = lines
                .next()
                .unwrap()
                .split(' ')
                .map(|n| n.parse().unwrap())
                .collect();
            let rows: Vec<Vec<u16>> = lines
                .map(|line| line.split(' ').map(|n| n.parse().unwrap()).collect())
                .collect();
            assert_eq!(rows.len(), size[1]);
            assert!(rows.iter().all(|row| row.len() == size[0]));
            cases.push(Case {
                height: height.parse().unwrap(),
                text: text.to_string(),
                width: size[0],
                rows,
            });
        }
        cases
    }

    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn close_to_the_preview() {
        let cases = fixtures();
        assert_eq!(cases.len(), 30);
        let mut checked = 0;
        let mut wrong = 0;
        let mut worst = (0, String::new());
        let mut hard_wrong = Vec::new();
        for case in &cases {
            let face = Fourteen::new(case.height, false);
            let text = case.text.as_bytes();
            let name = format!("height={} text={}", case.height, case.text);
            assert_eq!(
                usize::from(face.width(text, 0)),
                case.width,
                "width of {name}"
            );
            assert_eq!(usize::from(face.height()), case.rows.len());
            for (y, row) in case.rows.iter().enumerate() {
                for (x, &expected) in row.iter().enumerate() {
                    checked += 1;
                    let got = face.coverage(text, 0, x as i32, y as i32);
                    let error = got.abs_diff(expected);
                    wrong += usize::from(error > 0);
                    if error > worst.0 {
                        worst = (
                            error,
                            format!("{name} at ({x}, {y}): got {got}, expected {expected}"),
                        );
                    }
                    // Hard edges: on when at least half covered, as the
                    // preview; only LEDs right at the half may differ.
                    if (got >= 32) != (expected >= 32) && expected.abs_diff(32) > 4 {
                        hard_wrong.push(format!(
                            "{name} at ({x}, {y}): got {got}, expected {expected}"
                        ));
                    }
                }
            }
        }
        println!(
            "{wrong} of {checked} LEDs differ, worst by {}: {}",
            worst.0, worst.1
        );
        assert!(
            worst.0 <= 4,
            "worst error {} of {checked} LEDs: {}",
            worst.0,
            worst.1
        );
        assert!(hard_wrong.is_empty(), "hard edges differ: {hard_wrong:?}");
        assert!(checked > 10_000);
    }

    /// JavaScript's `Math.round`: the nearest integer, halves toward +infinity.
    fn js_round(x: f64) -> i32 {
        let floor = x.floor();
        (if x - floor >= 0.5 { floor + 1.0 } else { floor }) as i32
    }

    /// The integer measurements equal the preview's float maths for every
    /// height, not just the ones in the fixtures, including whether the
    /// diagonals' pulled-back ends cross (they do at some small heights,
    /// such as 12).
    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn measurements_match_the_float_maths() {
        for height in 7..=3000u16 {
            let face = Fourteen::new(height, false);
            let rows = f64::from(height);
            let t = js_round(rows / 8.0).max(1) * 8;
            assert_eq!(2 * face.half, t, "t at {height}");
            assert_eq!(face.wide, js_round(rows * 0.62) * 8, "W at {height}");
            assert_eq!(face.gap, js_round(f64::from(t) * 0.9), "gap at {height}");
            // The diagonal's pull-back and its ends, against floats.
            let (run, rise) = (face.run as f64, face.rise as f64);
            let length = run.hypot(rise);
            let pull = f64::from(face.gap) * 1.6;
            assert_eq!(face.crossed, 2.0 * pull > length, "crossed at {height}");
            assert!((face.end_x as f64 - 256.0 * run * pull / length).abs() <= 0.5);
            assert!((face.end_y as f64 - 256.0 * rise * pull / length).abs() <= 0.5);
            let near5 = if face.crossed { face.high5 } else { face.low5 };
            // ceil(5 * pull * length), allowing for the float's rounding.
            let exact = 5.0 * pull * length;
            assert!(near5 as f64 >= exact - 1e-6 && (near5 as f64) < exact + 1.0);
            let length2 = face.run * face.run + face.rise * face.rise;
            assert_eq!(face.low5 + face.high5, 5 * length2);
            let band = f64::from(face.half) * length;
            assert!(face.band as f64 <= band && band < face.band as f64 + 1.0);
        }
    }

    /// Every LED of `a` and `b` is the same.
    fn same(face: &Fourteen, a: &[u8], b: &[u8]) -> bool {
        let width = i32::from(face.width(a, 0).max(face.width(b, 0)));
        (0..width).all(|x| {
            (0..i32::from(face.height()))
                .all(|y| face.coverage(a, 0, x, y) == face.coverage(b, 0, x, y))
        })
    }

    #[test]
    fn characters_map_like_the_preview() {
        let face = Fourteen::new(10, false);
        // Lower case shows as capitals.
        for (lower, upper) in b"abcxyz".iter().zip(b"ABCXYZ") {
            assert!(same(&face, &[*lower], &[*upper]));
        }
        // Characters with no form, and non-ASCII bytes, are blank but a
        // letter wide; ':', '.' and ',' are narrow.
        for blank in [b'@', b'#', b'~', b'(', 0xC2, 200] {
            assert!(same(&face, &[blank], b" "), "{blank}");
        }
        assert_eq!(face.width(b"@", 0), face.width(b"8", 0));
        assert!(face.width(b":", 0) < face.width(b"8", 0));
        assert!(same(&face, b",", b"."));
        // Every capital and digit lights something.
        for byte in (b'A'..=b'Z').chain(b'0'..=b'9') {
            assert!(!same(&face, &[byte], b" "), "{}", byte as char);
        }
    }

    #[test]
    fn spacing_and_outside() {
        let face = Fourteen::new(12, false);
        let plain = face.width(b"88", 0);
        assert_eq!(face.width(b"88", 2), plain + 4);
        assert_eq!(face.width(b"", 3), 0);
        // With spacing, the second letter moves right by the extra LEDs.
        let first = i32::from(face.width(b"8", 0));
        for y in 0..12 {
            for x in 0..first {
                assert_eq!(
                    face.coverage(b"88", 0, x + first, y),
                    face.coverage(b"88", 2, x + first + 2, y),
                    "({x}, {y})"
                );
            }
        }
        // Nothing outside the text.
        assert_eq!(face.coverage(b"8", 0, -1, 2), 0);
        assert_eq!(face.coverage(b"8", 0, 1, -1), 0);
        assert_eq!(face.coverage(b"8", 0, 1, 12), 0);
        assert_eq!(face.coverage(b"8", 0, first + 5, 3), 0);
        assert_eq!(face.coverage(b"8", 0, i32::MAX, 3), 0);
        // Too short counts as 7.
        assert_eq!(Fourteen::new(2, false).height(), 7);
    }

    /// Very tall faces don't overflow.
    #[test]
    fn tall_faces() {
        for height in [3000u16, 20_000, u16::MAX] {
            let face = Fourteen::new(height, false);
            assert!(face.width(b"X", 0) > 0);
            // H's left side is lit a quarter of the way down; X's top
            // left corner isn't (the diagonals are pulled back).
            let quarter = i32::from(height) / 4;
            assert_eq!(face.coverage(b"H", 0, 0, quarter), 64);
            assert_eq!(face.coverage(b"H", 0, 0, 0), 0);
            assert_eq!(face.coverage(b"X", 0, 0, 0), 0);
            // The middle of X's upper left diagonal is lit.
            let (x, y) = (
                (i64::from(face.half) + face.run / 2) / 8,
                (i64::from(face.half) + face.rise / 2) / 8,
            );
            assert_eq!(face.coverage(b"X", 0, x as i32, y as i32), 64);
        }
    }

    /// The preview's `renderSeg14` before shrinking, transcribed into floats:
    /// how many of each LED's 64 pixels are lit, row by row.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    fn float_reference(text: &str, height: u16) -> Vec<Vec<u16>> {
        let rows = f64::from(height.max(7));
        let drawn = rows * 8.0;
        let t = js_round(rows / 8.0).max(1) as f64 * 8.0;
        let wide = f64::from(js_round(rows * 0.62)) * 8.0;
        let gap = f64::from(js_round(t * 0.9));
        let upper = text.to_uppercase();
        let narrow = |c: char| c == ':' || c == '.' || c == ',';
        let cells: Vec<f64> = upper
            .chars()
            .map(|c| if narrow(c) { t + 2.0 * gap } else { wide })
            .collect();
        let total: f64 = cells.iter().map(|w| w + 2.0 * gap).sum();
        let width = (total / 8.0).ceil() as usize * 8;
        let height = drawn as usize;
        let mut big = vec![false; width * height];
        let h = t / 2.0;
        let mut caps: Vec<[f64; 4]> = Vec::new();
        let mut x0 = 0.0;
        for (c, w) in upper.chars().zip(&cells) {
            let (l, r, centre) = (x0 + h, x0 + w - h, x0 + w / 2.0);
            let (top, middle, bottom) = (h, drawn / 2.0, drawn - h);
            let mut add = |ax: f64, ay: f64, bx: f64, by: f64, shrink: f64| {
                let (dx, dy) = (bx - ax, by - ay);
                let s = shrink / dx.hypot(dy);
                caps.push([ax + dx * s, ay + dy * s, bx - dx * s, by - dy * s]);
            };
            if c == ':' {
                caps.push([centre, drawn * 0.3, centre, drawn * 0.3]);
                caps.push([centre, drawn * 0.7, centre, drawn * 0.7]);
            } else if c == '.' || c == ',' {
                caps.push([centre, bottom, centre, bottom]);
            } else {
                let form = if c.is_ascii() { TABLE[c as usize] } else { 0 };
                let (g, d) = (gap, gap * 1.6);
                for index in 0..14 {
                    if form & (1 << index) == 0 {
                        continue;
                    }
                    match index {
                        A => add(l, top, r, top, g),
                        D => add(l, bottom, r, bottom, g),
                        F => add(l, top, l, middle, g),
                        E => add(l, middle, l, bottom, g),
                        B => add(r, top, r, middle, g),
                        C => add(r, middle, r, bottom, g),
                        G1 => add(l, middle, centre, middle, g),
                        G2 => add(centre, middle, r, middle, g),
                        I => add(centre, top, centre, middle, g),
                        L => add(centre, middle, centre, bottom, g),
                        H => add(l, top, centre, middle, d),
                        J => add(r, top, centre, middle, d),
                        K => add(l, bottom, centre, middle, d),
                        _ => add(r, bottom, centre, middle, d),
                    }
                }
            }
            x0 += w + 2.0 * gap;
        }
        for [ax, ay, bx, by] in caps {
            let min_x = (ax.min(bx) - h).floor().max(0.0) as usize;
            let max_x = ((ax.max(bx) + h).ceil() as usize).min(width - 1);
            let min_y = (ay.min(by) - h).floor().max(0.0) as usize;
            let max_y = ((ay.max(by) + h).ceil() as usize).min(height - 1);
            let (dx, dy) = (bx - ax, by - ay);
            let l2 = if dx * dx + dy * dy == 0.0 {
                1.0
            } else {
                dx * dx + dy * dy
            };
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let (px, py) = (x as f64, y as f64);
                    let u = (((px - ax) * dx + (py - ay) * dy) / l2).clamp(0.0, 1.0);
                    if (px - ax - u * dx).hypot(py - ay - u * dy) <= h {
                        big[y * width + x] = true;
                    }
                }
            }
        }
        let mut leds = vec![vec![0u16; width / 8]; height / 8];
        for y in 0..height {
            for x in 0..width {
                leds[y / 8][x / 8] += u16::from(big[y * width + x]);
            }
        }
        leds
    }

    /// Heights the fixtures leave out (including 12, where the diagonals'
    /// ends cross) against the float transcription of the preview.
    #[test]
    #[allow(clippy::cast_possible_wrap)]
    fn close_to_the_float_maths() {
        let text = "0123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ.+-*/<>?!$%'\"_=,@";
        let mut worst = (0, String::new());
        let mut wrong = 0;
        let mut checked = 0;
        for height in (7..=48).chain([64, 100]) {
            let face = Fourteen::new(height, false);
            let expected = float_reference(text, height);
            assert_eq!(
                usize::from(face.width(text.as_bytes(), 0)),
                expected[0].len()
            );
            for (y, row) in expected.iter().enumerate() {
                for (x, &want) in row.iter().enumerate() {
                    checked += 1;
                    let got = face.coverage(text.as_bytes(), 0, x as i32, y as i32);
                    let error = got.abs_diff(want);
                    wrong += usize::from(error > 0);
                    if error > worst.0 {
                        worst = (
                            error,
                            format!("height {height} at ({x}, {y}): got {got}, expected {want}"),
                        );
                    }
                }
            }
        }
        println!(
            "{wrong} of {checked} LEDs differ, worst by {}: {}",
            worst.0, worst.1
        );
        assert!(worst.0 <= 4, "worst error {}: {}", worst.0, worst.1);
    }

    /// The diagonals' row stepping (additions, stopping after the lit run)
    /// lights exactly the pixels a plain per-pixel test does.
    #[test]
    fn diagonal_rows_match_single_pixels() {
        for height in (7..=72).chain([100, 128]) {
            let face = Fourteen::new(height, false);
            let rows = i32::from(height) * 8;
            for (flip_x, flip_y) in [(false, false), (true, false), (false, true), (true, true)] {
                for top in (0..rows).step_by(8) {
                    for left in (-8..face.wide + 8).step_by(8) {
                        let mut plain = 0u64;
                        for r in 0..8 {
                            for c in 0..8 {
                                let (x, y) = (left + c, top + r);
                                let x = if flip_x { face.wide - x } else { x } - face.half;
                                let y = if flip_y { face.drawn - y } else { y } - face.half;
                                if face.on_slope(i64::from(x), i64::from(y)) {
                                    plain |= 1 << (8 * r + c);
                                }
                            }
                        }
                        assert_eq!(
                            face.slope(flip_x, flip_y, left, top),
                            plain,
                            "height {height}, LED at ({left}, {top}), flips {flip_x} {flip_y}"
                        );
                    }
                }
            }
        }
    }

    /// Host timing per LED, for comparing changes (not a pass/fail test):
    /// `cargo test --release -p ledkit-text --lib timing -- --ignored --nocapture`.
    #[test]
    #[ignore = "timing, run by hand in release"]
    #[allow(clippy::cast_precision_loss)]
    fn timing() {
        for height in [64u16, 16] {
            for text in [&b"XXXX"[..], b"8888"] {
                let face = Fourteen::new(height, false);
                let width = i32::from(face.width(text, 0));
                let leds = f64::from(width) * f64::from(height);
                let mut total = 0u64;
                let rounds = 200;
                let start = std::time::Instant::now();
                for _ in 0..rounds {
                    for y in 0..i32::from(height) {
                        for x in 0..width {
                            total += u64::from(face.coverage(std::hint::black_box(text), 0, x, y));
                        }
                    }
                }
                let ns = start.elapsed().as_nanos() as f64 / (leds * f64::from(rounds));
                println!(
                    "height {height} {}: {ns:.1} ns/LED ({total})",
                    std::str::from_utf8(text).unwrap()
                );
            }
            // The seven-segment face, for reference.
            let seven = crate::faces::seven::Seven::new(height, false, false);
            let width = i32::from(seven.width(b"8888", 0));
            let leds = f64::from(width) * f64::from(height);
            let mut total = 0u64;
            let start = std::time::Instant::now();
            for _ in 0..200 {
                for y in 0..i32::from(height) {
                    for x in 0..width {
                        let text = std::hint::black_box(&b"8888"[..]);
                        total += u64::from(seven.coverage(text, 0, x, y));
                    }
                }
            }
            let ns = start.elapsed().as_nanos() as f64 / (leds * 200.0);
            println!("height {height} 7seg-8888: {ns:.1} ns/LED ({total})");
        }
    }
}
