//! The seven-segment face: letters drawn like a clock or calculator display.
//!
//! An integer port of the preview page's `renderSeg` (`site/text-preview/glyphs.js`).
//! The letters are drawn at 8 pixels per LED and each LED shows how many of its
//! 8 x 8 pixels are lit (anti-aliased), or drawn straight at 1 pixel per LED
//! (hard-edged). Nothing is drawn into a buffer: each LED works out which
//! letter it is over and counts lit pixels row by row.

/// Segment bits: a (top), b (top right), c (bottom right), d (bottom),
/// e (bottom left), f (top left), g (middle).
const A: u8 = 1;
const B: u8 = 2;
const C: u8 = 4;
const D: u8 = 8;
const E: u8 = 16;
const F: u8 = 32;
const G: u8 = 64;
/// Marks a table entry as present (a space has a form: no segments).
const PRESENT: u8 = 128;

/// The preview's `SEGS`: which segments each character lights. Its degree
/// sign is left out: text here is bytes, and non-ASCII bytes are blank.
const FORMS: [(u8, &str); 56] = [
    (b'0', "abcdef"),
    (b'1', "bc"),
    (b'2', "abdeg"),
    (b'3', "abcdg"),
    (b'4', "bcfg"),
    (b'5', "acdfg"),
    (b'6', "acdefg"),
    (b'7', "abc"),
    (b'8', "abcdefg"),
    (b'9', "abcdfg"),
    (b'-', "g"),
    (b'_', "d"),
    (b'=', "dg"),
    (b' ', ""),
    (b'A', "abcefg"),
    (b'B', "cdefg"),
    (b'C', "adef"),
    (b'D', "bcdeg"),
    (b'E', "adefg"),
    (b'F', "aefg"),
    (b'G', "acdef"),
    (b'H', "bcefg"),
    (b'I', "ef"),
    (b'J', "bcde"),
    (b'L', "def"),
    (b'N', "ceg"),
    (b'O', "abcdef"),
    (b'P', "abefg"),
    (b'Q', "abcfg"),
    (b'R', "eg"),
    (b'S', "acdfg"),
    (b'T', "defg"),
    (b'U', "bcdef"),
    (b'Y', "bcdfg"),
    (b'Z', "abdeg"),
    (b'a', "abcdeg"),
    (b'b', "cdefg"),
    (b'c', "deg"),
    (b'd', "bcdeg"),
    (b'e', "abdefg"),
    (b'f', "aefg"),
    (b'g', "abcdfg"),
    (b'h', "cefg"),
    (b'i', "c"),
    (b'j', "bcd"),
    (b'l', "ef"),
    (b'n', "ceg"),
    (b'o', "cdeg"),
    (b'p', "abefg"),
    (b'q', "abcfg"),
    (b'r', "eg"),
    (b's', "acdfg"),
    (b't', "defg"),
    (b'u', "cde"),
    (b'y', "bcdfg"),
    (b'z', "abdeg"),
];

/// Builds a [`PRESENT`]-marked segment mask from the preview's letters `a`..`g`.
const fn mask(letters: &str) -> u8 {
    let letters = letters.as_bytes();
    let mut mask = PRESENT;
    let mut i = 0;
    while i < letters.len() {
        mask |= 1 << (letters[i] - b'a');
        i += 1;
    }
    mask
}

/// [`FORMS`] as a table by ASCII code: 0 where a character has no form.
const TABLE: [u8; 128] = {
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
    /// A digit-sized cell lighting these segments (none: blank).
    Segments(u8),
    /// A narrow cell with two dots.
    Colon,
    /// A narrow cell with one dot at the bottom.
    Dot,
}

impl Glyph {
    /// As the preview: the character itself if it has a form, otherwise its
    /// upper case. Characters with no form (and non-ASCII bytes) are blank.
    const fn of(byte: u8) -> Glyph {
        match byte {
            b':' => Glyph::Colon,
            b'.' => Glyph::Dot,
            128.. => Glyph::Segments(0),
            _ => {
                let own = TABLE[byte as usize];
                let form = if own & PRESENT != 0 {
                    own
                } else {
                    TABLE[byte.to_ascii_uppercase() as usize]
                };
                Glyph::Segments(form & !PRESENT)
            }
        }
    }
}

