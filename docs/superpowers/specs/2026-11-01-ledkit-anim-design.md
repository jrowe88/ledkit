# ledkit-anim design (milestone 7)

Date: 2026-11-01. Status: draft for the owner's review.

## Intent

Make strips and matrices easy to animate. A hobbyist picks an effect from a library by
the name they already know (rainbow, marquee, comet, sparkle, flames, swirl, concentric
rings, rotating arms ...), changes a few plainly named settings, and runs it. Effects can
be combined without writing new ones: one after another with transitions, stacked in
layers, or different effects on different parts of a strip.

It should look right on the owner's Adafruit NeoPixel 8x8 and DotStar 8x8 and on the
strips (100 WS2812, 144 SK9822, 300 APA102), and carry over unchanged to HUB75 panels in
phase 2.

## Goals

- An `Animation` trait, time-based: "what do the LEDs look like at time *t*" (the overall
  design already decided this; speed stays right at any frame rate).
- The phase 1 effects from the overall design, plus the matrix effects the owner named.
- Combinators: sequence/playlist with transitions, crossfade, layer with blend modes,
  per-segment.
- Named palettes, including full rainbow, red-orange and MATLAB's jet (the owner's
  examples), usable by every effect that takes colors.
- Every effect works with no settings (`Swirl::new()`), and each setting is a plain name
  in real units (`.tightness(..)`, `.tail_length(..)`, `.cycle_time(..)`).
- `no_std`, no heap, no `unsafe`, MSRV 1.82, clippy pedantic clean, fully host-tested.

## Non-goals

- Text and fonts (the owner raised them: a couple of fonts and digit sets in several
  sizes). Not in this milestone; a follow-up. `ledkit-layout` already draws text with
  `embedded-graphics` fonts in the meantime.
- Effects that need memory of past frames (Game of Life, flocking, particle physics,
  FastLED's Fire2012 heat buffer). See "One pure function" below for why.
- Audio-reactive effects, network control, a UI. (Overall design non-goals.)
- The desktop simulator (`ledkit-sim`): a later milestone.

## Anti-goals

- Settings as anonymous 0–255 "speed / intensity" sliders (the WLED model). Readable
  in a UI, unreadable in code; ledkit is for beginners reading code.
- An effect that only works on one wiring or one shape. Nothing may assume a one-wire
  strip.
- Floating point per pixel (repository rule: integer math per pixel).

## Constraints

From the workspace: `no_std`, no `alloc`, `#![forbid(unsafe_code)]`, MSRV 1.82, stable
Rust, clippy pedantic, docs plus an example on every public item, tests in the module
they test, MIT OR Apache-2.0. Ideas from WLED, blinksy (EUPL), NeoPixelBus (LGPL),
FastLED_NeoMatrix (GPL) and Pixelblaze patterns may be reimplemented but not copied.
Code may be ported, with a line in `NOTICE`, from FastLED (MIT: sine/noise helpers,
palettes, TwinkleFox), Adafruit CircuitPython LED_Animation (MIT), WS2812FX (MIT) and
SmartMatrix Aurora (MIT, check each file header). The survey with sources is in the
session notes (`ledkit-anim-survey.md`), not in the repo.

## The design

### One pure function per effect (decided by Claude; the core of the design)

An effect answers one question: *what color is the pixel at this position at this
time?* It keeps no frame buffer and no memory between frames.

```rust
pub trait Animation {
    /// The color of one pixel at one moment.
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb;
}
```

- `Frame` is computed once per frame: the elapsed time in milliseconds (`u32`, wraps
  after 49 days) and the surface size (`width`, `height`, `len`).
- `Position` is the pixel's `x`, `y` and its position along the chain (`index`). (Not
  called `Pixel`: `ledkit_core::Pixel` already exists and both will meet in the `ledkit`
  prelude.)
- A strip is a surface one pixel tall: `x == index`, `y == 0`. So every effect runs on
  both strips and matrices, and HUB75 later is just another surface.

