# ledkit-layout

Layouts for ledkit: say where your LEDs are, then draw by `(x, y)`.

This crate is in progress. So far it has the `Layout` trait and
`MatrixLayout`, which maps a grid position to a position on the strip:

```rust
use ledkit_layout::{Layout, MatrixLayout};

const LAYOUT: MatrixLayout = MatrixLayout::new(16, 16).serpentine();

assert_eq!(LAYOUT.index(15, 1), Some(16));
```

`(0, 0)` is the top-left of the picture, `x` grows right and `y` grows down.

## Finding your wiring

Light the panel with the wiring finder from the matrix example. It lights
LED 0 red, LEDs 1 to `WIDTH - 1` green, and the first 3 LEDs of the next line
(LEDs `WIDTH` to `WIDTH + 2`) blue. Look at the panel:

- Red to the end of the green is where the first line runs.
- Where the blue starts is where the second line starts. Blue next to the end
  of the green means serpentine; blue back at the red's side means
  progressive.

The green run is as long as the `WIDTH` you set. If the green wraps around
onto a second line, the panel is wired columns first, or the width is wrong.

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

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
