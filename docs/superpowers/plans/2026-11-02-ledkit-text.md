# ledkit-text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new `ledkit-text` crate (fonts, text areas, a ticker) whose look is shared, by
construction or by checked fixtures, with the text preview page.

**Architecture:** One Python generator (`tools/fontgen.py`) bakes every bitmap face (X11 and
Tiny 3x5) at 1x/1.25x/1.5x/2x, per glyph, into binary coverage tables for the crate and into
JSON for the preview page. The segment faces stay as JavaScript drawing code in the preview
(moved into a DOM-free `glyphs.js`); a Node script turns them into reference fixtures that
the crate's integer-maths ports are tested against. The crate rasterises nothing ahead of
time: each frame, every LED in a box asks the font for its coverage and mixes the
letters/outline/behind paints.

**Tech Stack:** Rust 2021, MSRV 1.82, `no_std`; Python 3.12 (generator, no packages); Node
(already on GitHub's ubuntu runners) for the reference fixtures.

**Spec:** `docs/superpowers/specs/2026-11-02-ledkit-text-design.md`

## Global Constraints

- `no_std`, no `alloc`, no `unsafe` (`unsafe_code = "forbid"` is a workspace lint), MSRV 1.82.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings` clean with workspace lints (pedantic).
- Integer maths per pixel; floating point only in generators and tests.
- Every public item has docs and an example; readable names for beginners.
- Tests live in the module they test (`#[cfg(test)] mod tests`).
- Fonts only from public-domain/MIT sources or original work, with a `NOTICE` line.
- Characters: printable ASCII (space to `~`) only.
- The preview is the reference for looks; change both together.
- Hardware is checked by the owner by eye (ESP32-C6, GPIO4, Adafruit NeoPixel 8x8, columns_first, BottomLeft).
- Commit after each task. Push or open a PR only when the owner says so.
- Before each commit: `cargo fmt --all -- --check`, `cargo test --workspace --all-features`, the clippy line above.

## Review Focus

- **Text longer than the capacity, or non-ASCII text (é, emoji):** cut off at capacity on a
  character boundary, non-ASCII shown as `?`, never a panic. Test in Task 7.
- **A box partly or wholly off the panel, or zero-sized:** draws only the LEDs that exist,
  no panic, no wrap-around. Test in Task 7.
- **Text wider or taller than its box with centre/middle alignment:** clipped on both sides
  around the centre (floor division of a negative leftover, as the preview's `Math.floor`).
  Test in Task 7.
- **A ticker whose list is empty, or set while the gap is showing, or a time that jumps
  backwards (a restarted clock):** empty shows blank, a list set during the gap starts
  right after it, a backwards time restarts the current message instead of underflowing.
  Test in Task 8.
- **Speed on a big panel:** 192x64 with 40-row 14-segment digits and Plasma letters must
  still run near 30 fps on an ESP32-C6. Not testable on the host; Task 9's terminal demo
  prints frame time, and the example README asks the owner to note the fps.

---

### Task 0: Branch

- [ ] **Step 1:** `git -C C:/Dev/jrowe88/ledkit worktree add -b m8-text ../ledkit-m8 m7-anim` (or from `main` if `m7-anim` has been merged), then `git cherry-pick` the spec and plan commits from branch `text-design`.
- [ ] **Step 2:** `cargo test --workspace --all-features` passes on the new branch before anything changes.

### Task 1: Font generator

**Files:**
- Create: `tools/fontgen.py`, `tools/test_fontgen.py`
- Create: `assets/fonts/ledkit-tiny-3x5.txt` (the `TINY` table moved out of `site/text-preview/template.html`)
- Create (generated): `crates/ledkit-text/src/faces/data/<face>.bin` for each face, and `crates/ledkit-text/src/faces/data/mod.rs`
- Create (generated): `site/text-preview/faces.json`
- Modify: `site/text-preview/build.py` (import `read_bdf` from `tools/fontgen.py` instead of its own copy)

**Interfaces:**
- Produces, for each face: name (`4x6` ... `10x20`, `tiny`), cell width and height, and for each size in `[1, 1.25, 1.5, 2]` an output `(width, height)` and a table of 95 glyphs (codes 32..=126) of `width*height` bytes, each the **sum of 16 samples** (0..=16).
- `.bin` layout: for size index 0..4: glyph 0..95, rows top to bottom, one byte per LED. `mod.rs` holds one `pub(crate) static` per face (see Task 4 for the type) built with `include_bytes!`.
- `faces.json`: `{name: {sizes: [{w, h, glyphs: [string of 'a'+sum per LED, ...]}]}}`.

- [ ] **Step 1: Write the failing tests** in `tools/test_fontgen.py` (plain `unittest`):
  - `test_one_x_is_the_font`: 1x sums of `5x8` `'A'` are 16 exactly where the BDF has a bit set and 0 elsewhere.
  - `test_sizes`: `5x8` at 1.25x is `(round(5*1.25), round(8*1.25))` = `(6, 10)`; at 2x `(10, 16)`.
  - `test_scale2x_rounds_a_diagonal`: on the 2x2 pattern `[[1,0],[0,1]]`, one Scale2x pass gives the 4x4 EPX result (write it out).
  - `test_isolated_glyph`: a glyph's table depends only on that glyph (pixels outside its cell count as 0).
  - `test_missing_is_question_mark`: any code the BDF lacks gets the table of `'?'`; for `tiny`, `'a'` equals `'A'` and `'@'` equals `'?'`.
  - `test_range`: every byte is 0..=16.
- [ ] **Step 2:** `python tools/test_fontgen.py` fails (no module).
- [ ] **Step 3: Implement `tools/fontgen.py`.** Per glyph cell (font width x height; `tiny` cell is 4x5, its 3x5 shape plus a blank column, as the preview): Scale2x three times with outside = 0, then for each output LED `(x, y)` with `s = size`, sample `i, j` in 0..4: `hx = floor(((x + (i+0.5)/4) / s) * 8)`, `hy` likewise, add the 8x pixel if inside. This is the preview's `antialias` applied to one cell. 1x is the bitmap times 16. `--check` exits non-zero if any generated file differs; with no flag, it writes them. Line endings `\n`.
- [ ] **Step 4:** tests pass; `python tools/fontgen.py` writes the files; `python tools/fontgen.py --check` passes.
- [ ] **Step 5:** commit.

### Task 2: Preview uses the baked faces (owner checks the look)

**Files:**
- Create: `site/text-preview/glyphs.js` (DOM-free: `SEGS`, `SEG14`, `renderSeg`, `renderSeg14`, `downsample`, `sharpen(v, k)`, `bitmap`)
- Modify: `site/text-preview/template.html`, `site/text-preview/build.py`, regenerate `index.html`
- Modify: `.github/workflows/ci.yml` `site` job: add `python tools/test_fontgen.py` and `python tools/fontgen.py --check`; `.github/workflows/pages.yml`: add `tools/**` to the paths

- [ ] **Step 1:** `build.py` injects `faces.json` (as `const FACES_DATA = ...`) and `glyphs.js` in place of `/*FONTS*/`.
- [ ] **Step 2:** In the template, bitmap and tiny faces draw from `FACES_DATA`: each glyph advances by its baked width plus the spacing in LEDs (not scaled), and the shown value is `sharpen(sum/16, k)` (1x is unaffected). Remove `renderBdf`, `renderTiny`, `antialias`, `scale2x`, `sized`, `scaleBlock` and the `TINY` table. Segment faces call `glyphs.js` with `k` passed in.
- [ ] **Step 3:** `python site/text-preview/build.py` then `--check` passes; open `site/text-preview/index.html` in the browser pane, check at 375px wide there is no sideways scroll and the console has no errors.
- [ ] **Step 4:** commit, then **stop and ask the owner to look at the preview** (local file) at 1.25x/1.5x/2x before going on. Their changes come back here.

### Task 3: Segment reference fixtures

**Files:**
- Create: `site/text-preview/reference.mjs`
- Create (generated): `crates/ledkit-text/src/faces/segments.fixtures`
- Modify: `.github/workflows/ci.yml` `site` job: `node site/text-preview/reference.mjs --check`

**Interfaces:**
- Produces a plain text file the Rust tests read with `include_str!`. One case per block:
  `case <seg7|seg7slant|seg14> height=<H> text=<text>` then `<width> <height>` then `height`
  lines of `width` space-separated integers: the **pre-sharpen** coverage in 64ths (the 8x
  drawing summed over each LED, as `downsample(.., 8)` times 64). Blank line between cases.
- Cases: for each face, heights 5 (7 for seg14), 8, 10, 16, 30, 40, texts `0123456789`, `12:34`, `3.5`, and for seg14 also `ABCDEFGHIJKLMNOPQRSTUVWXYZ`, `?!-+/`.

- [ ] **Step 1:** `reference.mjs` loads `glyphs.js` with `vm.runInNewContext`, renders each case at 8x without sharpening (`renderSeg(text, H*8, slant, false)` / the 8x stage of `renderSeg14`), downsamples, writes the file; `--check` compares.
- [ ] **Step 2:** run it; check `--check` passes; commit.

### Task 4: Crate, fonts, paints

**Files:**
- Create: `crates/ledkit-text/Cargo.toml` (like `ledkit-anim`'s; deps `ledkit-color`, `ledkit-anim`; dev-dep `ledkit-layout`), `README.md`, `src/lib.rs`, `src/font.rs`, `src/edges.rs`, `src/paint.rs`, `src/faces/mod.rs`, `src/faces/bitmap.rs`
- Modify: `.github/workflows/ci.yml` `no-std` job: the two `cargo build -p ledkit-text --all-features --target ...` lines

**Interfaces (produces):**
- `pub enum Size { X1, X1_25, X1_5, X2 }` (default `X1`).
- `pub enum Edges { Soft, Medium, Crisp, VeryCrisp }` (default `Medium`; k = 1, 2, 3.5, 6).
- `pub(crate) const fn sharpen(covered: u16, out_of: u16, edges: Edges) -> u8`: `clamp((v - 0.5) * k + 0.5)` with `v = covered/out_of`, scaled to 0..=255, rounded to nearest. With `k2 = 2k` (2, 4, 7, 12): `((2*covered - out_of) * k2 + 2*out_of) * 255 / (4*out_of)`, clamped, in `i32`.
- `pub(crate) struct BitmapFace { pub name: &'static str, pub sizes: [BakedSize; 4] }`, `pub(crate) struct BakedSize { pub width: u8, pub height: u8, pub data: &'static [u8] }`.
- `#[derive(Copy, Clone)] pub struct Font` with:
  - consts `X11_4X6, X11_5X7, X11_5X8, X11_6X10, X11_6X13_BOLD, X11_7X13_BOLD, X11_7X14_BOLD, X11_8X13_BOLD, X11_9X15_BOLD, X11_9X18_BOLD, X11_10X20, TINY_3X5`, each holding `&'static BitmapFace` (never an enum matched over all faces, so the linker drops unused faces);
  - `pub const fn seven_segment(height: u16) -> Font`, `seven_segment_slanted(height: u16)`, `fourteen_segment(height: u16)` (heights below 5, or 7 for 14-segment, are raised to it);
  - builders `size(self, Size)`, `edges(self, Edges)`, `hard_edges(self)` (segment faces: cut at half), `spacing(self, u8)` (0..=3 typical, any accepted);
  - `pub fn height(&self) -> u16`, `pub fn text_width(&self, text: &[u8]) -> u16`, and `pub(crate) fn coverage(&self, text: &[u8], x: i32, y: i32) -> u8` (0..=255 after sharpening; 0 outside the text). Coverage works on the whole string because the segment faces lay a string out in 8x pixels (Tasks 5 and 6); bitmap faces are monospaced, so the glyph is `x / (baked width + spacing)`.
- `#[derive(Copy, Clone)] pub enum Paint { Color(Rgb), Effect(Effect), Custom(&'static (dyn Animation + Sync)) }`, `impl From<Rgb>`, `From<Effect>`, and `From<T>` for each of the 16 effect types (`Solid, Blink, Breathe, Rainbow, ColorCycle, GradientScroll, ColorWipe, Chase, Comet, Sparkle, Flames, Swirl, Rings, Pinwheel, Plasma, Rain`) via a macro; `pub(crate) fn color_at(&self, at: Position, frame: &Frame) -> Rgb`.
- `pub mod prelude` re-exporting `Font, Size, Edges, Paint, TextArea, Ticker, Across, UpDown`, plus `Rgb` and the effect types.

- [ ] **Step 1: Failing tests:**
  - `edges::tests::sharpen_matches_the_preview`: for every `covered` 0..=16 and each `Edges`, `sharpen(c, 16, e)` equals `round(clamp((c/16 - 0.5)*k + 0.5) * 255)` computed in the test with `f32`.
  - `font::tests::one_x_is_exact`: `Font::X11_5X8` `'A'` coverage is 255 where the BDF row strings (copy two rows into the test) have `1`, else 0.
  - `font::tests::sizes`: `Font::X11_5X8.size(Size::X1_25).height() == 10`, `text_width(b"AB") == 12`; with `.spacing(2)` 16.
  - `font::tests::missing_and_case`: `TINY_3X5` coverage of `b'a'` equals `b'A'`; of `b'@'` equals `b'?'`; of `200` (non-ASCII) equals `b'?'`.
  - `paint::tests::from_color_and_effect`: `Paint::from(Rgb::RED).color_at(..)` is red; `Paint::from(Plasma::new())` returns what `Plasma::new().color_at` returns at the same position and frame.
- [ ] **Step 2:** `cargo test -p ledkit-text` fails to compile.
- [ ] **Step 3:** implement; bitmap coverage = `sharpen(sum, 16, edges)` where `sum` is the baked byte.
- [ ] **Step 4:** tests pass; both `no_std` builds pass; clippy clean.
- [ ] **Step 5:** commit.

### Task 5: Seven-segment face

**Files:** Create `crates/ledkit-text/src/faces/seven.rs`; Modify `font.rs` to route to it.

**Interfaces:** `pub(crate) fn seven_coverage(text: &[u8], height: u16, slanted: bool, x: i32, y: i32) -> u16` (covered 64ths, 0..=64; 0 outside) and `pub(crate) fn seven_width(text: &[u8], height: u16, slanted: bool) -> u16` (LEDs, the preview's `ceil(width8 / 8)`). Extra `spacing` adds `8 * spacing` 8x pixels after each glyph.

Port the preview's `renderSeg` exactly, in integers: the 8x drawing is `renderSeg` at height `8H` (its own `t`, `W`, `gap`, `mid`, `skew` computed from `8H` with JS `Math.round` = round half up), and an LED's coverage counts the set pixels of its 8x8 block. Slant: a pixel at 8x row `y8` is shifted right by `round(skew8 * (1 - y8/(8H-1)))`; compute that with integers. Characters are mapped as the preview does (`SEGS` key if present, else upper case); anything without a form is blank. Layout is the preview's: each glyph advances by its width plus `2*gap` in 8x pixels, the slant's `skew` is added once for the string, and glyph starts stay in 8x pixels (not rounded to LEDs).

- [ ] **Step 1: Failing test** `seven::tests::matches_the_preview`: for every `seg7` and `seg7slant` case in `segments.fixtures`, every LED's `seven_coverage` equals the fixture value exactly (it's all integer maths in both). Hard-edged: `>= 32` lit matches the preview's `v >= 0.5`.
- [ ] **Step 2–4:** fails, implement, passes.
- [ ] **Step 5:** commit.

### Task 6: Fourteen-segment face

**Files:** Create `crates/ledkit-text/src/faces/fourteen.rs`; Modify `font.rs`.

**Interfaces:** `pub(crate) fn fourteen_coverage(text: &[u8], height: u16, x: i32, y: i32) -> u16` (64ths) and `pub(crate) fn fourteen_width(text: &[u8], height: u16) -> u16`, laid out as `renderSeg14` does (narrow `:` `.` `,`).

Port `renderSeg14`'s capsules into fixed point (suggested: 1/16 of an 8x pixel); a pixel centre is inside a capsule when its squared distance to the segment is `<= (t/2)^2`. Only capsules whose bounding box overlaps the LED are tested.

- [ ] **Step 1: Failing test** `fourteen::tests::close_to_the_preview`: for every `seg14` case, every LED's coverage is within ±4 (of 64) of the fixture; and the hard-edged result differs from the preview's only where the fixture is within 4 of 32.
- [ ] **Step 2–4:** fails, implement, passes.
- [ ] **Step 5:** commit.

### Task 7: Text area

**Files:** Create `crates/ledkit-text/src/style.rs` (shared by text area and ticker), `crates/ledkit-text/src/text_area.rs`.

**Interfaces (produces):**
- `pub enum Across { Left, Centre, Right }`, `pub enum UpDown { Top, Middle, Bottom }` (defaults `Centre`, `Middle`).
- `pub(crate) struct Style { font: Font, letters: Paint, outline: Option<Paint>, behind: Option<Paint>, reverse: bool, left: i32, top: i32, width: u16, height: u16 }` with `pub(crate) fn draw(&self, surface: &mut (impl Surface + ?Sized), now: Time, text: &[u8], text_left: i32, text_top: i32)` (text position relative to the box; draws only LEDs inside both the box and the surface).
- `pub struct TextArea<const LEN: usize = 32>` with `TextArea::new(font: Font) -> TextArea` (default capacity) and `TextArea::<LEN>::with_capacity(font: Font)`; builders `at(left: i32, top: i32, width: u16, height: u16)`, `align(Across, UpDown)`, `letters(impl Into<Paint>)`, `outline(impl Into<Paint>)`, `behind(impl Into<Paint>)`, `reverse(bool)`; `set_text(&mut self, text: &str)`; `text_width(&self) -> u16`; `render(&self, surface: &mut (impl Surface + ?Sized), now: impl Into<Time>)`. A new area with no `at` covers the whole surface it's drawn on.

Per LED, as the preview's `draw`: coverage `a` (reversed: `255 - a`, everywhere in the box), outline amount = max of the 3x3 neighbours' coverage minus `a` (0 if no outline), colour = behind (black if none), lerped toward outline by the outline amount, then toward letters by `a` (`Rgb::lerp`). Paints get the LED's **panel** position and a `Frame` the size of the whole surface.

- [ ] **Step 1: Failing tests** (render onto a `Canvas` over an array, as ledkit-anim's tests do):
  - `exact_pixels`: `"12"` in `X11_5X8` at 1x on a 12x8 panel, centred: the lit LEDs equal the two BDF glyphs at columns 1..6 and 6..11.
  - `alignment`: `Left/Top`, `Right/Bottom`, `Centre/Middle` of `"1"` in a 10x10 box put the glyph's top-left at (0,0), (5,2), (2,1) (5x8 glyph).
  - `clips_to_box`: a 4x4 box at (2,2) never changes an LED outside it (fill the panel with a sentinel colour first).
  - `off_panel_and_empty`: boxes at (-3,-3), (100,100) and a 0x0 box draw without panicking and only touch real LEDs.
  - `overflow_centres`: `"88888"` (25 wide) in an 8-wide centred box shows the middle columns (left offset `floor((8-25)/2) = -9`).
  - `capacity_and_ascii`: `TextArea::<4>` given `"héllo!"` holds `"h?ll"`.
  - `reverse_and_outline`: reversed, the unlit LEDs of the glyph are black and the rest of the box is the letter colour; with an outline, an LED next to a letter but not in it has the outline colour.
  - `panel_coordinates`: an area at (4,0) whose letters are `Rainbow` colours an LED exactly as `Rainbow` colours that panel LED.
- [ ] **Step 2–4:** fails, implement, passes.
- [ ] **Step 5:** commit.

### Task 8: Ticker

**Files:** Create `crates/ledkit-text/src/ticker.rs`.

**Interfaces (produces):** `pub struct Ticker<const LEN: usize = 256>` (messages stored back to back in one buffer, so `LEN` is the total bytes; a second buffer the same size holds a pending list), `Ticker::new(font)`, `Ticker::<LEN>::with_capacity(font)`, the same style builders as `TextArea` except `align` → `up_down(UpDown)`, plus `speed(u16)` (LEDs per second, default 20, the preview's "medium"), `gap(u16)` (blank LEDs of travel after a message, default = box width), `hold(u32)` (ms a short message stays, default 3000); `set_messages(&mut self, messages: &[&str])`; `render(&mut self, surface, now)`.

Timeline of one message of width `M` in a box of width `W`, from its start time `t0`, `d = (now - t0) * speed / 1000` LEDs:
- `M > W`: text left = `W - d` until `d >= W + M`; then blank until `d >= W + M + gap`; then the next message.
- `M <= W`: centred and still until `now - t0 >= hold`; then blank for `gap * 1000 / speed` ms; then the next.
- At the switch, a pending list replaces the current one and starts at its first message; otherwise the next message, wrapping to the first.
- If `now < t0` (time went backwards), `t0 = now`.

- [ ] **Step 1: Failing tests:** on an 8x8 panel with `X11_5X7`, speed 20:
  - `enters_from_the_right`: at t0 the box is blank; at 50 ms the message's first column is at x = 7.
  - `scrolls_off_then_gap_then_next`: `["ABCD", "WXYZ"]` (20 wide each): from `d = 28` to `d = 36` (1400 ms to 1800 ms) the box is blank; at 1850 ms the second message's first column is at x = 7.
  - `short_message_holds`: `["1"]` alone is centred and unchanged from 0 to 2999 ms, blank during the gap, then shown again.
  - `new_list_waits_for_the_gap`: `set_messages` mid-scroll does not change the current message; after the gap the new list's first message enters.
  - `empty_and_backwards_time`: an empty list draws a blank box; rendering at 5000 ms then 1000 ms doesn't panic and restarts the message.
- [ ] **Step 2–4:** fails, implement, passes.
- [ ] **Step 5:** commit.

### Task 9: Examples

**Files:**
- Create: `examples/esp32c6-text/` (copy the shape of `examples/esp32c6-anim/`: `Cargo.toml` with its own `[workspace]`, `rust-toolchain.toml`, `.cargo/config.toml`, `Cargo.lock`, `README.md`, `src/main.rs`)
- Modify: `examples/anim-preview/src/main.rs`, its `Cargo.toml` and `Cargo.lock`
- Modify: `.github/workflows/ci.yml`: an `example-esp32c6-text` job like `example-esp32c6-anim`; `example-anim-preview` also runs `cargo run --locked -- text "hi" --frames 1`

- [ ] **Step 1:** `esp32c6-text`: the owner's 8x8 (same `LAYOUT` as `esp32c6-anim`), a `Ticker` with `X11_5X7`, `Rainbow` letters, three messages (`"ledkit"`, the seconds since boot formatted by the example as `"up 12s"`, refreshed with `set_messages` each loop, `"Hello!"`). Comments for beginners as in `esp32c6-anim`.
- [ ] **Step 2:** `anim-preview`: `cargo run -- text "message" [--size WxH] [--font NAME]` shows a ticker; `cargo run -- clock --size 64x64` shows a date area and a 30-row 7-segment time area with `Plasma` letters and a black outline, using the system clock. Both print average frame time on exit. `--frames N` stops after N frames (for CI).
- [ ] **Step 3:** `cargo build --release --locked` and `cargo clippy --release --locked -- -D warnings` in `examples/esp32c6-text`; `cargo run --locked -- text "hi" --frames 1` in `examples/anim-preview`.
- [ ] **Step 4:** commit, then ask the owner to flash `esp32c6-text` and check it by eye.

### Task 10: Docs and final checks

**Files:** `crates/ledkit-text/README.md` and crate docs (what it is, a desk clock and a ticker example, the face table, sizes, edges, the stencil colours), `AGENTS.md` (crate map row, "Exist so far"), `README.md` (status: text support exists; link to the preview), `docs/superpowers/plans/2026-10-25-ledkit-roadmap.md` ("Text support" section: built; WebAssembly preview still to come), `NOTICE` (X11 fonts now also in `ledkit-text`; Tiny 3x5 and segment faces are original).

- [ ] **Step 1:** write the docs; every public item has an example (`cargo doc -p ledkit-text --no-deps` has no warnings, doc tests pass).
- [ ] **Step 2:** the full CI list: fmt, clippy, `cargo test --workspace --all-features`, both `no_std` builds, `cargo +1.82 check --workspace --all-features`, `python tools/test_fontgen.py`, `python tools/fontgen.py --check`, `python site/text-preview/build.py --check`, `node site/text-preview/reference.mjs --check`.
- [ ] **Step 3:** commit; report to the owner and ask before pushing or opening a PR.