/// `round(numerator / denominator)` with halves rounded up, like JavaScript's
/// `Math.round`, for a non-negative numerator.
const fn round_div(numerator: i32, denominator: i32) -> i32 {
    (2 * numerator + denominator) / (2 * denominator)
}

const fn max(a: i32, b: i32) -> i32 {
    if a > b {
        a
    } else {
        b
    }
}

/// How many of the pixels `start..end` fall inside `low..high`.
const fn overlap(start: i32, end: i32, low: i32, high: i32) -> i32 {
    let from = max(start, low);
    let to = if end < high { end } else { high };
    max(to - from, 0)
}

/// A seven-segment face at one height, with its drawing measurements worked
/// out once.
///
/// All measurements are in drawing pixels: 8 per LED when anti-aliased (the
/// preview's `renderSeg` at height `8 * height`), 1 per LED when hard-edged
/// (`renderSeg` at `height`).
#[derive(Copy, Clone)]
pub(crate) struct Seven {
    /// LEDs tall, at least 5.
    height: u16,
    slanted: bool,
    hard: bool,
    /// Drawing pixels per LED: 8, or 1 when hard-edged.
    scale: i32,
    /// The drawing's height in pixels.
    drawn: i32,
    /// Stroke thickness.
    thick: i32,
    /// A digit cell's width.
    wide: i32,
    /// The gap on each side of a cell.
    gap: i32,
    /// The middle segment's top row.
    middle: i32,
    /// How far the top row leans right (0 when upright).
    skew: i32,
    /// The top rows of the colon's two dots.
    upper_dot: i32,
    lower_dot: i32,
}

impl Seven {
    /// A face `height` LEDs tall (below 5 counts as 5).
    pub(crate) const fn new(height: u16, slanted: bool, hard: bool) -> Seven {
        let height = if height < 5 { 5 } else { height };
        let scale = if hard { 1 } else { 8 };
        let drawn = height as i32 * scale;
        // The preview's measurements, rounded the way `Math.round` does:
        // t = round(H / 7), W = round(0.55 H), gap = round(0.8 t),
        // skew = round(0.18 H), mid = floor((H - t) / 2).
        let thick = max(1, round_div(drawn, 7));
        let wide = max(3, round_div(55 * drawn, 100));
        let gap = max(1, round_div(8 * thick, 10));
        let skew = if slanted {
            round_div(18 * drawn, 100)
        } else {
            0
        };
        Seven {
            height,
            slanted,
            hard,
            scale,
            drawn,
            thick,
            wide,
            gap,
            middle: (drawn - thick) / 2,
            skew,
            upper_dot: round_div(28 * drawn, 100) - (thick >> 1),
            lower_dot: round_div(72 * drawn, 100) - (thick >> 1),
        }
    }

    /// LEDs tall.
    pub(crate) const fn height(&self) -> u16 {
        self.height
    }

    /// The same face, hard-edged.
    pub(crate) const fn hardened(&self) -> Seven {
        Seven::new(self.height, self.slanted, true)
    }

    /// Whether each LED is fully on or off.
    pub(crate) const fn hard(&self) -> bool {
        self.hard
    }

    /// Whether the letters lean right.
    pub(crate) const fn slanted(&self) -> bool {
        self.slanted
    }

    /// How wide a glyph's cell is, in drawing pixels.
    const fn cell(&self, glyph: Glyph) -> i32 {
        match glyph {
            Glyph::Segments(_) => self.wide,
            Glyph::Colon | Glyph::Dot => self.thick + 2 * self.gap,
        }
    }

    /// How far past its cell the next glyph starts, in drawing pixels.
    const fn padding(&self, spacing: u8) -> i32 {
        2 * self.gap + spacing as i32 * self.scale
    }

    /// How many LEDs wide `text` is: each glyph's cell, its two gaps and
    /// `spacing` blank LEDs, plus the slant's lean once. 0 for no text.
    pub(crate) fn width(&self, text: &[u8], spacing: u8) -> u16 {
        if text.is_empty() {
            return 0;
        }
        let padding = i64::from(self.padding(spacing));
        let cells: i64 = text
            .iter()
            .map(|&byte| i64::from(self.cell(Glyph::of(byte))) + padding)
            .sum();
        let pixels = cells + i64::from(self.skew);
        let scale = i64::from(self.scale);
        u16::try_from((pixels + scale - 1) / scale).unwrap_or(u16::MAX)
    }

