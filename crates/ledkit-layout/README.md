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

Light the panel with the wiring finder from the matrix example (LED 0 red,
the rest of strip row 0 green), then:

| What you see | Builder call |
|---|---|
| Red in the top-left | nothing (the default) |
| Red in another corner | `.starting_at(Corner::TopRight)`, `BottomLeft` or `BottomRight` |
| Green runs down a column | `.columns_first()` |
| The next line starts under the end of the last | `.serpentine()` |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
