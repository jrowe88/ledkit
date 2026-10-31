# ledkit Milestone 6: `ledkit-layout` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver `ledkit-layout`. It lets you treat part of a strip as its own strip (segments), address a matrix by (x, y) whatever its wiring, and draw text, shapes and images on it through `embedded-graphics`. It is proven on the owner's matrix.

**Architecture:**
- A small `Layout` trait maps (x, y) to a strip index.
- `MatrixLayout` covers one panel: its size, progressive or serpentine wiring, rows or columns first, and the starting corner.
- `Rotated<L>` and `Tiled` compose layouts.
- `Canvas` borrows a strip's pixel slice (`strip.pixels_mut()`) together with a layout and offers `set(x, y, color)`. With the `embedded-graphics` feature it is also a `DrawTarget`.
- `Segment` describes a range of a strip, optionally reversed, and borrows the pixels the same way.
- Nothing owns the `Strip`: you draw, the borrow ends, and you call `strip.show()`. Brightness, gamma, dithering and the power limit stay in core's pipeline.

**Tech Stack:**
- Rust stable, MSRV 1.82, `#![no_std]`, no allocation.
- Dependencies: `ledkit-core`, `ledkit-color`, and `embedded-graphics-core` 0.4 (optional, feature `embedded-graphics`).
- `embedded-graphics` 0.8 as a dev-dependency, for tests and examples.
- The example uses `ledkit-esp` (esp-hal RMT) on the ESP32-C6, with the matrix on an external pin.

**Spec:** `docs/superpowers/specs/2026-10-25-ledkit-design.md`, the "`ledkit-layout`" section: segments; matrices with progressive or serpentine wiring, starting corner, rotation and tiling; Adafruit 8×8, 16×16 and 8×32 out of the box; a 2D `Canvas`; an `embedded-graphics` `DrawTarget`. Also the "Left to the builder" note on layout and anim crates. **Roadmap:** milestone 6, "text and shapes on an 8×8 / 16×16 matrix".

**Starting point:** branch `m6-layout` from `main` (milestones 1–4 merged).

## Global Constraints

