//! Ready-made color maps, by name.
//!
//! Hand any of these to an effect that takes colors, or read a color from one
//! yourself with `color_at(position)` (position 0 to 255).
//!
//! | Name | Looks like | Wraps |
//! |---|---|---|
//! | [`RAINBOW`] | red, orange, yellow, green, blue, purple | yes |
//! | [`RED_ORANGE`] | deep red to orange | no |
//! | [`JET`] | MATLAB's "jet": dark blue, blue, cyan, yellow, red, dark red | no |
//! | [`FIRE`] | black, dark red, orange, yellow, pale yellow | no |
//! | [`HEAT`] | black, red, yellow, white | no |
//! | [`LAVA`] | black and dark reds with flashes of orange and white | yes |
//! | [`OCEAN`] | deep blues, sea greens and aqua | yes |
//! | [`FOREST`] | greens | yes |
//! | [`CLOUD`] | dark blue sky with white and light blue | yes |
//! | [`PARTY`] | purple, pink, red, orange, yellow, no green | yes |
//! | [`SUNSET`] | purple, crimson, orange, gold | no |
//! | [`ICE`] | navy, bright blue, white | no |
//! | [`VIRIDIS`] | dark purple, blue, teal, green, yellow | no |
//!
//! "Wraps" means the end blends smoothly back into the start (see
//! [`ColorMap::wraps`](crate::ColorMap::wraps)).
//!
//! # Example
//!
//! ```
//! use ledkit_color::{palettes, Rgb};
//!
//! // Spread the jet colors along a 10-LED strip.
//! let mut strip = [Rgb::BLACK; 10];
//! for (i, pixel) in strip.iter_mut().enumerate() {
//!     *pixel = palettes::JET.color_at((i * 255 / 9) as u8);
//! }
//! assert_eq!(strip[0], Rgb::new(0, 0, 143));
//! ```

use crate::{Gradient, Palette, Rgb};

/// The classic rainbow: red, orange, yellow, green, blue, purple. The same as
/// [`Palette::RAINBOW`].
pub const RAINBOW: Palette<6> = Palette::RAINBOW;

/// Deep red to orange.
pub const RED_ORANGE: Gradient<2> =
    Gradient::new([(0, Rgb::new(255, 0, 0)), (255, Rgb::new(255, 120, 0))]);

/// MATLAB's "jet" color map: dark blue, blue, cyan, yellow, red, dark red.
pub const JET: Gradient<6> = Gradient::new([
    (0, Rgb::new(0, 0, 143)),
    (32, Rgb::new(0, 0, 255)),
    (96, Rgb::new(0, 255, 255)),
    (160, Rgb::new(255, 255, 0)),
    (224, Rgb::new(255, 0, 0)),
    (255, Rgb::new(128, 0, 0)),
]);

/// Flames: black (cold) through dark red, orange and yellow to pale yellow
/// (hottest). Position 0 is black, so cold spots are dark.
pub const FIRE: Gradient<5> = Gradient::new([
    (0, Rgb::new(0, 0, 0)),
    (70, Rgb::new(160, 0, 0)),
    (140, Rgb::new(255, 64, 0)),
    (200, Rgb::new(255, 160, 0)),
    (255, Rgb::new(255, 240, 160)),
]);

/// Black, red, yellow, white: a hot metal glow.
pub const HEAT: Gradient<4> = Gradient::new([
    (0, Rgb::new(0, 0, 0)),
    (85, Rgb::new(255, 0, 0)),
    (170, Rgb::new(255, 255, 0)),
    (255, Rgb::new(255, 255, 255)),
]);

/// Black and dark reds with flashes of orange and white.
pub const LAVA: Palette<8> = Palette::new([
    Rgb::new(0, 0, 0),
    Rgb::new(128, 0, 0),
    Rgb::new(139, 0, 0),
    Rgb::new(255, 0, 0),
    Rgb::new(255, 165, 0),
    Rgb::new(255, 255, 255),
    Rgb::new(255, 165, 0),
    Rgb::new(139, 0, 0),
]);

