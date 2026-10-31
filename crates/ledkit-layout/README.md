# ledkit-layout

Layouts for ledkit: say where your LEDs are, then draw by `(x, y)`.

An LED matrix is one long chain of LEDs folded into a grid. A `Layout` knows how
the chain is folded, so you can ask "which LED is at column 3, row 2?" instead of
counting. `MatrixLayout` is the one to start with:

```rust
use ledkit_layout::{Layout, MatrixLayout};

const LAYOUT: MatrixLayout = MatrixLayout::new(16, 16).serpentine();

assert_eq!(LAYOUT.index(15, 1), Some(16));
```

`(0, 0)` is the top-left of the picture, `x` grows right and `y` grows down.

## Segments

To use part of a strip as a strip of its own, use a `Segment`: a start, a
length, and optionally `.reversed()` to count from the far end. This suits a
light bar with one side wired back-to-back with the other.

```
use ledkit_color::Rgb;
use ledkit_layout::Segment;

let mut pixels = [Rgb::BLACK; 10];
let mut segment = Segment::new(2, 5).reversed().of(&mut pixels).unwrap();
segment.set(0, Rgb::RED); // the segment's first LED is its far end
assert_eq!(pixels[6], Rgb::RED);
```

## Rotation and tiling

Mounted a panel sideways or upside down? Turn the layout, not the wiring.
`.rotated(Rotation::Quarter)` turns it clockwise as you look at the panel, and
`(0, 0)` stays at the top-left of what you see. A quarter or three-quarter turn
swaps the width and height.

```
use ledkit_layout::{Layout, MatrixLayout, Rotation};

// A tall 8x32 panel lying on its side is a wide 32x8 picture.
let sideways = MatrixLayout::new(8, 32).rotated(Rotation::Quarter);
assert_eq!((sideways.width(), sideways.height()), (32, 8));
```

To join several panels into one big picture, use `Tiled`. It takes the
panel, then how many panels go across and down. Chain `.serpentine()`,
`.columns_first()` or `.starting_at(...)` to say how the *panels* are chained;
each panel keeps its own wiring.

```
use ledkit_layout::{Layout, MatrixLayout, Tiled};

// Four 8x8 panels across and two down make a 32x16 picture.
let wall = Tiled::new(MatrixLayout::new(8, 8), 4, 2);
assert_eq!((wall.width(), wall.height()), (32, 16));
assert_eq!(wall.len(), 512);
```

## Text and shapes

With the `embedded-graphics` feature, a `Canvas` is an
[`embedded-graphics`](https://docs.rs/embedded-graphics) draw target, so you can
draw text, lines, shapes and images. Anything that falls off the canvas is
clipped. `embedded-graphics` colors are `Rgb888`; `rgb_from_rgb888` and
`rgb888_from_rgb` convert to and from ledkit's `Rgb`. The matrix example
(`examples/esp32c6-matrix`) draws shapes and scrolling text this way.

## Finding your wiring

Light the panel with the wiring finder from the matrix example. It lights
LED 0 red, LEDs 1 to `WIDTH - 1` green, and the first 3 LEDs of the next line
(LEDs `WIDTH` to `WIDTH + 2`) blue. Red plus green together cover `WIDTH`
LEDs; the green alone is `WIDTH - 1`. Look at the panel:

- Red to the end of the green is where the first line runs.
- Where the blue starts is where the second line starts. Blue next to the end
  of the green means serpentine; blue back at the red's side means
  progressive.

The finder assumes the panel is wired rows first. If the green runs down a
column instead, or wraps onto a second line, the panel is wired columns first
(or `WIDTH` is wrong). Settle rows versus columns before you judge serpentine
from the blue LEDs.

Then use the table:

| What you see | Builder call |
|---|---|
| Red is in the top-left corner | nothing (the default) |
| Red is in the top-right corner | `.starting_at(Corner::TopRight)` |
| Red is in the bottom-left corner | `.starting_at(Corner::BottomLeft)` |
| Red is in the bottom-right corner | `.starting_at(Corner::BottomRight)` |
| Red to the end of the green runs along a row | nothing (rows first) |
| Red to the end of the green runs down a column | `.columns_first()` |
| Blue starts next to the end of the green | `.serpentine()` |
| Blue starts back at the red's side | nothing (progressive) |

## Power on big matrices

A matrix draws a lot of current. Each LED can pull about 60 mA at full white,
so a 16x16 (256 LEDs) at full white can draw over 10 A, far more than a USB
port can give. The matrix example starts with a 500 mA power limit and 10%
brightness, which is safe on USB. Use `set_power_limit` and ledkit dims the
whole picture just enough to stay under it, so a dim matrix is the limit
working:

```
use ledkit_core::prelude::*;
use ledkit_core::testing::MemoryOutput;

let mut strip = Strip::<Ws2812b, 64, _>::new(MemoryOutput::<Rgb, 64>::new());
strip.set_power_limit(Milliamps(500)); // the example's default
```

To raise it, use a proper 5 V supply rated for the current you set, with
thick enough wires, and raise the limit and the brightness together. Don't
power a big matrix from the board's USB port.

## Tested on hardware

**Status: pending hardware test.** The `esp32c6-matrix` example builds in CI but has not been run on a matrix yet.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
