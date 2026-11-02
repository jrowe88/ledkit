# ledkit-anim Implementation Plan (milestone 7)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new `no_std`, heap-free `ledkit-anim` crate: a pure per-pixel `Animation` trait, 16 effects, combinators (playlist with transitions, crossfade, layers, segments, modifiers), named palettes in `ledkit-color`, a terminal preview and an ESP32-C6 example.

**Architecture:** Every effect is `color_at(Position, &Frame) -> Rgb`, a pure function of place and time; `render` walks a `Surface` (strip, canvas, segment, slice) once per frame. Combinators are pure too: they ask their children for colors and blend, so nothing needs a buffer. Integer math only per pixel, with helpers in `ledkit_anim::math`.

**Tech Stack:** Rust 2021, MSRV 1.82, `no_std`, `ledkit-color`, `ledkit-layout`, `ledkit-core`; examples: `std` terminal preview, esp-hal 1.2 + `ledkit-esp` RMT.

**Spec:** `docs/superpowers/specs/2026-11-01-ledkit-anim-design.md`

## Global Constraints

- `#![cfg_attr(not(test), no_std)]`, no `alloc`, `unsafe_code = "forbid"` (workspace lints), MSRV 1.82, stable.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` (pedantic) clean; `cargo fmt`.
- No floating point in any per-pixel path. `Duration` → milliseconds happens in setters and once per frame.
- Every public item has a doc comment with an example (doc tests run); plain, beginner-level wording.
- Tests live in the module they test (`#[cfg(test)] mod tests`).
- `no_std` builds for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf` (thumbv6m has no 64-bit hardware multiply but `u64` math is fine; it has no atomics, so no `static` needing `Sync` tricks).
- Ported code (FastLED palettes) gets a `NOTICE` line. Nothing copied from WLED, blinksy, NeoPixelBus, FastLED_NeoMatrix, Pixelblaze.
- Every constructor and setter that can be `const fn` is `const fn` (needed for `Effect::ALL`).

## Review Focus

1. Tiny and odd surfaces: width 1, a 1x1 canvas, an empty slice, a segment of length 0, odd widths (center between pixels). Expect no panic, no divide by zero, sensible colors. Owned by Task 2 (render), Task 4/5/6 (each effect test module has a `tiny_surfaces` test rendering onto `[Rgb; 0]`, `[Rgb; 1]` and a 1x1 canvas).
2. Zero settings: period, cycle time, tail length, ring width, duration or density of 0. Expect no panic; treat 0 as "frozen" or "off" as each setting's doc says. Each effect's tests include `zero_settings_do_not_panic`.
3. Large times: `Time::from_millis(u32::MAX)` and wrap-around; `.at_speed(1000)` at large t. Expect no overflow panic in debug builds. Task 2 and Task 7 tests.
4. Long strips and big panels: 300 LEDs, a 64x32 canvas; distances and angles must not overflow `i32`/`u32`. Task 3 (`atan2`, `isqrt` extremes) and Task 6 (render a 64x32 canvas).
5. Playlist edges: empty playlist (black), one entry, all durations 0, transition longer than an entry. Task 8.

---

### Task 1: `ColorMap` and named palettes in `ledkit-color`

**Files:**
- Create: `crates/ledkit-color/src/palettes.rs`
- Modify: `crates/ledkit-color/src/gradient.rs` (add `ColorMap` trait + impls), `crates/ledkit-color/src/lib.rs` (`pub mod palettes; pub use gradient::ColorMap;`), `crates/ledkit-color/README.md`, `NOTICE`

**Interfaces:**
- Produces: `pub trait ColorMap { fn color_at(&self, position: u8) -> Rgb; fn wraps(&self) -> bool { false } }`; impls for `Gradient<N>` (wraps false) and `Palette<N>` (wraps true), forwarding to the inherent `const fn`.
- Produces: `ledkit_color::palettes::{RAINBOW, RED_ORANGE, JET, FIRE, HEAT, LAVA, OCEAN, FOREST, CLOUD, PARTY, SUNSET, ICE, VIRIDIS}` as `pub const` of `Gradient<N>` or `Palette<N>`. `RAINBOW` = `Palette::RAINBOW`. Wrapping (`Palette`): RAINBOW, OCEAN, FOREST, CLOUD, PARTY, LAVA. Non-wrapping (`Gradient`): RED_ORANGE, JET, FIRE, HEAT, SUNSET, ICE, VIRIDIS.
- JET: stops from MATLAB's definition: dark blue (0,0,143) at 0, blue (0,0,255) at 32, cyan (0,255,255) at 96, yellow (255,255,0) at 160, red (255,0,0) at 224, dark red (128,0,0) at 255.
- FIRE: black → red → orange → yellow → near-white (black at 0, white-yellow at 255) — `Flames` relies on 0 = black.
- VIRIDIS: 5–9 stops sampled from the published viridis (CC0) curve.
- FastLED-derived ones (LAVA, OCEAN, FOREST, CLOUD, PARTY, HEAT) may be reduced to 8 colors; add NOTICE line "FastLED (MIT): palette colors for LAVA, OCEAN, FOREST, CLOUD, PARTY, HEAT".

- [ ] **Step 1: Tests** in `palettes.rs`: `jet_ends_dark_blue_and_dark_red` (`JET.color_at(0) == Rgb::new(0,0,143)`, `JET.color_at(255) == Rgb::new(128,0,0)`); `fire_starts_black` (`FIRE.color_at(0) == Rgb::BLACK`); `wrapping_flags` (`ColorMap::wraps(&RAINBOW)`, `!ColorMap::wraps(&JET)`); `dyn_works` (`let m: &'static dyn ColorMap = &palettes::OCEAN; m.color_at(10);`). In `gradient.rs`: `color_map_matches_inherent` for both types.
- [ ] **Step 2:** `cargo test -p ledkit-color` → fails (missing items).
- [ ] **Step 3:** Implement. Module docs: a table of names and what they look like.
- [ ] **Step 4:** `cargo test -p ledkit-color` passes; clippy clean.
- [ ] **Step 5:** Commit `ledkit-color: ColorMap trait and named palettes`.