/// Deep blues, sea greens and aqua.
pub const OCEAN: Palette<8> = Palette::new([
    Rgb::new(25, 25, 112),
    Rgb::new(0, 0, 139),
    Rgb::new(0, 0, 205),
    Rgb::new(46, 139, 87),
    Rgb::new(0, 128, 128),
    Rgb::new(100, 149, 237),
    Rgb::new(127, 255, 212),
    Rgb::new(0, 139, 139),
]);

/// Greens, from dark forest to bright lime.
pub const FOREST: Palette<8> = Palette::new([
    Rgb::new(0, 100, 0),
    Rgb::new(85, 107, 47),
    Rgb::new(0, 128, 0),
    Rgb::new(34, 139, 34),
    Rgb::new(107, 142, 35),
    Rgb::new(50, 205, 50),
    Rgb::new(154, 205, 50),
    Rgb::new(46, 139, 87),
]);

/// A dark blue sky with drifting white and light blue.
pub const CLOUD: Palette<8> = Palette::new([
    Rgb::new(0, 0, 255),
    Rgb::new(0, 0, 139),
    Rgb::new(0, 0, 139),
    Rgb::new(0, 0, 255),
    Rgb::new(135, 206, 235),
    Rgb::new(173, 216, 230),
    Rgb::new(255, 255, 255),
    Rgb::new(135, 206, 235),
]);

/// Purple, pink, red, orange and yellow, with no green: bright and warm.
pub const PARTY: Palette<8> = Palette::new([
    Rgb::new(0x55, 0x00, 0xAB),
    Rgb::new(0xB5, 0x00, 0x4B),
    Rgb::new(0xE8, 0x17, 0x00),
    Rgb::new(0xAB, 0x77, 0x00),
    Rgb::new(0xAB, 0x55, 0x00),
    Rgb::new(0xF2, 0x00, 0x0E),
    Rgb::new(0x8F, 0x00, 0x71),
    Rgb::new(0x2F, 0x00, 0xD0),
]);

/// Purple, crimson, orange and gold.
pub const SUNSET: Gradient<4> = Gradient::new([
    (0, Rgb::new(80, 0, 120)),
    (100, Rgb::new(200, 0, 60)),
    (180, Rgb::new(255, 90, 0)),
    (255, Rgb::new(255, 200, 0)),
]);

/// Navy, bright blue and icy white.
pub const ICE: Gradient<3> = Gradient::new([
    (0, Rgb::new(0, 0, 64)),
    (128, Rgb::new(0, 128, 255)),
    (255, Rgb::new(220, 240, 255)),
]);

/// The "viridis" color map from scientific plotting: dark purple, blue, teal,
/// green, yellow. Its brightness rises evenly from end to end.
pub const VIRIDIS: Gradient<9> = Gradient::new([
    (0, Rgb::new(68, 1, 84)),
    (32, Rgb::new(71, 45, 123)),
    (64, Rgb::new(59, 82, 139)),
    (96, Rgb::new(44, 114, 142)),
    (128, Rgb::new(33, 145, 140)),
    (159, Rgb::new(40, 174, 128)),
    (191, Rgb::new(94, 201, 98)),
    (223, Rgb::new(173, 220, 48)),
    (255, Rgb::new(253, 231, 37)),
]);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorMap;

    #[test]
    fn jet_ends_dark_blue_and_dark_red() {
        assert_eq!(JET.color_at(0), Rgb::new(0, 0, 143));
        assert_eq!(JET.color_at(255), Rgb::new(128, 0, 0));
    }

    #[test]
    fn fire_starts_black() {
        assert_eq!(FIRE.color_at(0), Rgb::BLACK);
    }

    #[test]
    fn wrapping_flags() {
        assert!(ColorMap::wraps(&RAINBOW));
        assert!(ColorMap::wraps(&PARTY));
        assert!(!ColorMap::wraps(&JET));
        assert!(!ColorMap::wraps(&VIRIDIS));
    }

    #[test]
    fn dyn_works() {
        let map: &'static dyn ColorMap = &OCEAN;
        assert_eq!(map.color_at(0), Rgb::new(25, 25, 112));
    }
}