- License `MIT OR Apache-2.0`. The crate inherits the workspace package fields and `[lints] workspace = true`, and copies `LICENSE-MIT`, `LICENSE-APACHE` and a `NOTICE` like `ledkit-spi`'s.
- `#![cfg_attr(not(test), no_std)]`, no heap allocation, no nightly, MSRV 1.82, and `cargo +1.82 check` must pass with the feature on.
- Builds for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf` with `--all-features`.
- Every public item has a plain-English doc comment and an example. `missing_docs` and `clippy::pedantic` are enforced with `-D warnings`.
- Integer math per pixel, with no floating point in `index` or `set`. Use readable names.
- Layout never touches brightness, gamma or power. It writes colors into the strip's buffer, and `show()` does the rest.
- The crate depends on `ledkit-core` and `ledkit-color` only, plus optional `embedded-graphics-core`. Name the dependency exactly as written there.
- Commits end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Commit only the files the task touches.

## Decisions this plan makes

- **One crate now.** `ledkit-anim` stays a separate later crate that depends on this one; the spec leaves the split to the builder.
- **Coordinates are `u16`.** (0, 0) is the top-left of the picture as you look at the panel. `x` grows to the right and `y` grows down, matching `embedded-graphics`.
- **Out-of-range writes are ignored, never a panic.** `Canvas::set` does nothing outside the canvas, and `get` returns `None`. `embedded-graphics` routinely clips shapes and text partly off-screen, including at negative coordinates, so this is normal use, not an error.
- **Size mismatch is caught once, at construction.** `Canvas::new(pixels, layout)` returns `Err(LayoutTooBig { needed, available })` when the layout needs more LEDs than the slice holds. A slice longer than the layout is fine: the extra LEDs are simply not part of the canvas, so one strip can be a matrix plus a tail.
- **Wiring vocabulary**, kept to what a hobbyist can see on the panel:
  - `Wiring::Progressive` or `Wiring::Serpentine` (zig-zag)
  - `Order::RowsFirst` or `Order::ColumnsFirst`
  - `Corner::TopLeft`, `TopRight`, `BottomLeft` or `BottomRight`, for where the data input (pixel 0) sits
- **Builder style.** `MatrixLayout::new(width, height)` defaults to progressive, rows first, starting top-left. Chain `.serpentine()`, `.columns_first()` and `.starting_at(Corner::…)`. Every constructor and modifier is a `const fn`, so a layout can be a `const`.
- **Presets only where verified.** Add named constructors for Adafruit's 8×8 (product 1487), 16×16 (2547) and 8×32 (2294) only after checking their wiring against Adafruit's NeoMatrix guide or the product pages. Cite the source in a doc comment. If the wiring can't be confirmed, ship no preset and document how to find the wiring instead (next point).
- **Wiring finder.** The example's first mode lights pixel 0 red and the rest of strip row 0 green, so the user can read their panel's starting corner and direction in seconds. The README explains how to map what you see to the builder calls. This beats guessing for the many unbranded panels.
- **Rotation** is `Rotation::{None, Quarter, Half, ThreeQuarter}`, clockwise. `Rotated<L>` swaps width and height for quarter turns.
- **Tiling.** `Tiled::new(panel: MatrixLayout, across, down)` plus the same `Wiring`, `Order` and `Corner` choices applied to the panels themselves. Panel k's pixels follow panel k−1's in the chain.
- **Colors.**
  - `Canvas` works with any pixel `P` where `ledkit_color::Rgb: Into<P>` (both `Rgb` and `Rgbw` qualify).
  - `set` takes `impl Into<P>`.
  - `DrawTarget` uses `embedded_graphics_core::pixelcolor::Rgb888` as its color and converts through `Rgb`.
  - The `From<Rgb888> for Rgb` and `From<Rgb> for Rgb888` conversions live in `ledkit-layout` behind the feature. Orphan rules allow this because `Rgb888` is foreign: implement them as `From` in whichever direction the orphan rules permit, or as a helper `fn` if neither does. Report which.
- **The matrix example** runs on the ESP32-C6 through `ledkit-esp`, the RMT backend that just passed on hardware. The matrix's data line goes on **GPIO4** through the 74AHCT125. The settings at the top of `main.rs`:
  - `WIDTH` and `HEIGHT`
  - `LedType`
  - the wiring calls
  - `BRIGHTNESS` (10%)
  - `POWER_LIMIT`, `Milliamps(500)` with a comment: "safe on USB; a 16×16 at full white can draw over 10 A, so use a 5 V supply for more"
  - `DATA_PIN`

  The owner fills in the real size and chip at the hardware check.

## Review Focus

1. **Text or shapes drawn partly off the matrix** (negative or too-large coordinates, as `embedded-graphics` clipping produces) are drawn where visible, never panic, and never write past the slice. Pinned in Tasks 3 and 4.
2. **A layout bigger than the strip.** `Canvas::new` returns `LayoutTooBig { needed, available }` rather than panicking later. Pinned in Task 3.
3. **Every wiring combination is a one-to-one mapping.** For each of the 16 (wiring × order × corner) choices, and every rotation of a non-square 8×32, `index` hits each of `0..width*height` exactly once. Pinned in Tasks 1 and 2 as exhaustive property tests.
4. **Non-square rotation.** An 8×32 rotated a quarter turn is 32 wide and 8 tall, and (0, 0) lands on the correct corner. Pinned in Task 2.
5. **Power on big matrices.** The example sets a power limit by default, and the docs say why. Pinned in Task 5's code and README checks.

---

## File Structure

```
Cargo.toml                                   # (members glob already covers crates/*)
crates/ledkit-layout/
├── Cargo.toml, README.md, LICENSE-MIT, LICENSE-APACHE, NOTICE
└── src/
    ├── lib.rs          # crate docs (quick tour, finding your wiring), re-exports
    ├── layout.rs       # Layout trait, Corner, Wiring, Order, MatrixLayout
    ├── transform.rs    # Rotation, Rotated, Tiled
    ├── canvas.rs       # Canvas, LayoutTooBig
    ├── segment.rs      # Segment, SegmentMut
    └── graphics.rs     # feature "embedded-graphics": DrawTarget, OriginDimensions, color conversions