### Task 2: Crate skeleton: `Time`, `Frame`, `Position`, `Surface`, `Animation`, `Colors`

**Files:**
- Create: `crates/ledkit-anim/Cargo.toml` (deps: ledkit-color, ledkit-layout, ledkit-core by path + version "0.1.0", same metadata pattern as ledkit-layout), `README.md` (stub, filled in Task 10), `src/lib.rs`, `src/time.rs`, `src/animation.rs`, `src/surface.rs`, `src/colors.rs`
- Modify: `crates/ledkit-layout/src/segment.rs` — add `pub const fn start(&self) -> usize`, `pub const fn len(&self) -> usize`, `pub const fn is_empty(&self) -> bool`, `pub const fn is_reversed(&self) -> bool` (used by Task 9).
- Modify: `.github/workflows/ci.yml` no-std job: two `cargo build -p ledkit-anim --target ...` lines.

**Interfaces:**
- `pub struct Time(u32)` (ms): `const fn from_millis(u32)`, `const fn as_millis(self) -> u32`, `From<core::time::Duration>` (wrapping: `(d.as_millis() % (1 << 32)) as u32`). Derive `Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default`.
- `pub(crate) const fn millis(d: Duration) -> u32` saturating, for setters.
- `pub struct Frame { pub millis: u32, pub width: u16, pub height: u16 }` with `const fn len(&self) -> usize` (= w*h) and `const fn new(millis, width, height)`.
- `pub struct Position { pub x: u16, pub y: u16, pub index: u16 }`.
- `pub trait Animation { fn color_at(&self, at: Position, frame: &Frame) -> Rgb; fn render(&self, surface: &mut impl Surface, now: impl Into<Time>) where Self: Sized { .. } }` — render builds the `Frame`, loops `y` then `x`, skips `(x,y)` where `surface.index` is `None`, sets `index` to `min(i, u16::MAX)`.
- Blanket: `impl<A: Animation + ?Sized> Animation for &A`.
- `pub trait Surface { fn width(&self) -> u16; fn height(&self) -> u16; fn index(&self, x: u16, y: u16) -> Option<usize>; fn set(&mut self, x: u16, y: u16, color: Rgb); }` with impls: `[P] where P: From<Rgb>` (width = `min(len, u16::MAX)`, height 1), `SegmentMut<'_, P>`, `Canvas<'_, P, L: Layout>` (index from `layout().index`), `Strip<C: Chip, N, O>` (width N).
- `pub enum Colors { One(Rgb), Two(Rgb, Rgb), Map(&'static dyn ColorMap) }` with `const fn` constructors not needed (variants are public), methods `fn at(&self, position: u8) -> Rgb` (One: the color; Two: lerp), `fn wraps(&self) -> bool` (Map only), `fn scrolled(&self, phase: u8) -> Rgb` (wrapping: `at(phase)`; otherwise `at(math::triangle(phase))`). `impl From<Rgb> for Colors`. `Clone, Copy` (manual `Debug` printing the variant name).
- `pub enum Direction { Forward, Backward }` (default Forward; for rotations Forward = clockwise as you look at the panel) in `animation.rs`.

