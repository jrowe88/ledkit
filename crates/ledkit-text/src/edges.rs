//! [`Edges`]: how sharp the edges of letters look.

/// How sharp the edges of letters look on your LEDs.
///
/// Fonts are drawn larger than your LEDs and then shrunk, so a letter's edge
/// often lands half way across an LED. `Soft` lets that LED glow at half
/// brightness (smooth, a little blurry). `VeryCrisp` pushes it to fully on or
/// fully off (sharp, a little jagged). `Medium` is a good start.
///
/// ```
/// use ledkit_text::prelude::*;
///
/// let font = Font::X11_5X8.edges(Edges::Crisp);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum Edges {
    /// The smoothest look.
    Soft,
    /// A balance of smooth and sharp (the default).
    #[default]
    Medium,
    /// Sharper.
    Crisp,
    /// The sharpest look, nearly all on or off.
    VeryCrisp,
}

/// Turns "how much of this LED the letter covers" into a brightness.
///
/// `covered` out of `out_of` samples are inside the letter. The share
/// `v = covered / out_of` is stretched around one half:
/// `clamp((v - 0.5) * k + 0.5)`, where `k` is 1, 2, 3.5 or 6 for
/// `Soft`, `Medium`, `Crisp` and `VeryCrisp`. The result is 0..=255.
/// Integer maths only: with `k2 = 2k` this is
/// `((2 * covered - out_of) * k2 + 2 * out_of) * 255 / (4 * out_of)`.
#[allow(clippy::cast_sign_loss)] // `rounded` is between 0 and 255 here.
pub(crate) const fn sharpen(covered: u16, out_of: u16, edges: Edges) -> u8 {
    let k2: i32 = match edges {
        Edges::Soft => 2,
        Edges::Medium => 4,
        Edges::Crisp => 7,
        Edges::VeryCrisp => 12,
    };
    let covered = covered as i32;
    let out_of = out_of as i32;
    let top = ((2 * covered - out_of) * k2 + 2 * out_of) * 255;
    let bottom = 4 * out_of;
    // Round to nearest; below zero (`top` negative) clamps to 0.
    let rounded = if top <= 0 {
        0
    } else {
        (2 * top + bottom) / (2 * bottom)
    };
    if rounded > 255 {
        255
    } else {
        rounded as u8
    }
}

#[cfg(test)]
#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    #[test]
    fn sharpen_matches_the_preview() {
        let cases = [
            (Edges::Soft, 1.0_f32),
            (Edges::Medium, 2.0),
            (Edges::Crisp, 3.5),
            (Edges::VeryCrisp, 6.0),
        ];
        for (edges, k) in cases {
            for covered in 0..=16u16 {
                let v = f32::from(covered) / 16.0;
                let expected = (((v - 0.5) * k + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
                assert_eq!(
                    sharpen(covered, 16, edges),
                    expected,
                    "covered {covered}, {edges:?}"
                );
            }
        }
    }
}