Why: combinators become trivial and need no scratch buffers (a crossfade computes both
colors for the pixel and blends them; a layer blends two colors; a playlist picks which
child to ask), so there is no heap and no per-animation buffer sized by `N`. The trait
is object safe, so `&dyn Animation` works for lists. Everything is deterministic, so
tests can check any pixel at any time.

The cost: effects that need memory are out (see non-goals). "Random" effects (sparkle,
twinkle, rain, flicker) use a small integer hash of (pixel, time slot, seed) instead of a
running random generator; this is how FastLED's TwinkleFox works, and it looks the same.
Flames use moving integer noise instead of a heat buffer. Effects re-evaluate every pixel
every frame: about 64 pixels for an 8x8 and 300 for the longest strip, a few thousand for
a HUB75 panel, all integer math, which is fine on the target chips.

*Left to the builder:* exact field names of `Frame` and `Position`; whether `Frame` also
carries helpers such as the center point or precomputed scale factors.

### Drawing onto LEDs

```rust
let mut rainbow = Rainbow::new().cycle_time(Duration::from_secs(5));
loop {
    rainbow.render(&mut strip, clock.elapsed());          // a whole strip
    strip.show()?;
}
// a matrix: a Canvas from ledkit-layout
swirl.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT)?, elapsed);
// part of a strip: a SegmentMut from ledkit-layout
comet.render(&mut Segment::new(0, 50).of(strip.pixels_mut())?, elapsed);
```

- `render` is a provided method on `Animation` with `where Self: Sized`, taking
  `&mut impl Surface` and the elapsed time. The trait stays object safe, and a blanket
  `impl Animation for &A` (`A: ?Sized`) lets a `&dyn Animation` render too.