- [ ] **Step 1: Tests:** `time_from_duration_wraps` (`Time::from(Duration::from_millis(u64::from(u32::MAX) + 5)).as_millis() == 4`); `render_strip_slice` (a test animation returning `Rgb::new(x as u8, y as u8, index as u8)` rendered into `[Rgb::BLACK; 5]`: `pixels[3] == Rgb::new(3,0,3)`); `render_canvas_uses_layout` (4x3 serpentine canvas: the pixel at `(3,1)` gets `index` 4); `render_segment` (Segment::new(2,3) of 6 pixels: `pixels[2..5]` set, others black); `render_rgbw_strip` (into `[Rgbw; 2]`); `render_strip` (a `Strip<Ws2812b, 4, MemoryOutput<..>>`, pixel 2 set); `dyn_render` (`let a: &dyn Animation = &t; a.render(..)` compiles via the blanket impl); `empty_surfaces` (`[Rgb;0]`, 0-length segment); `colors_scrolled_ping_pongs` (`Colors::Two(BLACK, WHITE).scrolled(0) == BLACK`, `.scrolled(128)` ≈ WHITE, `.scrolled(255)` near BLACK).
- [ ] **Step 2:** Run `cargo test -p ledkit-anim` → fails.
- [ ] **Step 3:** Implement. `lib.rs` crate docs: placeholder sentence (full docs Task 10).
- [ ] **Step 4:** Tests pass; `cargo build -p ledkit-anim --target thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf` succeed.
- [ ] **Step 5:** Commit `ledkit-anim: crate skeleton, Animation, Surface, Time, Colors`.

### Task 3: `math` helpers

**Files:** Create `crates/ledkit-anim/src/math.rs` (public module, documented: "helpers for writing your own effects").

