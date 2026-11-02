# ledkit-text: design

*Date: 2026-11-02. Status: for the owner's review. Worked out with the owner over
2026-11-01/02, with the [text preview page](https://jrowe88.github.io/ledkit/text-preview/)
as the shared reference for how things should look.*

## Intent

Let a hobbyist put words and numbers on an LED matrix with a few lines of code, and have them
look good: from scrolling text on an 8x8 NeoPixel panel up to ~40-pixel-tall digits on a
strip of three 64x64 HUB75 panels (192x64).

The owner's uses, in their words: something on the desk "showing something simple, like a
clock"; "a marquee scrolling by with some ticker text"; "periodically updated with calendar
info". An 8x8 panel is "passable, but likely the very lower limit".

`ledkit-text` provides the **building blocks** for those: fonts, a text area, and a ticker.
Putting a whole screen together (a clock with weather, icons, several areas) is the user's
code, not this crate. With the building blocks, a desk clock is about ten lines.

## Goals

- Fonts that span 8x8 to 192x64, all usable with no setup.
- A **text area**: text drawn in a box on the panel, aligned inside it.
- A **ticker**: messages scrolling through a box, one after another.
- Colour that is as flexible as the rest of ledkit: letters, outline and background can each
  be a plain colour or any `ledkit-anim` animation.
- Looks match the preview page.

## Non-goals

- Laying out whole screens, icons or pictures (a weather sun is the user's own drawing).
- Clock or calendar logic: the user's code formats the time and date and hands over text.
- Text that changes while it scrolls (a ticking clock is a text area, not a ticker message).
- Characters beyond printable ASCII (space to `~`), kerning, proportional spacing,
  right-to-left text, word wrap.
- Per-message colours in the ticker (could come later).
- Shrinking a font below its drawn size; integer scaling past 2x (the owner rejected both
  after seeing them: shrinking blurs, 3x+ looks blocky and doesn't fit).

## Anti-goals (would make it a failure even if it works)

- Fuzzy text. Anti-aliasing must look crisp at the default setting, as in the preview.
- Needing a heap, an allocator, or a font file at run time.
- Setup that needs research: picking a font and a size should be one line each.
- Text that drifts from the preview's look without the preview being updated to match.

## Constraints (from the project)

- New crate `crates/ledkit-text`, in the root workspace. **Decided by the owner.**
- Depends on `ledkit-color`, `ledkit-anim` (for `Animation`, `Surface`, `Position`,
  `Frame`, `Time`). `ledkit-anim` currently lives on branch `m7-anim`, which is not merged
  into `main` and has no PR yet; this work builds on it (base the branch on `m7-anim`, or on
  `main` once it is merged).
- `no_std`, no `alloc`, no `unsafe`, MSRV 1.82, `clippy::pedantic` clean (workspace lints),
  host-tested, the CI no_std builds (`thumbv6m-none-eabi`, `riscv32imc-unknown-none-elf`).
- Integer maths per pixel. Floating point only for things computed once (tables, generators).
- Every public item documented with an example; readable names for beginners.
- Fonts only from public-domain/MIT sources or original work, with a `NOTICE` line.
- An example program outside the workspace with its own `Cargo.lock` and CI job, like the
  existing examples. The owner checks hardware by eye (ESP32-C6, Adafruit NeoPixel 8x8).
- README, crate map in `AGENTS.md`, and roadmap updated.

## Decided with the owner

### Fonts

The faces from the preview, as they are now ("a fine starting place"):

| Face | Source | Characters | Sizes |
|---|---|---|---|
| X11 misc-fixed 4x6, 5x7, 5x8, 6x10, 6x13 bold, 7x13 bold, 7x14 bold, 8x13 bold, 9x15 bold, 9x18 bold, 10x20 | `assets/fonts/x11-misc-fixed/*.bdf` (public domain, already in the repo and `NOTICE`) | printable ASCII | 1x, 1.25x, 1.5x, 2x |
| Tiny 3x5 | original (the preview's `TINY` table) | capitals, digits, some punctuation; lower case shows as capitals | 1x, 1.25x, 1.5x, 2x |
| 7-segment, upright or slanted | drawn, original | digits, `:` `.` `-` and the standard 7-segment letters; for numbers and clock labels only (K M V W X have no form) | any height from 5 rows |
| 14-segment | drawn, original | every capital, digits, basic punctuation | any height from 7 rows |

- **1x** is the font's own pixels, exactly.
- **1.25x / 1.5x / 2x** are anti-aliased: enlarge with Scale2x (EPX) three times (8x), then
  average down to the target height (4x4 samples per LED), then apply the edge sharpness
  curve `clamp((v - 0.5) * k + 0.5, 0, 1)`.
- **Anti-aliasing is done per letter, not per string** *(my call)*. Each glyph cell is
  enlarged on its own and advances by a whole number of LEDs (`round(cell width x size)`,
  plus the spacing). The preview currently anti-aliases the whole string at once, so letters
  land on fractional columns and the same letter can look different in different places.
  Per letter, every letter looks the same wherever it is, and the tables can be baked. The
  preview is changed to match **first**, in the same work, so the owner can check the look
  by eye before the crate copies it.
- **Edge sharpness** settings: soft (k = 1), medium (k = 2, the default), crisp (k = 3.5),
  very crisp (k = 6).
- The segment faces are drawn at whatever height is asked for; they can be anti-aliased
  (drawn at 8x and averaged down, same sharpness curve) or hard-edged (cut at 0.5). The
  preview draws them in floating point; the crate draws them in integer maths, so tests
  compare segment coverage with the preview **within a small tolerance** (one sharpness step
  of 1/16 coverage) rather than exactly *(my call)*. Bitmap faces at 1x, and baked
  anti-aliased bitmap faces, must match the (updated) preview exactly.
- Letter spacing: +0 to +3 extra columns between letters.

### Colour (the "stencil")

Each text area or ticker has three colour sources, each either a plain `Rgb` or any
`Animation`:

- **letters** (the fill), default white;
- **outline**: optional, one LED wide around the letters (an LED's outline amount is the
  most-lit neighbour minus how lit it is itself). Outlined letters usually need +2 spacing;
- **behind**: optional; off means unlit (black).

**Reverse** swaps which LEDs are "letter": the background is lit with the letter colour and
the letters are black. ("By reverse, I meant coloured background and black text.")

Compositing per LED, as in the preview: start from *behind*, mix in *outline* by the outline
amount, then mix in *letters* by the letter coverage (anti-aliased edges give partial
coverage).

### Text area

- A box on the panel: left, top, width, height. Drawing touches only LEDs inside the box.
- Alignment both ways: across (left, centre, right) and up and down (top, middle, bottom).
  ("Alignment might be good -- top, center, bottom.")
- Text that doesn't fit is clipped at the box edge.
- The user's code changes its text whenever it likes (a clock every second or minute).
- Two lines are two text areas stacked.

### Ticker

- A box, like a text area, whose text moves.
- The user's code gives it a short list of messages. It shows them in order and loops.
- Each message enters from the right edge, scrolls left at a steady speed, and leaves fully
  off the left edge; then a short blank gap; then the next message enters.
- A new list from the user's code takes effect at the next gap, never mid-message.
- A message that fits in the box doesn't scroll: it holds still for a few seconds, then the
  ticker moves on. (The owner hadn't thought about this; "that is fine".)
- Font, size and colours are set for the whole ticker. Up-and-down alignment applies;
  across alignment is the scroll.

## Calls I made (the owner hasn't weighed in; flagged for review)

- **Defaults:** text area centred both ways. Ticker gap = one box width of blank; hold for a
  short message = 3 seconds, centred; a new list starts at its first message.
- **Panel coordinates for colour sources.** Letters, outline and behind are asked for the
  colour at the LED's position on the *panel*, not inside the box. So a text area whose
  *behind* is the same Plasma that fills the rest of the panel blends in seamlessly (ledkit
  animations are pure functions of position and time), and a Rainbow across two stacked
  areas lines up. This is how "text over another pattern" works without the panel having to
  be read back.
- **Ticker state.** Scrolling within a message is a pure function of time, like every ledkit
  effect. Which message is showing, and when it started, is the only state; it lives in the
  ticker, which the user's loop advances by calling it with the current time.
- **Text storage.** Areas and tickers keep their own fixed-size copy of the text (no heap),
  with capacity set by a const generic and a sensible default. Text past the capacity is cut
  off. Characters a face lacks are drawn as that face's `?`, as the preview does.
- **The preview is the reference.** Where this document and the preview page disagree on how
  something looks, the preview wins; change both together. The only planned change to the
  preview is the per-letter anti-aliasing above (edit `template.html`, regenerate with
  `build.py`).

## Illustrative API (shape, not final names)

```rust
use ledkit_text::prelude::*;

// A desk clock on a 64x64 panel: small date on top, big time below.
let mut date = TextArea::new(Font::X11_6X10, Size::X1)
    .at(0, 2, 64, 12)
    .align(Across::Centre, UpDown::Middle)
    .letters(Rgb::new(120, 120, 255));
let mut time = TextArea::new(Font::seven_segment(30), Size::X1)
    .at(0, 20, 64, 40)
    .letters(Plasma::default())
    .outline(Rgb::BLACK);

loop {
    date.set_text("Mon 2 Nov");
    time.set_text("12:34");
    date.render(&mut canvas, now);
    time.render(&mut canvas, now);
    strip.show()?;
}

// A ticker on an 8x8.
let mut ticker: Ticker = Ticker::new(Font::X11_5X7, Size::X1).at(0, 0, 8, 8).speed(12);
ticker.set_messages(&["12:34", "Rain at 4pm", "Standup 9:30"]);
loop {
    ticker.render(&mut canvas, now); // advances to the next message when one finishes
    strip.show()?;
}
```

## Left to the builder

- **Baked or computed anti-aliasing for the bitmap faces.** My suggestion: bake the coverage
  tables for each face and size with a checked-in generator (with a `--check` mode in CI,
  like `site/text-preview/build.py`), one `static` per face and size, so the linker drops
  the ones a program doesn't use. Computing at run time is fine too if it's fast enough on
  an ESP32-C6 at 192x64 and 30 fps. Either way, the output must match the preview.
- Coverage precision (for example 4 or 8 bits per LED).
- How the segment faces are drawn in integer maths at run time, including anti-aliasing.
- Exact names, module layout, the const-generic capacities and their defaults, whether
  `TextArea` also implements `Animation` (for use with `Layer`, `Playlist` and so on).
- Speed units for the ticker (LEDs per second is what the preview uses).
- How the preview page and the crate share the Tiny 3x5 table and segment shapes until the
  WebAssembly preview replaces the JavaScript (a later step on the roadmap).
- Tests: at least exact pixel snapshots for a few strings per face at 1x, anti-aliased
  coverage checks against values produced by the preview, alignment and clipping, the
  ticker's timeline (enter, scroll, gap, next, hold for short messages, list swap at a gap).

## Later (not this work)

- The preview runs `ledkit-text` and `ledkit-anim` compiled to WebAssembly, replacing its
  JavaScript copies.
- Per-message ticker colours, vertical scrolling, more faces (a heavy face for big digits).