    /// How many pixels of `glyph` (upright, its cell starting at column 0)
    /// row `row` lights inside columns `low..high`.
    fn lit_in_row(&self, glyph: Glyph, row: i32, low: i32, high: i32) -> i32 {
        let (t, w, h, mid) = (self.thick, self.wide, self.drawn, self.middle);
        let within = |top: i32, bottom: i32| row >= top && row < bottom;
        match glyph {
            Glyph::Colon => {
                let lit = within(self.upper_dot, self.upper_dot + t)
                    || within(self.lower_dot, self.lower_dot + t);
                if lit {
                    overlap(self.gap, self.gap + t, low, high)
                } else {
                    0
                }
            }
            Glyph::Dot => {
                if within(h - t, h) {
                    overlap(self.gap, self.gap + t, low, high)
                } else {
                    0
                }
            }
            Glyph::Segments(form) => {
                // The segments never overlap, so their counts add up.
                let mut lit = 0;
                let mut segment = |bit: u8, top: i32, bottom: i32, left: i32, right: i32| {
                    if form & bit != 0 && within(top, bottom) {
                        lit += overlap(left, right, low, high);
                    }
                };
                segment(A, 0, t, t, w - t);
                segment(G, mid, mid + t, t, w - t);
                segment(D, h - t, h, t, w - t);
                segment(F, t, mid, 0, t);
                segment(B, t, mid, w - t, w);
                segment(E, mid + t, h - t, 0, t);
                segment(C, mid + t, h - t, w - t, w);
                lit
            }
        }
    }

    /// How far the slant moves pixel row `row` right, as
    /// `(shift, remainder)`: the preview's `round(skew * (1 - row / (H - 1)))`,
    /// which is `round(skew * (H - 1 - row) / (H - 1))`, kept as a quotient and
    /// remainder so [`Seven::lean_down`] can step to the next row without
    /// dividing.
    const fn lean_at(&self, row: i32) -> (i32, i64) {
        let last = (self.drawn - 1) as i64;
        let numerator = 2 * self.skew as i64 * (last - row as i64) + last;
        ((numerator / (2 * last)) as i32, numerator % (2 * last))
    }

    /// [`Seven::lean_at`] one row further down: the same shift or one less,
    /// since the lean (`skew`) is smaller than the height.
    const fn lean_down(&self, (shift, rest): (i32, i64)) -> (i32, i64) {
        let rest = rest - 2 * self.skew as i64;
        if rest < 0 {
            (shift - 1, rest + 2 * (self.drawn - 1) as i64)
        } else {
            (shift, rest)
        }
    }

    /// The shift of pixel row `row`, whose [`Seven::lean_at`] state is `lean`.
    ///
    /// `lean.0` rounds an exact half up. The preview works in floating
    /// point, where an exact half (only possible hard-edged, at odd heights)
    /// can come out a hair under and round down; [`float_lean_rounds_up`]
    /// settles those rows the way the preview does.
    fn shift(&self, lean: (i32, i64), row: i32) -> i32 {
        if lean.1 == 0 && !float_lean_rounds_up(self.skew, row, self.drawn - 1) {
            lean.0 - 1
        } else {
            lean.0
        }
    }

    /// How many of the LED at (`x`, `y`)'s pixels are lit, in 64ths: 0..=64
    /// anti-aliased, 0 or 64 hard-edged. (0, 0) is the top left of `text`.
    #[allow(clippy::cast_sign_loss)] // `lit` is never negative.
    pub(crate) fn coverage(&self, text: &[u8], spacing: u8, x: i32, y: i32) -> u16 {
        if x < 0 || y < 0 || y >= i32::from(self.height) {
            return 0;
        }
        let scale = self.scale;
        let (left, top) = (x * scale, y * scale);
        let right = left + scale;
        let first = self.lean_at(top);
        // The LED's bottom row shifts least, so this window (in upright
        // columns) holds every pixel the LED can see.
        let mut bottom = first;
        for _ in 1..scale {
            bottom = self.lean_down(bottom);
        }
        let window_low = left - self.shift(first, top);
        let window_high = right - self.shift(bottom, top + scale - 1);

        let padding = self.padding(spacing);
        let mut lit = 0;
        let mut start = 0;
        for &byte in text {
            if start >= window_high {
                break;
            }
            let glyph = Glyph::of(byte);
            let cell = self.cell(glyph);
            if start + cell > window_low {
                let mut lean = first;
                for row in top..top + scale {
                    let shift = self.shift(lean, row);
                    let (low, high) = (left - shift - start, right - shift - start);
                    lit += self.lit_in_row(glyph, row, low, high);
                    lean = self.lean_down(lean);
                }
            }
            start += cell + padding;
        }
        (lit * 64 / (scale * scale)) as u16
    }
}