examples/esp32c6-matrix/   # esp-hal RMT, external matrix on GPIO4
.github/workflows/ci.yml   # no-std builds of ledkit-layout; example job
```

---

### Task 1: Crate, `Layout` trait and `MatrixLayout`

**Files:** Create the crate (Cargo.toml, README stub, licenses, NOTICE), `src/lib.rs` and `src/layout.rs`. Modify the root README's crate table (`ledkit-layout`: "in progress") and `.github/workflows/ci.yml` (the no-std job also builds `ledkit-layout --all-features` for both targets).

**Interfaces (produces):**
- `pub trait Layout { fn width(&self) -> u16; fn height(&self) -> u16; fn index(&self, x: u16, y: u16) -> Option<usize>; fn len(&self) -> usize { width * height } }`. `index` returns `None` outside the grid. Document that implementations must map the grid one-to-one onto `0..len()`.
- `pub enum Corner { TopLeft, TopRight, BottomLeft, BottomRight }`, `pub enum Wiring { Progressive, Serpentine }`, `pub enum Order { RowsFirst, ColumnsFirst }` (all `Copy`, `Debug`, `PartialEq`, `Eq`).
- `pub struct MatrixLayout`, with these `const fn`s:
  - `new(width: u16, height: u16)`, which panics for a zero size, so in a `const` it's a build error
  - `serpentine(self)` and `progressive(self)`
  - `columns_first(self)` and `rows_first(self)`
  - `starting_at(self, Corner)`
  - `index(&self, x, y) -> Option<usize>`, also as a `const fn`, with `Layout` delegating to it

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn progressive_rows_from_top_left() {
    let m = MatrixLayout::new(4, 3);
    assert_eq!(m.index(0, 0), Some(0));
    assert_eq!(m.index(3, 0), Some(3));
    assert_eq!(m.index(0, 1), Some(4));
    assert_eq!(m.index(4, 0), None);
    assert_eq!(m.index(0, 3), None);
}
#[test]
fn serpentine_rows_reverse_every_other_row() {
    let m = MatrixLayout::new(4, 3).serpentine();
    assert_eq!(m.index(3, 1), Some(4));   // row 1 runs right to left
    assert_eq!(m.index(0, 1), Some(7));
    assert_eq!(m.index(0, 2), Some(8));
}
#[test]
fn columns_first_from_bottom_right() {
    let m = MatrixLayout::new(4, 3).columns_first().starting_at(Corner::BottomRight);
    assert_eq!(m.index(3, 2), Some(0));
    assert_eq!(m.index(3, 0), Some(2));
    assert_eq!(m.index(2, 2), Some(3));
}
/// Review Focus 3: all 16 combinations on 4×3 and 8×32 are one-to-one onto 0..len.
#[test]
fn every_wiring_is_a_bijection() { /* for each Wiring × Order × Corner and size: collect index(x, y)
   for all x < w, y < h; sort; assert == (0..w*h).collect() */ }
```

Also add a doc example showing `const LAYOUT: MatrixLayout = MatrixLayout::new(16, 16).serpentine();`.

- [ ] **Step 2: Run them and see them fail.** Run `cargo test -p ledkit-layout`. Expected: compile errors.
- [ ] **Step 3: Implement.** The crate docs begin with the quick tour (Task 3 fills in the rest) and a "Finding your wiring" section: run the example's wiring finder, see where the red LED and the green row land, and map that to `starting_at`, `columns_first` and `serpentine`, with a small table.
- [ ] **Step 4: Run the Verify list.**
- [ ] **Step 5: Commit**: `ledkit-layout: Layout trait and MatrixLayout`.

---

### Task 2: Rotation, tiling and presets

**Files:** `src/transform.rs` and `src/layout.rs` (presets).