**Interfaces (all `pub const fn` where possible):**
- `sine(phase: u8) -> u8`: one full wave per 256, `sine(0) == 128`, `sine(64) == 255`, `sine(192) == 0` (±1). Quarter-wave table of 65 `u8` literals (generate with a throwaway script: `round(127.5 + 127.5*sin(2πi/256))`).
- `triangle(phase: u8) -> u8`: 0 at 0, 254/255 at 128, back down; `triangle(0)==0`, `triangle(128)>=254`.
- `atan2(dy: i32, dx: i32) -> u16`: full turn = 65536, 0 = pointing right (+x), increasing clockwise as you look at the panel (y grows down) — so `atan2(1, 0)` ≈ 16384. Error ≤ 1° (182 steps). `atan2(0,0) == 0`. Inputs up to ±2^20 without overflow.
- `isqrt(n: u32) -> u32` (floor).
- `hash(n: u32) -> u32` (integer avalanche hash, e.g. Wellons' lowbias32, written fresh); `hash2(a: u32, b: u32) -> u32 = hash(a ^ hash(b))`.
- `noise(x: u32, y: u32) -> u8`: 2D value noise, coordinates in 1/256 of a cell; lattice values from `hash2`, smoothstep interpolation in integers. Continuous: neighbours 1/256 apart differ by at most ~8.
- `scale_u8(value: u32, max: u32) -> u8`: `value*255/max` clamped, `max == 0` → 0.

- [ ] **Step 1: Tests:** `sine_key_points`, `sine_is_symmetric` (`sine(p) + sine(p.wrapping_add(128))` within 254..=256 for all p), `triangle_key_points`, `atan2_quadrants` (right 0, down ~16384, left ~32768, up ~49152, ±182), `atan2_large_inputs` (`atan2(1<<20, -(1<<20))` ≈ 24576), `isqrt_exact_and_floor` (0,1,15→3,16→4,u32::MAX→65535), `hash_spreads` (256 inputs → >200 distinct low bytes), `noise_is_continuous` (|noise(x,y)-noise(x+1,y)| ≤ 8 over a sweep), `noise_uses_full_range` (over a 64x64 sample min < 40, max > 215).
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: integer math helpers`.

### Task 4: Strip effects, part 1 — `Solid`, `Blink`, `Breathe`, `Rainbow`, `ColorCycle`, `GradientScroll`

**Files:** Create `crates/ledkit-anim/src/effects/mod.rs` and one file per effect (`solid.rs`, `blink.rs`, `breathe.rs`, `rainbow.rs`, `color_cycle.rs`, `gradient_scroll.rs`).

**Interfaces** (each: `#[derive(Clone, Copy, Debug)]`, `pub const fn new() -> Self`, `Default`, `const fn` setters returning `Self`, `impl Animation`). 1D effects use `at.x` and `frame.width`:
- `Solid`: `.color(Rgb)` (default `Rgb::WHITE`).
- `Blink`: `.color(Rgb)` (WHITE), `.off_color(Rgb)` (BLACK), `.period(Duration)` (1 s), `.duty(u8 percent)` (50). On while `t % period < period*duty/100`. Period 0 → always on.
- `Breathe`: `.color(Rgb)` (BLUE), `.period(Duration)` (4 s), `.levels(min: u8, max: u8)` (0, 255). Level follows a raised cosine: `min` at t=0, `max` at period/2.
- `Rainbow`: `.cycle_time(Duration)` (5 s, time for a color to go round once), `.width(u8)` (1 rainbow across the strip), `.saturation(u8)` (255), `.direction(Direction)`. Hue (u16 turn) = `x*65536*width/w` minus `t*65536/cycle` for Forward (colors move toward higher x). Uses `Hsv`.
- `ColorCycle`: `.colors(Colors)` (`Colors::Map(&palettes::RAINBOW)`), `.cycle_time(Duration)` (10 s). Whole surface one color: `colors.scrolled(t*256/cycle)`.
- `GradientScroll`: `.colors(Colors)` (RAINBOW map), `.cycle_time(Duration)` (5 s), `.width(u8)` (1), `.direction(Direction)`. `colors.scrolled(x*256*width/w - t*256/cycle)`.
- Every effect that has `.colors(Colors)` also gets the convenience setters `.palette(&'static dyn ColorMap)` and `.color(Rgb)`; two-color ones get `.colors_between(a: Rgb, b: Rgb)` (= `Colors::Two`). Setter `.colors(..)` takes `Colors`.

- [ ] **Step 1: Tests** per file: `Solid` returns its color everywhere; `Blink` on at t=0, off at t=600 ms with defaults, `duty(0)` always off; `Breathe` t=0 → `BLUE.scale(0)`… i.e. BLACK, t=2000 → BLUE; `Rainbow` x=0,t=0 → `Rgb::RED`, colors move forward (`color_at(x=1, t=dt) == color_at(x=0, t=0)` for `dt = cycle/w` on a 10-pixel strip, width 1); `ColorCycle` same color on every pixel; `GradientScroll` with `Colors::Two(BLACK, WHITE)` at t=0: x=0 BLACK, x=w/2 ≈ WHITE (ping-pong); plus `tiny_surfaces` and `zero_settings_do_not_panic` in each.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: solid, blink, breathe, rainbow, color cycle, gradient scroll`.

### Task 5: Strip effects, part 2 — `ColorWipe`, `Chase`, `Comet`, `Sparkle`, `Flames`

**Files:** `effects/color_wipe.rs`, `chase.rs`, `comet.rs`, `sparkle.rs`, `flames.rs`.

**Interfaces:**
- `ColorWipe`: `.color(Rgb)` (GREEN), `.background(Rgb)` (BLACK), `.wipe_time(Duration)` (2 s), `.direction(Direction)`, `.looping(bool)` (false: fill then hold; true: fill, then wipe back to background, repeat). Pixel x is lit when `x < t*w/wipe_time` (Forward).
- `Chase` (the marquee): `.color(Rgb)` (WHITE), `.background(Rgb)` (BLACK), `.size(u16)` (1 lit), `.spacing(u16)` (2 dark), `.speed(u16 pixels_per_second)` (10), `.direction(Direction)`. Runs on `at.index` and `frame.len()` (chain order, so it snakes on a matrix). Lit when `(index - offset) mod (size+spacing) < size` with `offset = t*speed/1000`.
- `Comet`: `.color(Rgb)` (CYAN), `.tail_length(u16)` (8), `.speed(u16 pixels_per_second)` (30), `.bounce(bool)` (false), `.direction(Direction)`. Head moves in 1/16-pixel steps; brightness falls linearly to 0 over the tail behind the head; without bounce the head runs from 0 to `w + tail` then restarts (tail leaves cleanly); with bounce it goes back and forth over `0..w` and the tail is always behind the direction of travel.
- `Sparkle`: `.colors(Colors)` (`One(WHITE)`), `.background(Rgb)` (BLACK), `.density(u8 percent)` (10), `.twinkle_time(Duration)` (1 s). Per pixel: offset `hash(index) % twinkle`; slot `(t+offset)/twinkle`; active when `hash2(index, slot) % 100 < density`; brightness = `triangle(phase within slot)`; color from `colors.at(hash2(index, slot) >> 8)`, blended over the background.
- `Flames`: `.colors(Colors)` (`Map(&palettes::FIRE)`), `.colors_between(start, end)`, `.size(u8 percent)` (80: how far up the flames reach), `.speed(u16 pixels_per_second)` (8, how fast the flicker rises). Base: bottom row on a matrix (`y = height-1`), index 0 on a strip (`height == 1`, uses x). Heat = `noise(across*256/3, (up*256 + t*speed*256/1000)/2)` scaled by a falloff that reaches 0 at `size%` of the height/length; color `colors.at(heat)`; with `Colors::Two`, heat 0 is `start`... and black fade: final color = `colors.at(heat).scale(heat)` so cold is black.

- [ ] **Step 1: Tests:** wipe half done at t=1 s on a 10-strip (pixels 0..5 lit, 5..10 background), held full at t=5 s, `looping` back to background at 4 s; chase pattern `size 1 spacing 2` at t=0 lights 0,3,6; moves one pixel per 100 ms; comet head pixel is `color` at t=0, the pixel ahead is BLACK, tail dims monotonically; bounce reverses (head position at `t` and at `2*w*1000/speed - t` equal); sparkle with density 0 is all background, density 100 lights > 0 pixels, deterministic (same t → same frame); flames: above `size` black, bottom row brighter on average than the row at 60% height over 20 frames, strip and 8x8 both render; `tiny_surfaces`, `zero_settings_do_not_panic`.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: color wipe, chase, comet, sparkle, flames`.

### Task 6: Matrix effects — `Swirl`, `Rings`, `Pinwheel`, `Plasma`, `Rain`

**Files:** `effects/geometry.rs` (shared: `pub(crate) struct Polar { distance16: u32, angle: u16 }` and `fn polar(at, frame, center: Option<(u16,u16)>) -> Polar` with coordinates in 1/16 pixel, default center `((w-1)*8, (h-1)*8)` i.e. between pixels on even sizes), `swirl.rs`, `rings.rs`, `pinwheel.rs`, `plasma.rs`, `rain.rs`.

**Interfaces:**
- `Swirl`: `.colors(Colors)` (RAINBOW), `.tightness(u8)` (32; 0 = straight spokes), `.arms(u8)` (1), `.turn_time(Duration)` (4 s), `.direction(Direction)`, `.center(x: u16, y: u16)`. Phase = `angle*arms/256 + distance16*tightness/64 ∓ t*256/turn_time`; `colors.scrolled(phase)`.
- `Rings` (concentric): `.colors(Colors)` (RAINBOW), `.ring_width(u8 pixels)` (4: distance for one full trip through the colors), `.cycle_time(Duration)` (2 s), `.outward(bool)` (true), `.center(x, y)`.
- `Pinwheel` (rotating arms / rotating X): `.color(Rgb)` (WHITE), `.background(Rgb)` (BLACK), `.arms(u8)` (4), `.turn_time(Duration)` (3 s), `.direction(Direction)`, `.center(x, y)`. Lit when `(angle*arms + rotation) mod 65536 < 32768`, edges softened over ~1/16 of an arm.
- `Plasma`: `.colors(Colors)` (RAINBOW), `.scale(u8 pixels)` (8, size of a blob), `.cycle_time(Duration)` (6 s). Average of four `sine` waves (x, y, x+y, distance from center) each with its own time speed.
- `Rain`: `.color(Rgb)` (GREEN), `.density(u8 percent)` (40, share of columns with a drop), `.speed(u16 pixels_per_second)` (8), `.trail_length(u16)` (4). Per column: period `(h + trail)*1000/speed`, offset `hash(x) % period`, active per cycle by `hash2(x, cycle) % 100 < density`, head at `((t+offset)%period)*speed/1000`, trail fades linearly upward.

- [ ] **Step 1: Tests:** swirl `tightness(0)`: two pixels on the same ray from the center (e.g. on a 9x9 with center (4,4): (5,4) and (7,4)) have equal colors; rings: on an 8x8, pixels (3,3),(4,3),(3,4),(4,4) equal colors (same distance); pinwheel 4 arms: color at (6,1) equals color at (6,6) rotated 90° i.e. 8x8 symmetric pairs `(x,y)` and `(7-y, x)` equal; plasma: deterministic and changes between t=0 and t=500 on the 8x8; rain with density 0 is all black, density 100 lights a drop in every column at some t; `big_panel` renders 64x32 canvas at `t = u32::MAX` without panic; each effect renders on a 1x1 canvas and a 100-pixel strip.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: swirl, rings, pinwheel, plasma, rain`.

### Task 7: Modifiers and layers

**Files:** Create `crates/ledkit-anim/src/combine/mod.rs` (the `AnimationExt` trait), `combine/modifiers.rs`, `combine/layer.rs`.

**Interfaces:**
- `pub trait AnimationExt: Animation + Sized` with blanket impl for all `A: Animation`; methods (all return structs that implement `Animation`, fields private, `Clone, Copy, Debug` where children are):
  - `reversed(self) -> Reversed<Self>`: `x → w-1-x`, `index → len-1-index`.
  - `mirrored(self) -> Mirrored<Self>`: child sees width `ceil(w/2)` (and len halved likewise for `index` on strips); right half mirrors left.
  - `at_speed(self, percent: u16) -> AtSpeed<Self>`: child time = `t*percent/100` in `u64`, truncated to `u32` (wraps).
  - `starting_at(self, start: Duration) -> StartingAt<Self>`: child time = `t - start`, or 0 before `start`.
  - `dimmed(self, level: u8) -> Dimmed<Self>`: `color.scale(level)`.
  - `layer<T: Animation>(self, top: T, blend: Blend) -> Layer<Self, T>`.
  - (Task 8 adds `then` and `crossfade_to`.)
- `pub enum Blend { Over, Add, Multiply, Screen, Lighten, Darken, Average }` with `pub const fn mix(self, base: Rgb, top: Rgb) -> Rgb`. `Over`: `a = max(top.r, top.g, top.b)`; each channel `min(255, top + base*(255-a)/255)`.

- [ ] **Step 1: Tests:** reversed rainbow pixel 0 == plain rainbow pixel w-1; mirrored on 10 pixels: pixel 2 == pixel 7; at_speed(200) at t == plain at 2t; at_speed(1000) at `u32::MAX` doesn't panic; starting_at(1 s) at t=0 and t=1 s both equal plain at t=0; dimmed(0) is black; blend table: `Over` black top → base, white top → white, `(128,0,0)` over `(0,0,200)` → `(128,0,100)` ±1; `Add` saturates; `Multiply` with white is identity; `Screen` with black is identity; `Lighten`/`Darken` per channel; `Average` of black and white is 127 or 128.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: modifiers and layers with blend modes`.

### Task 8: Transitions, `Then`/crossfade, `Playlist`

**Files:** `combine/transition.rs`, `combine/then.rs`, `combine/playlist.rs`.

**Interfaces:**
- `pub enum Easing { Linear, EaseIn, EaseOut, EaseInOut }` with `pub const fn apply(self, progress: u8) -> u8` (quadratic in/out, smoothstep for in-out; endpoints exact).
- `pub enum Edge { Left, Right, Top, Bottom }` (where a wipe starts).
- `pub enum Style { Cut, Fade, FadeThroughBlack, Wipe(Edge), Dissolve, Iris }`.
- `pub struct Transition { style: Style, millis: u32, easing: Easing }` with `const fn` constructors `cut()`, `fade(Duration)`, `fade_through_black(Duration)`, `wipe(Edge, Duration)`, `dissolve(Duration)`, `iris(Duration)`, setter `.easing(Easing)` (default `EaseInOut` for fades, `Linear` for wipe/dissolve/iris), `const fn length(&self) -> u32`, and `pub fn mix(&self, from: Rgb, to: Rgb, progress: u8, at: Position, frame: &Frame) -> Rgb` (progress before easing; 0 → `from`, 255 → `to` exactly for every style). Dissolve uses `hash(index)`; iris uses distance from the center (`geometry::polar`).
- `AnimationExt::then<B: Animation>(self, next: B, at: Duration, transition: Transition) -> Then<Self, B>`: before `at` → self; from `at` the transition runs over `transition.length()`, with `next` on its own clock (`t - at`) and self continuing; after → next. `AnimationExt::crossfade_to(self, next, at, over)` = `then(next, at, Transition::fade(over))`.
- `pub struct Entry<'a> { .. }` with `pub const fn new(animation: &'a dyn Animation, duration: Duration) -> Self`, `pub const fn transition(self, t: Transition) -> Self` (the transition used when this entry comes in; default `Transition::cut()`).
- `pub struct Playlist<'a> { entries: &'a [Entry<'a>], looping: bool }`: `pub const fn new(entries: &'a [Entry<'a>]) -> Self` (loops), `pub const fn play_once(self) -> Self`. Entry i runs on its own clock from its start; during the first `transition.length()` ms of entry i (capped to entry i's duration), the previous entry keeps running on its clock and is mixed into i. When looping, entry 0's transition mixes from the last entry, except during the very first pass (t < total). `play_once`: after the end, the last entry holds at its final time (`duration - 1`). Empty → BLACK; total 0 → entry 0 at time 0.

- [ ] **Step 1: Tests:** every style: progress 0 → from, 255 → to on a 10-strip and 8x8; fade at 128 ≈ midpoint; fade-through-black at 128 is black; wipe from Left at 128 switched the left half only; dissolve at 128 switched 30–70% of 100 pixels; iris at 128 switched the center but not the corners of an 8x8; easing endpoints; `then`: before `at` = A, at `at + length` = B at B's time `length`, and B sees time 0 at `at` (test animation reporting time as color); playlist: two entries 1 s each with cut: t=0.5 s → first, t=1.5 s → second at its time 500, t=2.5 s → first again; fade transition 200 ms mid-way mixes; `play_once` holds the last frame at t = 10 s; empty playlist black; transition longer than the entry doesn't panic; `Effect` values (Task 9) not needed here — use test animations.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: transitions, crossfade and playlists`.

### Task 9: `Segments`, the `Effect` enum, prelude

**Files:** `combine/segments.rs`, `src/effect.rs`, `src/prelude.rs` (or a `pub mod prelude` in `lib.rs`).

**Interfaces:**
- `pub struct Segments<'a> { parts: &'a [(Segment, &'a dyn Animation)] }`, `pub const fn new(parts) -> Self`. Matches `at.index` against each `Segment` (`start()`, `len()`, `is_reversed()` from Task 2), first match wins; child sees `Position { x: local, y: 0, index: local }` and `Frame { width: len, height: 1, millis }`; no match → BLACK.
- `pub enum Effect { Solid(Solid), Blink(Blink), Breathe(Breathe), Rainbow(Rainbow), ColorCycle(ColorCycle), GradientScroll(GradientScroll), ColorWipe(ColorWipe), Chase(Chase), Comet(Comet), Sparkle(Sparkle), Flames(Flames), Swirl(Swirl), Rings(Rings), Pinwheel(Pinwheel), Plasma(Plasma), Rain(Rain) }`, `impl Animation` (match), `pub const ALL: [Effect; 16]` (defaults), `pub const fn name(&self) -> &'static str` (lowercase with spaces, e.g. `"color wipe"`), `pub fn from_name(name: &str) -> Option<Effect>` ignoring case, spaces, `-` and `_`, accepting the aliases: static, strobe, flash, pulse, color loop, fade, palette scroll, wipe, fill, loading, theater chase, marquee, running lights, meteor, scanner, larson, cylon, glitter, twinkle, fire, candle, spiral, concentric, rotating arms, rotating x, matrix rain, digital rain. `From<Rainbow> for Effect` etc. for each.
- `prelude`: `Animation, AnimationExt, Surface, Time, Frame, Position, Colors, Direction`, all effects, `Effect`, `Blend, Transition, Easing, Edge, Style, Playlist, Entry, Segments`, plus `ledkit_color::{palettes, ColorMap, Rgb, Color}` re-exports.

- [ ] **Step 1: Tests:** segments: two parts on 10 pixels `(Segment::new(0,4), red solid)`, `(Segment::new(6,4).reversed(), rainbow)`: pixels 0..4 red, 4..6 black, child of second sees index 0 at pixel 9; child frame width is 4; `from_name("Rotating-X")` is a Pinwheel, `from_name("MARQUEE")` a Chase, `from_name("nope")` None; every `ALL[i].name()` round-trips through `from_name`; names unique; `ALL` renders on the 8x8 at several times without panic; prelude doc test with the first-program example compiles.
- [ ] **Step 2–4:** fail, implement, pass.
- [ ] **Step 5:** Commit `ledkit-anim: segments, effects by name, prelude`.

### Task 10: Docs

**Files:** `crates/ledkit-anim/src/lib.rs` crate docs and `README.md` (same contents: prelude-first first program on a strip, the matrix version with `Canvas`, the `Time` conversion line for esp-hal, effect table with aliases and settings, palettes, combining effects, "write your own effect" in about a dozen lines, the 49-day wrap note, render does not show/brightness); `CONTRIBUTING.md` "Add an effect" section; `AGENTS.md` (crate map: ledkit-anim exists, depends on color, layout, core; module list); `docs/superpowers/plans/2026-10-25-ledkit-roadmap.md` (row 7 done, notes: design and plan paths, follow-ups: text/fonts, stateful effects, border marquee, rectangular regions, hardware status); `README.md` at the root (crate list) if it lists crates; `NOTICE`.

- [ ] **Step 1:** Write. **Step 2:** `cargo test --workspace --all-features` (doc tests) and `RUSTDOCFLAGS=-D warnings cargo doc -p ledkit-anim --no-deps` pass. **Step 3:** Commit `docs: ledkit-anim guide, roadmap, crate map`.

### Task 11: `examples/anim-preview` (desktop terminal preview)

**Files:** `examples/anim-preview/{Cargo.toml, Cargo.lock, .gitignore, README.md, src/main.rs}`; CI job `example-anim-preview` (stable, `cargo build --locked`, `cargo clippy --locked -- -D warnings`).

**Behavior:** `cargo run -- swirl` (any `Effect::from_name` name; `list` prints the names; `--strip 30` for a strip instead of 8x8; `--demo` runs the same playlist as the ESP32 example). Renders into `[Rgb; N]` through a `Canvas` with `MatrixLayout::new(8,8)`, prints each frame as two-space truecolor blocks (`\x1b[48;2;r;g;bm  `) at ~30 fps with cursor-home, for 10 s or until Ctrl-C. `std` only, no extra dependencies.

- [ ] **Step 1:** Write it. **Step 2:** `cargo run -- list` lists 16 names; `cargo run -- swirl` for a second shows changing frames (eyeball). **Step 3:** Commit `examples: anim-preview in the terminal`.

### Task 12: `examples/esp32c6-anim`

**Files:** copy the scaffolding of `examples/esp32c6-matrix` (`.cargo/config.toml`, `rust-toolchain.toml`, `.gitignore`, Cargo profile) into `examples/esp32c6-anim`; `src/main.rs`, `README.md`, `Cargo.lock`; CI job `example-esp32c6-anim` mirroring `example-esp32c6-matrix`'s steps.

**Behavior:** `const MATRIX: bool = true;` picks the 8x8 (NeoPixel 8x8: `MatrixLayout::new(8, 8).columns_first().starting_at(Corner::BottomLeft)` as tested in milestone 6) or a strip (`const STRIP_LEDS: usize = 100`, `type LedType = Ws2812b`). Data on GPIO4 through the 74AHCT125 (README wiring: pin 1 OE to GND, 14 VCC 5 V, 7 GND, data in pin 2, out pin 3). Brightness 0.2, a power limit, dithering off (as other examples). Clock: `esp_hal::time::Instant::now()` at start, each frame `Time::from_millis(start.elapsed().as_millis() as u32)`. Matrix show: `Playlist` of swirl, rings, pinwheel, plasma, flames, rain (8 s each, alternating fade / iris / dissolve transitions). Strip show: rainbow, comet laid over a slow gradient scroll (`Blend::Over`), chase, sparkle, flames, then `Segments` (three thirds: comet, sparkle, breathe), 8 s each with fades. Frames every ~16 ms.

- [ ] **Step 1:** Write. **Step 2:** `cargo build --release --locked` and `cargo clippy --release -- -D warnings` in the example directory succeed (needs the riscv32imac target, installed). **Step 3:** Commit `examples: esp32c6-anim, effects on a strip and the 8x8`.

### Task 13: Final verification and PR

- [ ] Run the full CI set: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo test --workspace --all-features`, both `no_std` builds for every crate, `cargo +1.82 check --workspace --all-features` (if the toolchain is installed), both examples' build + clippy.
- [ ] Whole-branch review by a fresh reviewer on the most capable model; fix findings.
- [ ] Ask the owner before pushing and opening the PR (owner preference).
