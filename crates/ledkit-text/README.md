# ledkit-text

Text for LED matrices: pick a font, a size and a paint (one color, or any
effect), and ask each LED what it should show. `no_std`, no allocator.

Part of [ledkit](https://github.com/jrowe88/ledkit).

## A first look

A font knows how big its letters are:

```rust
use ledkit_text::prelude::*;

let font = Font::X11_5X8;
assert_eq!(font.height(), 8);
assert_eq!(font.text_width(b"Hi"), 10);
```