**Interfaces (produces):**
- `pub enum Rotation { None, Quarter, Half, ThreeQuarter }` (clockwise).
- `pub struct Rotated<L: Layout>`, with `pub const fn new(inner: L, rotation: Rotation)`. It implements `Layout`, swapping width and height for a quarter or three-quarter turn. Also add `MatrixLayout::rotated(self, Rotation) -> Rotated<MatrixLayout>`.
- `pub struct Tiled`, with `pub const fn new(panel: MatrixLayout, across: u16, down: u16)` and the same modifiers as `MatrixLayout`: `serpentine()`, `columns_first()` and `starting_at(Corner)`. These apply to the order of the panels. It implements `Layout` with width `panel.width * across` and height `panel.height * down`.
- Presets, as `MatrixLayout` associated `const fn`s: `adafruit_8x8()`, `adafruit_16x16()` and `adafruit_8x32()`. Add each only if verified (see Decisions), with a source link in its docs. Report which you could confirm.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn quarter_turn_swaps_size_and_moves_origin() {
    let r = MatrixLayout::new(8, 32).rotated(Rotation::Quarter);
    assert_eq!((r.width(), r.height()), (32, 8));
    // After a clockwise quarter turn, the picture's top-left is the panel's bottom-left.
    assert_eq!(r.index(0, 0), MatrixLayout::new(8, 32).index(0, 31));
}
#[test]
fn half_turn_maps_corners() {
    let m = MatrixLayout::new(4, 3);
    let r = m.rotated(Rotation::Half);
    assert_eq!(r.index(0, 0), m.index(3, 2));
}
/// Review Focus 3 + 4: every rotation of 8×32, and tiled 2×2 of 4×3 under every
/// panel-order choice, are one-to-one onto 0..len.
#[test]
fn transforms_are_bijections() { /* as Task 1's helper */ }
#[test]
fn tiled_panels_follow_each_other() {
    let t = Tiled::new(MatrixLayout::new(4, 3), 2, 1);
    assert_eq!((t.width(), t.height()), (8, 3));
    assert_eq!(t.index(4, 0), Some(12)); // first pixel of the second panel
}
```

- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Implement.** Make the bijection helper a shared `#[cfg(test)]` function, used by Tasks 1 and 2.
- [ ] **Step 4: Run the Verify list.**
- [ ] **Step 5: Commit**: `ledkit-layout: rotation, tiling and panel presets`.

---

### Task 3: `Canvas` and `Segment`

**Files:** `src/canvas.rs`, `src/segment.rs` and `lib.rs` (quick tour).

**Interfaces (produces):**
- `pub struct LayoutTooBig { pub needed: usize, pub available: usize }`, with `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Display` ("this layout needs {needed} LEDs but the strip has {available}") and `core::error::Error`.
- `pub struct Canvas<'a, P, L: Layout>`:
  - `pub fn new(pixels: &'a mut [P], layout: L) -> Result<Self, LayoutTooBig>`
  - `width()` and `height()`
  - `set(&mut self, x: u16, y: u16, color: impl Into<P>)`, which ignores positions outside the canvas
  - `get(&self, x, y) -> Option<P>`, where `P: Copy`
  - `fill(&mut self, color: impl Into<P>)`, which touches only the canvas's LEDs, not any tail beyond the layout
  - `clear(&mut self)`, where `P: Default`
  - `layout(&self) -> &L`
- `pub struct Segment { start: usize, len: usize, reversed: bool }`:
  - `pub const fn new(start: usize, len: usize)` and `pub const fn reversed(self)`
  - `pub fn of<'a, P>(&self, pixels: &'a mut [P]) -> Result<SegmentMut<'a, P>, LayoutTooBig>`