- The elapsed time is a `Time` (milliseconds in a `u32`), made with
  `Time::from_millis(..)` or from a `core::time::Duration`. Board clocks have their own
  duration types (esp-hal's `Instant::elapsed` returns `esp_hal::time::Duration`), so
  examples convert with `Time::from_millis(now.as_millis() as u32)`; one line, shown in
  the docs. Time wraps after about 49 days; effects whose period does not divide 2^32 ms
  jump once at the wrap. Documented, accepted. Speed scaling uses `u64` intermediates.
- `Surface` is a small trait (`width`, `height`, `index(x, y)` for the chain position,
  `set(x, y, Rgb)`) implemented for `Strip`, `Canvas` (index from its layout),
  `SegmentMut` and plain slices of pixels. Colors go in through the pixel type's
  `From<Rgb>`, so RGBW strips work.
- Settings that are times take `core::time::Duration`.
- `render` only sets pixels; brightness, gamma, dithering and the power limit stay in
  `Strip::show`.

### Effects in this milestone

Every effect is a struct with `new()` (good defaults) and builder setters. Names and
settings as below; the setting names are the plan, the exact defaults are the builder's
call (tuned to look good on the 8x8 and a 100-LED strip).

Along the strip (on a matrix these run along `x`, unless noted):

| Effect | Also known as | Settings |
|---|---|---|
| `Solid` | static | color |
| `Blink` | strobe, flash | on color, off color, period, duty |
| `Breathe` | pulse | color(s), period, min/max level (raised cosine, as in pulse) |
| `Rainbow` | color loop | cycle time, width (how many rainbows across), saturation |
| `ColorCycle` | fade | colors/palette, time per color, hard or smooth |
| `GradientScroll` | palette scroll | palette, cycle time, width |
| `ColorWipe` | wipe, fill, loading | color, background, time, direction, then hold/clear |
| `Chase` | theater chase, marquee, running lights | colors, size, spacing, speed, direction |
| `Comet` | meteor, scanner, Larson, Cylon | color, tail length, speed, bounce |
| `Sparkle` | glitter, twinkle | color/palette, density, twinkle time, background |
| `Flames` | fire, candle | colors (start/end) or palette, size (flame height), speed |

On a matrix, `Flames` rises from the bottom row with `size` as the flame height; on a
strip it burns from index 0. `Chase` on a matrix runs along the chain order (the owner's
"marquee"); a border-marquee is left for later.

Matrix effects (also run on a strip, as a 1-pixel slice through the middle):

| Effect | Also known as | Settings |
|---|---|---|
| `Swirl` | spiral | tightness, arms, speed, palette, direction |
| `Rings` | concentric | ring width, speed (out or in), palette, center |
| `Pinwheel` | rotating arms / rotating X | arms, speed, palette, direction, center |
| `Plasma` | | scale, speed, palette |
| `Rain` | matrix rain, digital rain | color, density, speed, trail length |

Positions in the 2D effects are fixed point (the center of an 8x8 is between pixels);
angle and distance use an integer `atan2` and square root; waves use a sine table. These
helpers live in a `math` module of `ledkit-anim`. *Builder's call:* whether that module
is public.

A pick-by-name layer (decided by Claude): an `Effect` enum with one variant per built-in
effect, `Effect::name()`, `Effect::from_name("swirl")` (accepting the aliases above) and
`Effect::ALL`. It implements `Animation`. Its use: a playlist of `[Effect; N]` needs no
`dyn`, and a button or serial command can switch effects by name.

### Colors and palettes (goes in `ledkit-color`)

- A `ColorMap` trait in `ledkit-color` (`color_at(position: u8) -> Rgb`, deliberately
  the same name as the existing inherent `const fn`s, plus `wraps() -> bool`),
  implemented by the existing `Gradient<N>` (does not wrap) and `Palette<N>` (wraps).
- Scrolling effects (`GradientScroll`, `Swirl`, `Rings`, `Pinwheel`, `Plasma`) run a
  wrapping map round and round, and a non-wrapping one (jet, viridis) back and forth, so
  there is never a hard seam. (Decided by Claude.)
- A `palettes` module of named constants: `RAINBOW` (a new constant pointing at the
  existing `Palette::RAINBOW`), `RED_ORANGE`,
  `JET` (MATLAB's, built from its published piecewise-linear definition), `FIRE`, `HEAT`,
  `LAVA`, `OCEAN`, `FOREST`, `CLOUD`, `PARTY`, `SUNSET`, `ICE`, `VIRIDIS`. FastLED's
  palettes are MIT and may be ported with attribution; viridis is CC0. *Builder's call:*
  the final list (at least the owner's three plus fire, ocean and party).
- Effects take colors through one small type, `Colors` (decided by Claude): a single
  color, two colors to blend between (the owner's flame "start and end colors"), or a
  `&'static dyn ColorMap`. So `.colors(Rgb::RED, Rgb::YELLOW)` and
  `.palette(&palettes::FIRE)` both work and every effect has the same color vocabulary.
  The limit: a palette built at run time must be stored in a `static`. Acceptable for
  phase 1; a generic version can come later without breaking the setters.

### Combinators

All are pure (no state), so they are functions of time like the effects. Each is a struct,
with short builder methods on an extension trait so they read left to right.

- **Sequence / playlist.** `Playlist::new(&entries)` over a slice of
  `(animation, how long, transition)` where the animation is `&dyn Animation` or an
  `Effect`; loops by default, or plays once and holds the last frame. Each entry gets its
  own clock starting at 0 when it begins (including the incoming side of a transition),
  so a wipe or a bouncing comet always starts from the beginning. (Decided by Claude.) Two-step shortcut:
  `a.for_time(d).then(b)`.
- **Transitions** (used by playlists and by `Crossfade`): a `Transition` enum with
  `Cut`, `Fade` (crossfade), `FadeThroughBlack`, `Wipe(Direction)`, `Dissolve` (pixels
  switch in a random order), `Iris` (a growing circle from the center). Each takes a duration and an easing curve
  (`Linear`, `EaseInOut` and a few more; *builder's call* which).
- **Crossfade.** `a.crossfade_to(b, at, over)`: from `a` to `b` starting at a time; `b`
  gets its own clock from `at`, as in a playlist.
- **Layer with blend modes.** `base.layer(top, Blend::Add)`. Modes: `Over` (the top's brightness,
  its largest channel, is its opacity: black is see-through and a comet's dim tail fades
  into what is under it; decided by Claude), `Add`, `Multiply` (use a pattern as a mask),
  `Screen`, `Lighten`, `Darken`, `Average`. Also `.dimmed(level)` on any animation.
- **Per-segment.** `Segments::new(&[(Segment, &dyn Animation)])`: each child sees its own
  segment as a whole strip (`index` and `x` count from 0, `len` is the segment length),
  pixels outside every segment stay black. Segments are spans of the chain order, on a
  matrix too (same as `ledkit-layout`'s `Segment`); rectangular regions are later. Rendering separately into each `SegmentMut`
  also works; the combinator exists so a split-strip show can go into a playlist.
- **Small modifiers:** `.reversed()`, `.mirrored()`, `.speed(percent)`,
  `.starting_at(t)` (shift time, for restarting an effect mid-run).

### Crate layout

- New `crates/ledkit-anim` (workspace member), depending on `ledkit-color`,
  `ledkit-layout` and `ledkit-core` (for the `Strip` surface; the `AGENTS.md` crate map
  gets the extra dependency). `ColorMap` and
  `palettes` go in `ledkit-color`; `Colors` goes in `ledkit-anim`.
- Modules: `animation` (trait, `Frame`, `Position`, `Surface`), `effects/` (one file per
  effect), `combine/` (playlist, transitions, crossfade, layer, segments, modifiers),
  `effect` (the by-name enum), `math`. A `prelude`.
- Tests in each module: spot pixels at chosen times, periodicity, bounds (no panics on
  1-pixel, 1x1 and empty surfaces), combinator algebra (crossfade at 0 = A, at end = B;
  `Over` with black top = base; segments leave the gaps black).
- `no_std` builds for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf` like the
  other crates (both have no hardware float, a good check on the integer-only rule).

### Examples (outside the workspace, own `Cargo.lock`, own CI job)

- `examples/esp32c6-anim`: ESP32-C6 DevKit, data on GPIO4 through the 74AHCT125, RMT
  output. A `const` picks strip or 8x8 matrix (NeoPixel 8x8: `columns_first`,
  `BottomLeft`). On the matrix: a playlist of swirl, rings, pinwheel, plasma, flames,
  rain with crossfades. On the strip: rainbow, comet over a slow gradient (layer),
  chase, sparkle, flames, then a split-strip segment show. Hardware check by the owner
  by eye.
  Time comes from esp-hal's `Instant`, converted as above.
- `examples/anim-preview` (decided by Claude, cheap): a desktop program that prints the
  8x8 or a strip as colored blocks in the terminal, so effects can be seen without
  flashing a board. Not a simulator; just enough to look.

### Docs

Crate README and crate docs in the usual plain style (prelude first, a first-program
example, the effect table with aliases, "write your own effect" in a dozen lines),
`CONTRIBUTING.md` "add an effect" guide, `AGENTS.md` crate map row moved to "exists",
roadmap row 7 marked done with notes, `NOTICE` lines for anything ported.

## Left to the builder

Exact defaults; field names of `Frame`/`Position`; easing set; final palette list beyond the
required ones; noise and hash functions (ported from FastLED or written fresh); the
example's exact show order.

## As built (2026-11-01)

Differences from the design above, decided while building:

- `Pinwheel` takes `color` and `background` (crisp arms read better than a palette on
  8x8); `Breathe`, `Chase`, `Blink` take single colors; `ColorCycle` has no "hard"
  mode (a `Blink` or a playlist of `Solid`s covers it).
- Transitions are a `Transition` struct (style, length, easing) with constructors
  (`fade`, `wipe(Edge, ..)`, ...) and a `Style` enum; wipes take an `Edge`, not a
  `Direction`.
- The two-step shortcut is `a.then(b, at, transition)` and `a.crossfade_to(b, at,
  over)`; there is no `for_time`.
- `render` on a `&dyn Animation` is written `(&animation).render(..)` (a provided
  generic method can't be called on a trait object).
- `Mirrored` on a matrix numbers the half-width surface row by row, so chain-order
  effects mirror too. `at_speed` above 100% wraps its clock sooner than 49 days.
- `Rain` on a strip shows as random flashes (each LED is a one-LED column).
- The scanner aliases (scanner, Larson, Cylon) give a `Comet` that bounces.
