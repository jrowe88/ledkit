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

Call `set_messages` again whenever the news changes: the message showing
finishes first, then the new list starts.