- `pub struct SegmentMut<'a, P>`, with `len()`, `set(i, color)` (ignores out-of-range), `get(i)`, `fill(color)` and `iter_mut()`, in segment order.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn canvas_draws_into_the_strip_buffer() {
    let mut strip = Strip::<Ws2812b, 12, _>::new(MemoryOutput::<Rgb, 12>::new());
    {
        let mut c = Canvas::new(strip.pixels_mut(), MatrixLayout::new(4, 3).serpentine()).unwrap();
        c.set(0, 1, Rgb::RED);
    }
    assert_eq!(strip.pixels()[7], Rgb::RED);
}
#[test] // Review Focus 1
fn off_canvas_writes_are_ignored() {
    let mut px = [Rgb::BLACK; 12];
    let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 3)).unwrap();
    c.set(4, 0, Rgb::RED); c.set(0, 3, Rgb::RED); c.set(u16::MAX, u16::MAX, Rgb::RED);
    assert!(px.iter().all(|p| *p == Rgb::BLACK));
}
#[test] // Review Focus 2
fn layout_too_big_is_an_error() {
    let mut px = [Rgb::BLACK; 10];
    assert_eq!(Canvas::new(&mut px, MatrixLayout::new(4, 3)).err(),
               Some(LayoutTooBig { needed: 12, available: 10 }));
}
#[test]
fn fill_leaves_the_tail_alone() { /* 12-pixel layout on a 14-pixel slice: fill RED; px[12], px[13] stay BLACK */ }
#[test]
fn rgbw_canvas_takes_rgb() { /* Canvas over [Rgbw; 4] with 2×2; set(…, Rgb::RED) stores Rgbw::from(Rgb::RED) */ }
#[test]
fn reversed_segment_counts_from_its_far_end() {
    let mut px = [Rgb::BLACK; 10];
    let mut s = Segment::new(2, 5).reversed().of(&mut px).unwrap();
    s.set(0, Rgb::RED);
    assert_eq!(px[6], Rgb::RED);
}
```

- [ ] **Step 2: Run them and see them fail.**
- [ ] **Step 3: Implement.** The crate docs' quick tour (runnable doctest, with `MemoryOutput`) shows the full flow:
  - make a `Strip`
  - in a block, make a `Canvas` from `strip.pixels_mut()` and draw
  - call `strip.show()` after the block

  Explain in one sentence why the canvas is in its own block: it borrows the pixels, so the strip can't send them until you're done drawing.
- [ ] **Step 4: Run the Verify list.**
- [ ] **Step 5: Commit**: `ledkit-layout: Canvas and Segment`.

---

### Task 4: `embedded-graphics` support

**Files:** `src/graphics.rs` and `Cargo.toml` (feature `embedded-graphics = ["dep:embedded-graphics-core"]`; dev-dependency `embedded-graphics = "0.8"`).

**Interfaces (produces):**
- With the feature on: `impl<P, L> DrawTarget for Canvas<'_, P, L>` with `Color = Rgb888` and `Error = core::convert::Infallible`, where `L: Layout` and `Rgb: Into<P>`. It also implements `OriginDimensions` (size = width × height).
  - Pixels at negative or out-of-range points are skipped.
  - `fill_solid` and `clear` may be overridden for speed, but only if the behaviour stays identical.
- Color conversions between `Rgb888` and `ledkit_color::Rgb`, per the Decisions note.

- [ ] **Step 1: Write the failing tests** (`#[cfg(all(test, feature = "embedded-graphics"))]`)

```rust
#[test]
fn draws_a_rectangle_outline() {
    let mut px = [Rgb::BLACK; 16];
    let mut c = Canvas::new(&mut px, MatrixLayout::new(4, 4)).unwrap();
    Rectangle::new(Point::new(0, 0), Size::new(4, 4))
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
        .draw(&mut c).unwrap();
    assert_eq!(c.get(0, 0), Some(Rgb::RED));
    assert_eq!(c.get(1, 1), Some(Rgb::BLACK));
}
#[test] // Review Focus 1
fn text_hanging_off_every_edge_does_not_panic() {
    let mut px = [Rgb::BLACK; 64];
    let mut c = Canvas::new(&mut px, MatrixLayout::new(8, 8).serpentine()).unwrap();
    let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);
    for x in [-20, -3, 0, 5, 30] { for y in [-12, 0, 9, 40] {
        Text::new("Hi!", Point::new(x, y), style).draw(&mut c).unwrap();
    } }
    assert!(px.iter().any(|p| *p != Rgb::BLACK));
}
```

- [ ] **Step 2: Run the tests and see them fail.** Run `cargo test -p ledkit-layout --features embedded-graphics`.
- [ ] **Step 3: Implement.** Add a crate-docs section, "Text and shapes", with a doctest that draws text onto an 8×8 canvas.
- [ ] **Step 4: Run the Verify list**, both with and without the feature.
- [ ] **Step 5: Commit**: `ledkit-layout: embedded-graphics DrawTarget`.

