//! Bitmap faces: fonts baked at four sizes into coverage tables.

/// One font at one size: `width` by `height` bytes per glyph, for the 95
/// printable ASCII characters (codes 32 to 126) in order.
pub(crate) struct BakedSize {
    pub width: u8,
    pub height: u8,
    /// Row by row; each byte is how many of 16 samples are inside the glyph.
    pub data: &'static [u8],
}

/// A bitmap font baked at 1x, 1.25x, 1.5x and 2x.
pub(crate) struct BitmapFace {
    pub name: &'static str,
    pub sizes: [BakedSize; 4],
}
