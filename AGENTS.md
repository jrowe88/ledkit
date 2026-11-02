# AGENTS.md

## What ledkit is

An Adafruit-quality family of Rust crates for addressable LEDs (strips, rings
and matrices), aimed at hobbyists with roughly Arduino-level experience. It
should "just work": low friction, low research, learn by doing. It works
alongside the existing ecosystem (`smart-leds`, per-chip drivers) rather than
replacing it. The design lives in `docs/superpowers/specs/`.

## Crate map

| Crate | Purpose | Depends on |
|---|---|---|
| `ledkit` | The one crate hobbyists add; re-exports with a `prelude`; features pick the backend | all below (optional) |
| `ledkit-color` | Color types and utilities | nothing (optional `rgb`) |
| `ledkit-core` | Chip catalogue, color orders, encoders, `Strip`, output traits | `ledkit-color` |
| `ledkit-layout` | Segments, matrices, coordinate mapping, `embedded-graphics` | `ledkit-core` |
| `ledkit-anim` | `Animation` trait, effects, combinators | `ledkit-color`, `ledkit-layout`, `ledkit-core` |
| `ledkit-spi` | Backend over `embedded-hal` 1.0 SPI | `ledkit-core` |
| `ledkit-esp` | ESP32 RMT backend (esp-hal, `no_std`) | `ledkit-core`, `esp-hal` |
| `ledkit-esp-idf` | ESP32 RMT backend (esp-idf-hal, `std`) | `ledkit-core`, `esp-idf-hal` |
| `ledkit-rp` | RP2040/RP2350 PIO backend | `ledkit-core`, an RP HAL |
| `ledkit-sim` | Desktop simulator (`std`) | `ledkit-anim` |

Exist so far: `ledkit-color`, `ledkit-core`, `ledkit-layout`, `ledkit-anim`, `ledkit-spi`, `ledkit-uart`, `ledkit-esp` and `ledkit-esp-idf`. The rest are planned.

`ledkit-esp` and `ledkit-esp-idf` live outside the root workspace (they only build for an ESP32 target, with their own toolchains), so `cargo test --workspace` doesn't touch them. They are built and linted through their examples (`examples/esp32c6-rmt`, `examples/esp32c6-esp-idf`, `examples/esp32c6-matrix`, and the Xtensa `examples/esp32s3-*`, which use the `esp` toolchain from `espup`) and in CI. The ESP-IDF examples set a short `target-dir` (`C:/t/ledkit`, `C:/t/ledkit-s3`) in their `.cargo/config.toml`, because `esp-idf-sys` rejects output paths over 88 characters on Windows; on macOS/Linux (and in CI) set `CARGO_TARGET_DIR=target`. Hardware status is recorded in each crate's README.

Writing a new backend? Follow the "Add a backend" guide in `CONTRIBUTING.md`; `ledkit-spi` is the model for a bus backend, and `ledkit-esp` / `ledkit-esp-idf` for a pulse (RMT) backend that times the reset gap itself.

## ledkit-anim modules

- `animation`: the `Animation` trait (`color_at(Position, &Frame) -> Rgb`, plus `render`), `Position`, `Frame`, `Direction`. `surface`: `Surface`, implemented for `Strip`, `Canvas`, `SegmentMut`, arrays and slices. `time`: `Time` (milliseconds, `u32`).
- `colors`: `Colors` (one color, two to blend, or a `&'static dyn ColorMap`). The `ColorMap` trait and the named `palettes` live in `ledkit-color`.
- `effects/`: one file per effect, each a `Copy` struct with `const fn new()` and `const fn` setters; `geometry` (distance and angle from the center) is shared. `effect`: the `Effect` enum, by name, with aliases.
- `combine/`: `AnimationExt` (modifiers, `layer`, `then`, `crossfade_to`), `Blend`, `Transition` / `Style` / `Easing` / `Edge`, `Playlist` / `Entry`, `Segments`.
- `math`: public integer helpers (`sine`, `triangle`, `phase`, `atan2`, `isqrt`, `hash`, `noise`, ...).

Rule: an animation is a pure function of position and time. It keeps no state between frames, so combinators need no buffers; an effect that needs memory of past frames (Game of Life, Fire2012's heat) doesn't fit this trait. Integer math per pixel.

## ledkit-core modules

- `chips` / `chip`: the `Chip` trait and the catalogue (`Ws2812b`, `Apa102`, ...). Chips are zero-sized marker types carrying pixel type, color order, protocol/timing and power model.
- `pixel`: `Pixel`, `Channel`, `ChannelOrder` (wire order).
- `timing`: `Phase`, `ClocklessTiming`, `ClockedFormat`, `Protocol`, `PowerModel`.
- `brightness`, `pipeline`, `power`: brightness (perceptual by default), color gamma (opt-in), correction, dithering and the power limit, applied in `show`.
- `strip`: `Strip<C, N, O>`, the pixel buffer.
- `output`: the `Output` / `OutputAsync` traits.
- `wire`: pixels to bytes in chip order (`clockless_bytes`, `clocked_frame`).
- `encode`: timing to peripheral data: `SpiEncoding` (SPI), and for pulse peripherals `PulseTicks`, `BitPulse`, `ResetGap` and the RMT word helpers (`rmt_bit_word`, `fill_rmt_words`, ...).
- `testing`: `MemoryOutput`, for host tests. `interop`: `smart-leds` feature.

Rule: a backend implements `Output` (and/or `OutputAsync`) using the `wire` and `encode` helpers. It never re-implements color order, timing math or the SPI bit patterns, and never applies brightness or gamma itself (`Strip` already did).

## Conventions

- `no_std` and no `alloc` in the library crates (only `ledkit-esp-idf` and `ledkit-sim` use `std`).
- Integer math per pixel; floating point only where it is computed once (for example tables).
- Readable names over terse ones. This library is for beginners.
- Every public item has docs and an example.
- Tests live in the module they test (`#[cfg(test)] mod tests`).
- Before committing run `cargo test --workspace --all-features` and the CI commands:
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`,
  and the `no_std` builds for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf`.
- Never copy code from LGPL/EUPL projects. MIT/Apache sources may be used with attribution in `NOTICE`.