---

### Task 5: Matrix example on the ESP32-C6

**Files:** Create `examples/esp32c6-matrix/`, copying the structure from `examples/esp32c6-rmt`. Modify `.github/workflows/ci.yml` to add a job `example-esp32c6-matrix` that runs `cargo build --release --locked`.

**Interfaces:** Consumes `ledkit_esp::{RmtOutput, buffer_len}`, `Strip`, `Canvas`, `MatrixLayout` and the `embedded-graphics` feature.

- [ ] **Step 1: Write `src/main.rs`.**
  - Put the settings constants at the top, with plain comments (see Decisions).
  - Set `const LAYOUT: MatrixLayout` from them, with a comment pointing to the README's "Finding your wiring".
  - The program cycles through three modes, 5 s each (shorter where noted), forever:
    1. **Wiring finder:** pixel 0 red, the rest of the strip's first `WIDTH` pixels green, for 3 s, with serial printing what to look for.
    2. **Shapes:** a border rectangle, a diagonal line and a filled circle in three colors.
    3. **Scrolling text:** "ledkit" in `FONT_6X10`, scrolling right to left, one pixel per frame at about 30 ms per frame, with the hue shifting along the text.
  - Set dithering off, with the same comment as the other examples.
  - Pass `rmt.frequency().as_hz()` to `RmtOutput::new`, as `esp32c6-rmt` does.
- [ ] **Step 2: Build it.** Run `cargo build --release`. Expected: success, with no warnings.
- [ ] **Step 3: Write `README.md`.**
  - What you'll see.
  - Wiring: GPIO4 → 74AHCT125 → 330 Ω → the matrix's DIN; a 5 V supply; common ground. Add the capacitor note.
  - Power: the 500 mA default limit, why, and how to raise it with a proper supply.
  - Finding your wiring: map the finder's red and green to the builder calls, using the same table as the crate docs.
  - Troubleshooting:
    - scrambled text means wrong wiring;
    - mirrored text means the wrong corner;
    - a dim matrix is the power limit at work.
  - Status: "pending hardware test".
- [ ] **Step 4: Commit**: `examples: text and shapes on a matrix (ESP32-C6, RMT)`.
- [ ] **Step 5: Hardware check** (controller and owner). The owner sets `WIDTH`, `HEIGHT`, `LedType` and the wiring from the finder, then flashes. Confirm:
  - the shapes are in the right places, not mirrored;
  - the text reads left to right and scrolls smoothly;
  - nothing lights outside the shapes.

  Record the result, including the panel's size, chip and wiring, in the example README. If the panel is a verified Adafruit model, also record it in the preset docs.

---

### Task 6: Docs and roadmap

**Files:** the `ledkit-layout` README and `lib.rs` (kept in step); the root `README.md` (crate table, "Pick your board", and the gallery note for the matrix GIF); `AGENTS.md` (crate map); `CONTRIBUTING.md` (one line on adding a panel preset); `docs/superpowers/plans/2026-10-25-ledkit-roadmap.md` (the milestone 6 row as delivered, plus follow-ups).

- [ ] **Step 1: Finish the crate docs.**
  - Quick tour
  - Finding your wiring
  - Rotation and tiling
  - Segments
  - Text and shapes
  - Power on big matrices
  - Tested-on-hardware status, from Task 5
- [ ] **Step 2: Update the root README and AGENTS.md.** `ledkit-layout` is now "available".
- [ ] **Step 3: Update the roadmap.**
  - The milestone 6 row describes what was delivered.
  - Follow-ups:
    - an image (BMP) drawing example
    - verifying any presets left unconfirmed
    - layouts for rings (circle coordinates) if wanted later
- [ ] **Step 4: Verify, then commit**: `docs: ledkit-layout guide and roadmap`.

---

## Verify (every task, before committing; all must be clean)

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`
- `cargo test --workspace --all-features` and `cargo test -p ledkit-layout` (with the feature off)
- `cargo build -p ledkit-layout --all-features` for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`
- `cargo +1.82 check --workspace --all-features`
- Task 5: `cargo build --release` in the example