/// Rounds `value * 2^exponent` to the 53 significant bits of a JavaScript
/// number (a 64-bit float), ties to even, as `(value, exponent)` again.
#[allow(clippy::cast_possible_wrap)] // `leading_zeros` is at most 128.
fn to_float(value: u128, exponent: i32) -> (u128, i32) {
    let bits = 128 - value.leading_zeros() as i32;
    if bits <= 53 {
        return (value, exponent);
    }
    let dropped = bits - 53;
    let kept = value >> dropped;
    let rest = value & ((1 << dropped) - 1);
    let half = 1 << (dropped - 1);
    let round_up = rest > half || (rest == half && kept & 1 == 1);
    (kept + u128::from(round_up), exponent + dropped)
}

/// Whether the preview's `Math.round(skew * (1 - row / last))` rounds up
/// where the exact value is a half, by redoing its three float operations
/// (divide, subtract, multiply) with integers, rounding each like a float.
///
/// Only called for rows where the exact value is a half, so `0 < row < last`.
#[allow(clippy::cast_sign_loss)] // All three arguments are positive here.
fn float_lean_rounds_up(skew: i32, row: i32, last: i32) -> bool {
    let (skew, row, last) = (skew as u128, row as u128, last as u128);
    // row / last: enough quotient bits (row >= 1, last < 2^20 gives at least
    // 60), then one sticky bit standing for any remainder.
    let scaled = row << 80;
    let sticky = u128::from(scaled % last != 0);
    let ratio = to_float(((scaled / last) << 1) | sticky, -81);
    // 1 - ratio, exact in integers, then rounded.
    let one = 1u128 << -ratio.1;
    let rest = to_float(one - ratio.0, ratio.1);
    // skew * rest.
    let product = to_float(skew * rest.0, rest.1);
    // The exact value is n + 1/2 with n = floor(skew * (last - row) / last).
    // Compare product >= n + 1/2, as product * 2 >= 2n + 1.
    let twice_n_plus_one = 2 * (skew * (last - row) / last) + 1;
    let exponent = product.1 + 1;
    if exponent >= 0 {
        product.0 << exponent >= twice_n_plus_one
    } else {
        product.0 >= twice_n_plus_one << -exponent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One case of `segments.fixtures`.
    struct Case {
        name: String,
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
            let rest = header.strip_prefix("case ").expect("a case header");
            let (name, rest) = rest.split_once(" height=").unwrap();
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
                name: name.to_string(),
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
    fn matches_the_preview() {
        let mut checked = 0;
        let mut wrong = 0;
        let mut first_wrong = None;
        for case in fixtures() {
            let (slanted, hard) = match case.name.as_str() {
                "seg7" => (false, false),
                "seg7slant" => (true, false),
                "seg7hard" => (false, true),
                "seg7slanthard" => (true, true),
                _ => continue,
            };
            let face = Seven::new(case.height, slanted, hard);
            let text = case.text.as_bytes();
            assert_eq!(
                usize::from(face.width(text, 0)),
                case.width,
                "width of {} height={} text={}",
                case.name,
                case.height,
                case.text
            );
            assert_eq!(usize::from(face.height()), case.rows.len());
            for (y, row) in case.rows.iter().enumerate() {
                for (x, &expected) in row.iter().enumerate() {
                    checked += 1;
                    let got = face.coverage(text, 0, x as i32, y as i32);
                    if got != expected {
                        wrong += 1;
                        first_wrong.get_or_insert(format!(
                            "{} height={} text={} at ({x}, {y}): got {got}, expected {expected}",
                            case.name, case.height, case.text
                        ));
                    }
                }
            }
        }
        assert_eq!(
            wrong, 0,
            "{wrong} of {checked} LEDs differ; first: {first_wrong:?}"
        );
        assert!(checked > 10_000);
    }

    /// JavaScript's `Math.round`: the nearest integer, halves toward +infinity.
    fn js_round(x: f64) -> i32 {
        let floor = x.floor();
        (if x - floor >= 0.5 { floor + 1.0 } else { floor }) as i32
    }

    /// The integer measurements and slant equal the preview's float maths for
    /// every height, not just the ones in the fixtures.
    #[test]
    fn measurements_match_the_float_maths() {
        for height in 5..=3000u16 {
            for hard in [false, true] {
                let face = Seven::new(height, true, hard);
                let h = f64::from(face.drawn);
                let t = js_round(h / 7.0).max(1);
                assert_eq!(face.thick, t, "t at {h}");
                assert_eq!(face.wide, js_round(h * 0.55).max(3), "W at {h}");
                assert_eq!(face.gap, js_round(f64::from(t) * 0.8).max(1), "gap at {h}");
                let skew = js_round(h * 0.18);
                assert_eq!(face.skew, skew, "skew at {h}");
                assert_eq!(face.upper_dot, js_round(h * 0.28) - (t >> 1));
                assert_eq!(face.lower_dot, js_round(h * 0.72) - (t >> 1));
                let mut step = face.lean_at(0);
                for row in 0..face.drawn {
                    let lean = js_round(f64::from(skew) * (1.0 - f64::from(row) / (h - 1.0)));
                    assert_eq!(face.lean_at(row), step, "stepping at {h}, row {row}");
                    assert_eq!(face.shift(step, row), lean, "lean at {h}, row {row}");
                    step = face.lean_down(step);
                }
            }
        }
    }

    /// Lights the LED at (x, y) of `text`: does any part of it show?
    fn lit(face: &Seven, text: &[u8], x: i32, y: i32) -> bool {
        face.coverage(text, 0, x, y) > 0
    }

    /// Every LED of `a` and `b` (one character each) is the same.
    fn same(face: &Seven, a: &[u8], b: &[u8]) -> bool {
        let width = i32::from(face.width(a, 0).max(face.width(b, 0)));
        (0..width).all(|x| {
            (0..i32::from(face.height()))
                .all(|y| face.coverage(a, 0, x, y) == face.coverage(b, 0, x, y))
        })
    }

    #[test]
    fn characters_map_like_the_preview() {
        let face = Seven::new(10, false, false);
        // Lower case has its own forms: 'c' and 'e' are the small c and e,
        // 'b' looks like 'B' (both are the small b).
        assert!(!same(&face, b"c", b"C") && !same(&face, b"e", b"E"));
        assert!(same(&face, b"b", b"B"));
        // K, M, V, W and X have no form in either case: blank.
        for letter in *b"kKmMvVwWxX" {
            assert!(same(&face, &[letter], b" "), "{}", letter as char);
        }
        // Non-ASCII bytes (such as the first byte of a UTF-8 degree sign) are blank.
        assert!(same(&face, &[0xC2], b" "));
        assert!(same(&face, &[200], b" "));
        // A blank still takes a digit's width; colon and dot are narrower.
        assert_eq!(face.width(b"k", 0), face.width(b"8", 0));
        assert!(face.width(b":", 0) < face.width(b"8", 0));
        assert!((0..face.width(b"8", 0)).any(|x| lit(&face, b"8", i32::from(x), 0)));
    }

    #[test]
    fn spacing_and_edges() {
        for hard in [false, true] {
            let face = Seven::new(8, false, hard);
            let plain = face.width(b"88", 0);
            assert_eq!(face.width(b"88", 2), plain + 4);
            assert_eq!(face.width(b"", 3), 0);
            // With spacing the second digit moves right by the extra LEDs.
            let first = face.width(b"8", 0);
            for y in 0..8 {
                for x in 0..i32::from(first) {
                    assert_eq!(
                        face.coverage(b"88", 0, x + i32::from(first), y),
                        face.coverage(b"88", 2, x + i32::from(first) + 2, y),
                        "hard {hard}, ({x}, {y})"
                    );
                }
            }
            // Nothing outside the text.
            assert_eq!(face.coverage(b"8", 0, -1, 2), 0);
            assert_eq!(face.coverage(b"8", 0, 1, -1), 0);
            assert_eq!(face.coverage(b"8", 0, 1, 8), 0);
            assert_eq!(face.coverage(b"8", 0, i32::from(first) + 5, 0), 0);
        }
        // Too short counts as 5.
        assert_eq!(Seven::new(2, false, false).height(), 5);
    }
}
