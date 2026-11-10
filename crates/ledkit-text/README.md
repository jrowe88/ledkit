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
assert_eq!(font.text_width("Hi"), 10);
```

## The fonts

| Font | What it is |
|---|---|
| `Font::X11_4X6`, `X11_5X7`, `X11_5X8`, `X11_6X10`, `X11_6X13`, `X11_6X13_BOLD`, `X11_7X13_BOLD`, `X11_7X14_BOLD`, `X11_8X13_BOLD`, `X11_9X15_BOLD`, `X11_9X18_BOLD`, `X11_10X20` | The classic X11 "misc-fixed" bitmap fonts: the name is the letter width by height in LEDs. From the 8x8 up to big panels. |
| `Font::TINY_3X5` | A 3 by 5 font, capitals and digits only (lower case shows as capitals). Use `.spacing(1)` for a gap between letters. |
| `Font::seven_segment(height)` | Like a digital clock: digits, `:` and the letters a seven-segment display can make. Any height from 5 up. |
| `Font::seven_segment_slanted(height)` | The same, leaning to the right. |
| `Font::fourteen_segment(height)` | A "starburst" display: every capital letter, digits and basic punctuation. Any height from 7 up. |

Hold a font in a `const` and adjust it; every method gives back a new `Font`:

```rust
use ledkit_text::prelude::*;

const BIG: Font = Font::X11_5X8.size(Size::X2).edges(Edges::Crisp).spacing(1);
assert_eq!(BIG.height(), 16);
```

- **Size:** `Size::X1` (the default, letters exactly as drawn), `X1_25`, `X1_5` and
  `X2`. The bigger sizes are smoothed (anti-aliased) so curves look right instead
  of blocky. The segment fonts ignore the size: you give them a height.
- **Edges:** how sharp that smoothing is: `Edges::Soft`, `Medium` (the default),
  `Crisp` and `VeryCrisp`. On a segment font, `.hard_edges()` makes each LED
  fully on or off, like a real segment display.
- **Spacing:** `.spacing(n)` adds `n` LEDs between letters.

## Letters, outline and what's behind

Text is a stencil with three paints: the **letters**, an optional one-LED
**outline** around them, and what's **behind** (the rest of the box). Each is a plain
colour or any effect, set with `.letters(...)`, `.outline(...)` and `.behind(...)`.
`.reverse(true)` swaps the letters and what's behind, like a highlighted label.

```rust
use ledkit_layout::{Canvas, MatrixLayout};
use ledkit_text::prelude::*;

let mut pixels = [Rgb::BLACK; 32 * 8];
let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(32, 8)).unwrap();

let mut area = TextArea::new(Font::X11_5X7)
    .at(0, 0, 32, 8)
    .align(Across::Centre, UpDown::Middle)
    .letters(Rgb::WHITE)
    .outline(Rgb::new(0, 0, 90))
    .behind(Rgb::new(20, 0, 0));
area.set_text("SALE");
area.render(&mut panel, Time::from_millis(0));
```

## Text areas and alignment

`.at(left, top, width, height)` puts the box on your panel, and
`.align(Across::Left | Centre | Right, UpDown::Top | Middle | Bottom)` lines the text up
inside it (centred by default). Text too wide for the box is clipped; use a
`Ticker` for text that should scroll.

## A desk clock

A `TextArea` draws text in a box on your panel, lined up how you like. Two
areas stacked make two lines: here a small date on top and a big clock below,
on a 32x24 panel.

```rust
use ledkit_anim::Time;
use ledkit_layout::{Canvas, MatrixLayout};
use ledkit_text::prelude::*;

let mut pixels = [Rgb::BLACK; 32 * 24];
let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(32, 24)).unwrap();

let mut date = TextArea::new(Font::TINY_3X5.spacing(1))
    .at(0, 0, 32, 7)
    .letters(Rgb::new(120, 120, 255));
let mut time = TextArea::new(Font::seven_segment(14))
    .at(0, 8, 32, 16)
    .letters(Plasma::new());

// In your loop: change the text when it changes, then draw both.
date.set_text("MON 2");
time.set_text("12:34");
let now = Time::from_millis(0);
date.render(&mut panel, now);
time.render(&mut panel, now);
```

To blink the colon, swap it for a space every other second, but give the area
a fixed left edge (`Across::Left`, with the box starting where the wider of
`"12:34"` and `"12 34"` would be centred), since the space is a different width
from the colon and centred text would jiggle.

Letters, the outline and what's behind can each be a plain colour or any
effect. Effects are coloured by where each LED is on the whole panel, so a
rainbow across two areas lines up.

## A ticker

A `Ticker` scrolls a list of messages through a box, one after another, and
starts again from the first. Each one comes in from the right and leaves off
the left; one short enough to fit stays still in the middle for 3 seconds.
Give it the time each time round your loop and it does the rest.

```rust
use ledkit_layout::{Canvas, MatrixLayout};
use ledkit_text::prelude::*;

let mut pixels = [Rgb::BLACK; 8 * 8];
let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(8, 8)).unwrap();

let mut ticker = Ticker::new(Font::X11_5X7).letters(Rainbow::new()).speed(12);
ticker.set_messages(&["12:34", "Rain at 4pm", "Standup 9:30"]);

// In your loop, with the milliseconds since your program started:
for millis in (0..5000).step_by(33) {
    ticker.render(&mut panel, Time::from_millis(millis));
    // strip.show() here.
}
```

Call `set_messages` again whenever the news changes, even once a second with
the whole list built again: the message showing finishes first, then the new
list carries on with the next message in order. The same list again changes
nothing.

## Try it

- See every font, size and edge setting in the
  [text preview](https://jrowe88.github.io/ledkit/text-preview/).
- On your computer, in the terminal: in `examples/anim-preview`, run
  `cargo run -- text "Hello" --font 7seg` for a ticker, or `cargo run -- clock` for a
  date and a 7-segment time. `cargo run -- list` shows the font names.
- On a board: `examples/esp32s3-text` (tested on hardware) and `examples/rp2350-text` tour
  every font that fits an 8x8 DotStar panel, with outlines, backgrounds and effects.
- `examples/esp32c6-text` scrolls a ticker on an 8x8 matrix (not yet tested
  on hardware).
