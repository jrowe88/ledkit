# ledkit Milestone 1: Foundation + `ledkit-color` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the ledkit Cargo workspace (CI, licenses, contributor docs) and deliver a complete, documented, tested `ledkit-color` crate.

**Architecture:** A Cargo workspace with crates under `crates/`. `ledkit-color` is `#![no_std]`, has no heap allocation, and uses integer math for everything that runs per pixel (scaling, blending, HSV, gamma lookup). Floating point appears only in one-off constructors (`Hue::from_degrees`, `Rgb::from_kelvin`) and in `const fn` table generation at compile time. Unit tests live in each module (`#[cfg(test)]`) and run on the host.

**Tech Stack:** Rust stable (MSRV 1.82, for `const fn` floating-point math), edition 2021; `libm` 0.2 (no_std math for Kelvin); optional `rgb` 0.8 interop; GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-25-ledkit-design.md` (see "`ledkit-color`", "Constraints", "API feel").
**Roadmap:** `docs/superpowers/plans/2026-10-25-ledkit-roadmap.md`

## Global Constraints

- License: `MIT OR Apache-2.0` on every crate.
- `#![no_std]` with no heap allocation; tests may use `std` (`#![cfg_attr(not(test), no_std)]`).
- No nightly features; MSRV `rust-version = "1.82"`; edition 2021.
- Must build for `thumbv6m-none-eabi` (Cortex-M0+, no FPU) and `riscv32imc-unknown-none-elf`.
- No code or tables copied from LGPL/EUPL sources (Adafruit_NeoPixel, blinksy). Values ported from MIT sources (Adafruit CircuitPython LED_Animation named colors, FastLED's correction constant) get attribution in `NOTICE` and in the source comment.
- Readable public API: plain-English names, every public item documented (`missing_docs` is denied in CI), with an example on every public type.
- Repo author identity: `Jim Rowe <jim.rowe@gmail.com>` (already set in the repo's local git config).
- Commit messages end with: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

## Review Focus

1. **Fading to black through the gamma table:** values 1–23 map to 0 at gamma 2.6, which is correct, but the table must never *decrease* as input rises (a fade would flicker). The monotonicity test lives in Task 4.
2. **Hue wrap-around:** `from_degrees(360.0)`, `from_degrees(-90.0)` and `from_degrees(720.0 + 60.0)` must equal their wrapped equivalents. A rainbow that jumps at the seam is the most visible bug. Tests in Task 3.
3. **Exact endpoints:** `scale(255)` and `lerp(t = 255)` must return the input and target *exactly*, not one level off, or crossfades never quite finish. Tests in Task 2.
4. **The no-FPU target:** someone on a SAMD21 (Cortex-M0+) expects it to compile. CI builds `thumbv6m-none-eabi` from Task 1, and `libm` must be the only float-math path (no `f32::powf`). Covered by the CI build in Task 1, re-run in each task's verify step.
5. **White extraction edge cases:** pure white `(255, 255, 255)` → `Rgbw (0, 0, 0, 255)`; a saturated color → `w = 0`, unchanged RGB; black → all zero. Tests in Task 2.

---

## File Structure

```
ledkit/
├── Cargo.toml                    # workspace: members, shared package metadata, lints
├── .gitignore
├── LICENSE-MIT, LICENSE-APACHE, NOTICE
├── README.md                     # project intro + crate table (stub; full guide is milestone 8)
├── AGENTS.md, CLAUDE.md          # AI-assistant guide; CLAUDE.md just points to AGENTS.md
├── CONTRIBUTING.md
├── .github/workflows/ci.yml
└── crates/ledkit-color/
    ├── Cargo.toml
    ├── README.md
    └── src/
        ├── lib.rs                # crate docs, module wiring, re-exports
        ├── rgb.rs                # Rgb, Rgbw: scale, lerp, white extraction
        ├── named.rs              # named color constants on Rgb
        ├── hue.rs                # Hue, Hsv, HSV -> RGB
        ├── gamma.rs              # GammaTable (const-generated), GAMMA_2_6, GAMMA_2_2
        ├── temperature.rs        # Rgb::from_kelvin, correction presets, Rgb::corrected
        ├── gradient.rs           # Gradient<N>, Palette<N>
        └── interop.rs            # `rgb` feature: From conversions
```

---

### Task 1: Workspace skeleton, CI, and project docs

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`, `README.md`, `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `.github/workflows/ci.yml`
- Create: `crates/ledkit-color/Cargo.toml`, `crates/ledkit-color/README.md`, `crates/ledkit-color/src/lib.rs`

**Interfaces:**
- Produces: the workspace that every later crate joins via `members = ["crates/*"]`; shared `[workspace.package]` and `[workspace.lints]` that crates inherit with `edition.workspace = true`, `lints.workspace = true`, etc.

- [ ] **Step 1: Install the embedded build targets**

Run: `rustup target add thumbv6m-none-eabi riscv32imc-unknown-none-elf`
Expected: both reported installed (or "up to date").

- [ ] **Step 2: Write the workspace `Cargo.toml`**

`[workspace]` with `resolver = "2"` and `members = ["crates/*"]`.
`[workspace.package]`: `version = "0.1.0"`, `edition = "2021"`, `rust-version = "1.82"`, `license = "MIT OR Apache-2.0"`, `repository = "https://github.com/jrowe88/ledkit"`, `authors = ["Jim Rowe"]`.
`[workspace.lints.rust]`: `missing_docs = "warn"`, `unsafe_code = "forbid"`.
`[workspace.lints.clippy]`: `pedantic = { level = "warn", priority = -1 }`, then allow `module_name_repetitions`, `cast_possible_truncation`, `cast_lossless`, and `must_use_candidate` (they are noisy for color math).

- [ ] **Step 3: Create `crates/ledkit-color`**

`Cargo.toml`: name `ledkit-color`, inherit the workspace fields, `description = "Colors for addressable LEDs: RGB/RGBW, hue, gamma, color temperature, gradients. no_std."`, `keywords = ["led", "neopixel", "color", "no-std", "embedded"]`, `categories = ["embedded", "no-std", "graphics"]`, `readme = "README.md"`, `[lints] workspace = true`.
Dependencies: `libm = "0.2"`; optional `rgb = { version = "0.8", optional = true, default-features = false }`. Feature `rgb = ["dep:rgb"]`.
`src/lib.rs`: `#![cfg_attr(not(test), no_std)]` and a one-paragraph crate doc comment. The full crate docs with examples are written in Task 7.

- [ ] **Step 4: Add licenses, NOTICE and the docs stubs**

- `LICENSE-APACHE`: the standard Apache-2.0 text. `LICENSE-MIT`: MIT text with `Copyright (c) 2026 Jim Rowe`.
- `NOTICE`: an "Attributions" section listing *Adafruit CircuitPython LED_Animation (MIT, Copyright (c) Adafruit Industries): named color values* and *FastLED (MIT, Copyright (c) 2013 FastLED): the TypicalLEDStrip correction value*.
- `README.md`: a one-paragraph pitch from the spec's Intent, a crate table (only `ledkit-color` marked available, the rest "planned"), the license section.
- `AGENTS.md`: what ledkit is; the crate map from the spec; conventions (no_std, no alloc, integer math per pixel, readable names, docs + example on every public item, tests in-module, run `cargo test --workspace` and the CI commands before committing); "never copy code from LGPL/EUPL projects". `CLAUDE.md`: one line, "See AGENTS.md."
- `CONTRIBUTING.md`: how to build and test (the CI commands below), the license statement for contributions, and placeholder headings for "Add an LED type", "Add a backend" and "Add an effect" (each marked "coming in a later milestone").
- `.gitignore`: `/target`, `**/*.rs.bk`, `Cargo.lock` is **committed** (it's a workspace with future binaries/examples).

- [ ] **Step 5: Write `.github/workflows/ci.yml`**

Trigger on push to `main` and on pull requests. Jobs on `ubuntu-latest` using `dtolnay/rust-toolchain@stable` (with `components: rustfmt, clippy` and `targets: thumbv6m-none-eabi,riscv32imc-unknown-none-elf`) and `Swatinem/rust-cache@v2`:
- `fmt`: `cargo fmt --all -- --check`
- `clippy`: `cargo clippy --workspace --all-features -- -D warnings`
- `test`: `cargo test --workspace --all-features`
- `no-std`: `cargo build -p ledkit-color --target thumbv6m-none-eabi` and `cargo build -p ledkit-color --target riscv32imc-unknown-none-elf`
- `msrv`: on toolchain `1.82`, run `cargo check --workspace`

- [ ] **Step 6: Verify everything builds**

Run, in order:
`cargo build --workspace` → succeeds
`cargo build -p ledkit-color --target thumbv6m-none-eabi` → succeeds
`cargo build -p ledkit-color --target riscv32imc-unknown-none-elf` → succeeds
`cargo clippy --workspace --all-features -- -D warnings` → no warnings
`cargo fmt --all -- --check` → no output

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Set up ledkit workspace, CI, licenses and contributor docs"
```

---

### Task 2: `Rgb`, `Rgbw`, scaling, blending, white extraction, named colors

**Files:**
- Create: `crates/ledkit-color/src/rgb.rs`, `crates/ledkit-color/src/named.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (add modules; `pub use rgb::{Rgb, Rgbw};` and `pub type Color = Rgb;`)

**Interfaces:**
- Produces:
  - `pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }`: `Copy, Clone, Debug, PartialEq, Eq, Hash, Default`
  - `pub const fn Rgb::new(r: u8, g: u8, b: u8) -> Rgb`
  - `pub const fn Rgb::scale(self, factor: u8) -> Rgb`: each channel is `(c * factor + 127) / 255` (255 = unchanged, 0 = black)
  - `pub const fn Rgb::lerp(self, target: Rgb, t: u8) -> Rgb`: each channel is `(a * (255 - t) + b * t + 127) / 255`
  - `pub struct Rgbw { pub r: u8, pub g: u8, pub b: u8, pub w: u8 }` with the same derives, `const fn new`, `const fn scale`, `const fn lerp` (same formulas, four channels)
  - `impl From<Rgb> for Rgbw` (sets `w = 0`)
  - `pub const fn Rgbw::from_rgb_extracting_white(rgb: Rgb) -> Rgbw`: `w = min(r, g, b)`, and each RGB channel minus `w`
  - Named constants on `Rgb` (in `named.rs`, as `impl Rgb { pub const ... }`): `BLACK (0,0,0)`, `WHITE (255,255,255)`, `RED (255,0,0)`, `GREEN (0,255,0)`, `BLUE (0,0,255)`, `YELLOW (255,150,0)`, `ORANGE (255,40,0)`, `CYAN (0,255,255)`, `TEAL (0,255,120)`, `PURPLE (180,0,255)`, `MAGENTA (255,0,20)`, `PINK (242,90,255)`, `GOLD (255,222,30)`, `AQUA (50,255,255)`, `JADE (0,255,40)`, `AMBER (255,100,0)`, `OLD_LACE (253,245,230)`, `WARM_WHITE (255,167,87)`, `COOL_WHITE (255,254,250)`. Put a comment above the Adafruit-derived values: "Values from Adafruit CircuitPython LED_Animation (MIT); see NOTICE." `WARM_WHITE` and `COOL_WHITE` are 2700 K and 6500 K from Task 5's formula.
  - `pub type Color = Rgb;`, so hobbyist code can write `Color::BLUE`.

- [ ] **Step 1: Write the failing tests** (in `rgb.rs` under `#[cfg(test)] mod tests`)

```rust
#[test] fn scale_255_is_identity() { let c = Rgb::new(10, 128, 255); assert_eq!(c.scale(255), c); }
#[test] fn scale_0_is_black() { assert_eq!(Rgb::new(10, 128, 255).scale(0), Rgb::BLACK); }
#[test] fn scale_half() { assert_eq!(Rgb::new(255, 100, 0).scale(128), Rgb::new(128, 50, 0)); }
#[test] fn lerp_endpoints_exact() {
    let (a, b) = (Rgb::new(10, 20, 30), Rgb::new(200, 100, 0));
    assert_eq!(a.lerp(b, 0), a);
    assert_eq!(a.lerp(b, 255), b);
}
#[test] fn lerp_midpoint() { assert_eq!(Rgb::BLACK.lerp(Rgb::WHITE, 128), Rgb::new(128, 128, 128)); }
#[test] fn extract_white_from_pure_white() {
    assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::WHITE), Rgbw::new(0, 0, 0, 255));
}
#[test] fn extract_white_from_saturated_color_is_zero() {
    assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::new(255, 0, 40)), Rgbw::new(255, 0, 40, 0));
}
#[test] fn extract_white_from_pastel() {
    assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::new(255, 200, 100)), Rgbw::new(155, 100, 0, 100));
}
#[test] fn extract_white_from_black() { assert_eq!(Rgbw::from_rgb_extracting_white(Rgb::BLACK), Rgbw::default()); }
#[test] fn rgb_into_rgbw_has_no_white() { assert_eq!(Rgbw::from(Rgb::new(1, 2, 3)), Rgbw::new(1, 2, 3, 0)); }
#[test] fn rgbw_scale_and_lerp_endpoints() {
    let w = Rgbw::new(10, 20, 30, 40);
    assert_eq!(w.scale(255), w);
    assert_eq!(w.lerp(Rgbw::new(0, 0, 0, 255), 255), Rgbw::new(0, 0, 0, 255));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color`
Expected: FAIL (compile errors: `Rgb` and `Rgbw` not found).

- [ ] **Step 3: Implement `rgb.rs` and `named.rs`** as specified in Interfaces. Do the arithmetic in `u16`. Each type gets a doc comment with a runnable example (for example `let dim_red = Rgb::RED.scale(64);`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p ledkit-color`
Expected: all tests and doctests PASS.

- [ ] **Step 5: Verify the lints and the no_std build**

Run: `cargo clippy -p ledkit-color --all-features -- -D warnings` and `cargo build -p ledkit-color --target thumbv6m-none-eabi`
Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add crates/ledkit-color
git commit -m "ledkit-color: add Rgb/Rgbw with scaling, blending, white extraction and named colors"
```

---

### Task 3: `Hue` and `Hsv`

**Files:**
- Create: `crates/ledkit-color/src/hue.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (`pub use hue::{Hue, Hsv};`)

**Interfaces:**
- Consumes: `Rgb` (Task 2).
- Produces:
  - `pub struct Hue(pub u16)`: a full turn of the color wheel is 65536 steps; `Copy, Clone, Debug, PartialEq, Eq, Hash, Default`
  - Constants: `Hue::RED = Hue(0)`, `YELLOW = Hue(10923)`, `GREEN = Hue(21845)`, `CYAN = Hue(32768)`, `BLUE = Hue(43691)`, `MAGENTA = Hue(54613)`
  - `pub fn Hue::from_degrees(degrees: f32) -> Hue`: wraps any value, including negative values and values ≥ 360. Compute `wrapped = libm::fmodf(degrees, 360.0)`, add `360.0` if it's negative, then `((wrapped / 360.0) * 65536.0 + 0.5) as u32 & 0xFFFF`. (`f32::rem_euclid` is `std`-only, so it isn't available in this `no_std` crate.)
  - `pub fn Hue::from_fraction(fraction: f32) -> Hue`: `0.0..1.0` is a full turn, wrapping; same approach with `libm::fmodf(fraction, 1.0)`
  - `pub const fn Hue::shifted(self, steps: u16) -> Hue`: a wrapping add, used to rotate rainbows
  - `pub struct Hsv { pub hue: Hue, pub saturation: u8, pub value: u8 }` with `pub const fn new(hue: Hue, saturation: u8, value: u8)`
  - `impl From<Hsv> for Rgb` and `pub const fn Rgb::from_hue(hue: Hue) -> Rgb` (full saturation and value)

The HSV → RGB algorithm (integer, so it's fast on chips without an FPU, and it lands exactly on the primary colors at 60° steps):

```text
scaled  = hue as u32 * 6
sector  = scaled >> 16                 // 0..=5
rising  = ((scaled & 0xFFFF) >> 8)     // 0..=255
falling = 255 - rising
full-color (r, g, b) by sector:
  0: (255, rising, 0)   1: (falling, 255, 0)   2: (0, 255, rising)
  3: (0, falling, 255)  4: (rising, 0, 255)    5: (255, 0, falling)
saturation: c = 255 - ((255 - c) * s + 127) / 255
value:      c = (c * v + 127) / 255
```

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn primaries_are_exact() {
    for (deg, rgb) in [(0.0, (255,0,0)), (60.0, (255,255,0)), (120.0, (0,255,0)),
                       (180.0, (0,255,255)), (240.0, (0,0,255)), (300.0, (255,0,255))] {
        let c = Rgb::from_hue(Hue::from_degrees(deg));
        assert_eq!((c.r, c.g, c.b), rgb, "at {deg} degrees");
    }
}
#[test] fn degrees_wrap_around() {
    assert_eq!(Hue::from_degrees(360.0), Hue::RED);
    assert_eq!(Hue::from_degrees(-60.0), Hue::from_degrees(300.0));
    assert_eq!(Hue::from_degrees(780.0), Hue::from_degrees(60.0));
}
#[test] fn fraction_wraps() { assert_eq!(Hue::from_fraction(1.25), Hue::from_fraction(0.25)); }
#[test] fn shifted_wraps() { assert_eq!(Hue(65000).shifted(1000), Hue(464)); }
#[test] fn zero_saturation_is_white() { assert_eq!(Rgb::from(Hsv::new(Hue::BLUE, 0, 255)), Rgb::WHITE); }
#[test] fn zero_value_is_black() { assert_eq!(Rgb::from(Hsv::new(Hue::GREEN, 255, 0)), Rgb::BLACK); }
#[test] fn half_value_red() { assert_eq!(Rgb::from(Hsv::new(Hue::RED, 255, 128)), Rgb::new(128, 0, 0)); }
#[test] fn hue_constants_match_degrees() {
    assert_eq!(Hue::from_degrees(120.0), Hue::GREEN);
    assert_eq!(Hue::from_degrees(240.0), Hue::BLUE);
}
#[test] fn rainbow_has_no_jumps() {
    // Neighboring hues differ by at most 1 level per channel: no visible seams, including at the wrap.
    let mut prev = Rgb::from_hue(Hue(65535 - 255));
    for h in (0..=65535u32).step_by(256) {
        let c = Rgb::from_hue(Hue(h as u16));
        for (a, b) in [(prev.r, c.r), (prev.g, c.g), (prev.b, c.b)] { assert!(a.abs_diff(b) <= 7); }
        prev = c;
    }
}
```

(`rainbow_has_no_jumps` steps 256 hue units, about 1.4°. One sector spans about 10923 units, which is 255 levels, so each step changes a channel by at most about 6 levels. The bound of 7 catches real seams without flaking.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color hue`
Expected: FAIL (compile errors: `Hue` not found).

- [ ] **Step 3: Implement `hue.rs`** with the algorithm above and doc examples (for example `let orange = Rgb::from_hue(Hue::from_degrees(30.0));`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p ledkit-color`
Expected: PASS.

- [ ] **Step 5: Verify the lints and the no_std builds**

Run: `cargo clippy -p ledkit-color --all-features -- -D warnings`, then `cargo build -p ledkit-color --target thumbv6m-none-eabi`, then `cargo build -p ledkit-color --target riscv32imc-unknown-none-elf`
Expected: all clean.

- [ ] **Step 6: Commit**

```bash
git add crates/ledkit-color
git commit -m "ledkit-color: add Hue and integer HSV to RGB conversion"
```

---

### Task 4: Compile-time gamma tables

**Files:**
- Create: `crates/ledkit-color/src/gamma.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (`pub use gamma::{GammaTable, GAMMA_2_2, GAMMA_2_6};`)

**Interfaces:**
- Consumes: `Rgb`, `Rgbw` (Task 2).
- Produces:
  - `pub struct GammaTable([u8; 256])`
  - `pub const fn GammaTable::new(numerator: u32, denominator: u32) -> GammaTable`: gamma = numerator / denominator, computed at compile time
  - `pub const GammaTable::LINEAR: GammaTable` (the identity)
  - `pub const GAMMA_2_6: GammaTable = GammaTable::new(13, 5);` (the default; the same exponent Adafruit and FastLED use)
  - `pub const GAMMA_2_2: GammaTable = GammaTable::new(11, 5);`
  - `pub const fn GammaTable::apply(&self, level: u8) -> u8`
  - `pub const fn GammaTable::apply_rgb(&self, color: Rgb) -> Rgb` and `pub const fn GammaTable::apply_rgbw(&self, color: Rgbw) -> Rgbw`

Algorithm for `new`. It computes `round(255 * (x/255)^(n/d))` with no roots, only multiplication, so it works in `const fn`:

```text
for each input x in 0..=255 (keep y from the previous x; outputs never decrease):
  target = (x / 255.0)^n                          // f64, multiply in a loop
  while y < 255 and ((y + 0.5) / 255.0)^d <= target:  y += 1
  table[x] = y
```

(`((y + ½)/255)^d ≤ (x/255)^n` means the true value is at least `y + ½`, so the rounded result is at least `y + 1`. Starting each search from the previous `y` keeps the table monotonic and the loop at about 512 steps.)

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn gamma_2_6_known_values() {
    for (x, y) in [(0, 0), (1, 0), (23, 0), (24, 1), (32, 1), (64, 7), (128, 42), (200, 136), (254, 252), (255, 255)] {
        assert_eq!(GAMMA_2_6.apply(x), y, "input {x}");
    }
}
#[test] fn gamma_2_2_midpoint() { assert_eq!(GAMMA_2_2.apply(128), 56); }
#[test] fn linear_is_identity() { for x in 0..=255u8 { assert_eq!(GammaTable::LINEAR.apply(x), x); } }
#[test] fn tables_never_decrease() {
    for t in [GAMMA_2_6, GAMMA_2_2] { for x in 1..=255u8 { assert!(t.apply(x) >= t.apply(x - 1)); } }
}
#[test] fn matches_float_reference_exactly() {
    for x in 0..=255u8 {
        let expected = (255.0 * (f64::from(x) / 255.0).powf(2.6)).round() as u8;
        assert_eq!(GAMMA_2_6.apply(x), expected, "input {x}");
    }
}
#[test] fn apply_rgb_per_channel() { assert_eq!(GAMMA_2_6.apply_rgb(Rgb::new(128, 255, 0)), Rgb::new(42, 255, 0)); }
#[test] fn table_is_built_at_compile_time() { const T: GammaTable = GammaTable::new(13, 5); assert_eq!(T.apply(128), 42); }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color gamma`
Expected: FAIL (`GammaTable` not found).

- [ ] **Step 3: Implement `gamma.rs`**, with doc comments explaining in plain words why gamma correction matters ("your eye sees brightness non-linearly, so a fade looks wrong without it").

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p ledkit-color`
Expected: PASS. If `matches_float_reference_exactly` fails at a single input because of a float tie, fix the comparison (`<` vs `<=`), not the test.

- [ ] **Step 5: Verify the lints and the no_std build**

Run: `cargo clippy -p ledkit-color --all-features -- -D warnings` and `cargo build -p ledkit-color --target thumbv6m-none-eabi`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/ledkit-color
git commit -m "ledkit-color: add compile-time gamma tables"
```

---

### Task 5: Color temperature and white-balance correction

**Files:**
- Create: `crates/ledkit-color/src/temperature.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (`mod temperature; pub use temperature::correction;`, so the path is `ledkit_color::correction::TYPICAL_LED_STRIP`)

**Interfaces:**
- Consumes: `Rgb` (Task 2).
- Produces:
  - `pub fn Rgb::from_kelvin(kelvin: u16) -> Rgb`: the approximation of blackbody color (Tanner Helland's curve fit), clamped to 1000–40000 K, using `libm::powf` and `libm::logf`. With `t = kelvin / 100`:
    - red: `255` if `t <= 66`, else `329.698727446 * (t - 60)^-0.1332047592`
    - green: `99.4708025861 * ln(t) - 161.1195681661` if `t <= 66`, else `288.1221695283 * (t - 60)^-0.0755148492`
    - blue: `255` if `t >= 66`; `0` if `t <= 19`; else `138.5177312231 * ln(t - 10) - 305.0447927307`
    - each channel clamped to 0–255 and rounded
  - `pub const fn Rgb::corrected(self, correction: Rgb) -> Rgb`: per-channel scale (`r` by `correction.r`, and so on), using `scale`'s formula
  - `pub mod correction` with `pub const NONE: Rgb = Rgb::WHITE;` and `pub const TYPICAL_LED_STRIP: Rgb = Rgb::new(255, 176, 240);` (comment: "From FastLED (MIT); see NOTICE. Compensates for the strong green of most WS2812-type strips.")

- [ ] **Step 1: Write the failing tests**

```rust
fn close(a: Rgb, b: (u8, u8, u8)) -> bool { a.r.abs_diff(b.0) <= 1 && a.g.abs_diff(b.1) <= 1 && a.b.abs_diff(b.2) <= 1 }
#[test] fn known_temperatures() {
    for (k, rgb) in [(1900, (255,132,0)), (2700, (255,167,87)), (4000, (255,206,166)), (6500, (255,254,250)), (10000, (202,218,255))] {
        let c = Rgb::from_kelvin(k);
        assert!(close(c, rgb), "{k} K gave {c:?}");
    }
}
#[test] fn out_of_range_is_clamped() {
    assert_eq!(Rgb::from_kelvin(0), Rgb::from_kelvin(1000));
    assert_eq!(Rgb::from_kelvin(u16::MAX), Rgb::from_kelvin(40000));
}
#[test] fn named_whites_match_formula() {
    assert!(close(Rgb::from_kelvin(2700), (Rgb::WARM_WHITE.r, Rgb::WARM_WHITE.g, Rgb::WARM_WHITE.b)));
    assert!(close(Rgb::from_kelvin(6500), (Rgb::COOL_WHITE.r, Rgb::COOL_WHITE.g, Rgb::COOL_WHITE.b)));
}
#[test] fn no_correction_is_identity() { let c = Rgb::new(9, 99, 199); assert_eq!(c.corrected(correction::NONE), c); }
#[test] fn typical_strip_correction() {
    assert_eq!(Rgb::WHITE.corrected(correction::TYPICAL_LED_STRIP), Rgb::new(255, 176, 240));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color temperature`
Expected: FAIL (`from_kelvin` not found).

- [ ] **Step 3: Implement `temperature.rs`.** Doc examples: `Rgb::from_kelvin(2700) // warm, like an incandescent bulb`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p ledkit-color`
Expected: PASS.

- [ ] **Step 5: Verify the lints and the no-FPU build**

Run: `cargo clippy -p ledkit-color --all-features -- -D warnings` and `cargo build -p ledkit-color --target thumbv6m-none-eabi`
Expected: clean. This confirms `libm` is the only float-math path.

- [ ] **Step 6: Commit**

```bash
git add crates/ledkit-color
git commit -m "ledkit-color: add color temperature and white-balance correction"
```

---

### Task 6: Gradients and palettes

**Files:**
- Create: `crates/ledkit-color/src/gradient.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (`pub use gradient::{Gradient, Palette};`)

**Interfaces:**
- Consumes: `Rgb::lerp` (Task 2), named colors (Task 2).
- Produces:
  - `pub struct Gradient<const N: usize> { stops: [(u8, Rgb); N] }`: color stops at positions 0–255
  - `pub const fn Gradient::new(stops: [(u8, Rgb); N]) -> Gradient<N>`: panics (in const context, a compile error) if `N == 0` or the positions aren't in ascending order
  - `pub const fn Gradient::color_at(&self, position: u8) -> Rgb`: before the first stop → the first color; after the last → the last color; between two stops at `p0 < p1` → `lerp(c0, c1, ((position - p0) * 255 / (p1 - p0)) as u8)`; when equal positions are next to each other, the later stop wins
  - `pub struct Palette<const N: usize> { colors: [Rgb; N] }`: evenly spaced colors around a *circle* (it wraps)
  - `pub const fn Palette::new(colors: [Rgb; N]) -> Palette<N>`: panics if `N == 0`
  - `pub const fn Palette::color_at(&self, position: u8) -> Rgb`: `scaled = position as u32 * N`, `i = scaled >> 8`, `t = (scaled & 0xFF) as u8`, `lerp(colors[i], colors[(i + 1) % N], t)`
  - `pub const Palette::RAINBOW: Palette<6>` = `[RED, ORANGE, YELLOW, GREEN, BLUE, PURPLE]`

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn gradient_endpoints_and_clamping() {
    let g = Gradient::new([(64, Rgb::RED), (192, Rgb::BLUE)]);
    assert_eq!(g.color_at(0), Rgb::RED);
    assert_eq!(g.color_at(64), Rgb::RED);
    assert_eq!(g.color_at(192), Rgb::BLUE);
    assert_eq!(g.color_at(255), Rgb::BLUE);
}
#[test] fn gradient_midpoint() {
    let g = Gradient::new([(0, Rgb::BLACK), (255, Rgb::WHITE)]);
    assert_eq!(g.color_at(128), Rgb::new(128, 128, 128));
}
#[test] fn gradient_three_stops() {
    let g = Gradient::new([(0, Rgb::RED), (128, Rgb::GREEN), (255, Rgb::BLUE)]);
    assert_eq!(g.color_at(128), Rgb::GREEN);
}
#[test] fn single_stop_gradient_is_solid() {
    let g = Gradient::new([(100, Rgb::GOLD)]);
    assert_eq!(g.color_at(0), Rgb::GOLD);
    assert_eq!(g.color_at(255), Rgb::GOLD);
}
#[test] fn palette_hits_each_color() {
    let p = Palette::new([Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE]);
    assert_eq!(p.color_at(0), Rgb::RED);
    assert_eq!(p.color_at(64), Rgb::GREEN);
    assert_eq!(p.color_at(128), Rgb::BLUE);
    assert_eq!(p.color_at(192), Rgb::WHITE);
}
#[test] fn palette_wraps_back_toward_first_color() {
    let p = Palette::new([Rgb::RED, Rgb::BLUE]);
    let near_end = p.color_at(255);
    assert!(near_end.r > 200 && near_end.b < 10, "should be almost back to red: {near_end:?}");
}
#[test] fn rainbow_palette_starts_red() { assert_eq!(Palette::RAINBOW.color_at(0), Rgb::RED); }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color gradient`
Expected: FAIL (`Gradient` not found).

- [ ] **Step 3: Implement `gradient.rs`**, with doc examples (a sunset gradient; a palette cycled along a strip by position).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p ledkit-color`
Expected: PASS.

- [ ] **Step 5: Verify the lints and the no_std build**

Run: `cargo clippy -p ledkit-color --all-features -- -D warnings` and `cargo build -p ledkit-color --target thumbv6m-none-eabi`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/ledkit-color
git commit -m "ledkit-color: add gradients and palettes"
```

---

### Task 7: `rgb` crate interop, crate docs, and publish readiness

**Files:**
- Create: `crates/ledkit-color/src/interop.rs`
- Modify: `crates/ledkit-color/src/lib.rs` (`#[cfg(feature = "rgb")] mod interop;`, plus the full crate docs)
- Modify: `crates/ledkit-color/README.md`, `README.md`

**Interfaces:**
- Consumes: `Rgb`, `Rgbw`.
- Produces (feature `rgb`):
  - `impl From<rgb::RGB8> for Rgb` and `impl From<Rgb> for rgb::RGB8`. This is what makes ledkit colors work directly with `smart-leds-trait` drivers, which use `rgb::RGB8`.
  - `impl From<rgb::RGBA8> for Rgbw` (alpha → white) and the reverse. Document that `smart-leds-trait` has its own RGBW type, and that interop with it lands in `ledkit-core` (milestone 2).

- [ ] **Step 1: Write the failing tests** (in `interop.rs`, `#[cfg(test)]`)

```rust
#[test] fn round_trips_rgb8() {
    let c = Rgb::new(1, 2, 3);
    let theirs: rgb::RGB8 = c.into();
    assert_eq!((theirs.r, theirs.g, theirs.b), (1, 2, 3));
    assert_eq!(Rgb::from(theirs), c);
}
#[test] fn rgba8_alpha_is_white() {
    assert_eq!(Rgbw::from(rgb::RGBA8::new(1, 2, 3, 4)), Rgbw::new(1, 2, 3, 4));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p ledkit-color --features rgb`
Expected: FAIL (missing `From` impls).

- [ ] **Step 3: Implement `interop.rs`**

- [ ] **Step 4: Write the crate documentation**

`lib.rs` crate docs and `crates/ledkit-color/README.md` get a "Quick tour" example that compiles as a doctest. It should show, in this order: a named color, dimming with `scale`, a hue, a gamma-corrected color, a warm white from Kelvin, and a rainbow palette sampled along a 30-LED strip as an array. Also add a short "Why integer math?" note (fast on chips without a floating-point unit) and the feature list (`rgb`). In the root `README.md`, mark `ledkit-color` as "available (0.1, unreleased)".

- [ ] **Step 5: Run the full CI command set locally**

Run:
`cargo fmt --all -- --check`
`cargo clippy --workspace --all-features -- -D warnings`
`cargo test --workspace --all-features`
`cargo build -p ledkit-color --target thumbv6m-none-eabi`
`cargo build -p ledkit-color --target riscv32imc-unknown-none-elf`
`cargo doc -p ledkit-color --all-features --no-deps`
Expected: all succeed, no warnings.

- [ ] **Step 6: Check it's ready to publish (don't publish)**

Run: `cargo package -p ledkit-color --list` (the expected file list: `Cargo.toml`, `README.md`, the `src/*.rs` files and the license files; copy the root license files into the crate folder if they're missing), then `cargo publish -p ledkit-color --dry-run`.
Expected: the dry run succeeds. Actual publishing is milestone 9.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "ledkit-color: add rgb interop, crate docs and publish readiness"
```
