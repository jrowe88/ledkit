# AGENTS.md

## What ledkit is

An Adafruit-quality family of Rust crates for addressable LEDs (strips, rings
and matrices), aimed at hobbyists with roughly Arduino-level experience. It
should "just work": low friction, low research, learn by doing. It works
alongside the existing ecosystem (`smart-leds`, per-chip drivers) rather than
replacing it. Start with `docs/roadmap.md` (status and open work) and `docs/specs/` (design).

## Crate map

| Crate | Purpose | Depends on |
|---|---|---|
| `ledkit` | The one crate hobbyists add; re-exports with a `prelude`; features pick the backend | all below (optional) |
| `ledkit-color` | Color types and utilities | nothing (optional `rgb`) |
| `ledkit-core` | Chip catalogue, color orders, encoders, `Strip`, output traits | `ledkit-color` |
| `ledkit-layout` | Segments, matrices, coordinate mapping, `embedded-graphics` | `ledkit-core` |
| `ledkit-anim` | `Animation` trait, effects, combinators | `ledkit-color`, `ledkit-layout`, `ledkit-core` |
| `ledkit-spi` | Backend over `embedded-hal` 1.0 SPI | `ledkit-core` |
| `ledkit-uart` | Backend over a serial port (`embedded-io`, inverted 8N1, 2 LED bits per byte) | `ledkit-core` |
| `ledkit-esp` | ESP32 RMT backend (esp-hal, `no_std`) | `ledkit-core`, `esp-hal` |
| `ledkit-esp-idf` | ESP32 RMT backend (esp-idf-hal, `std`) | `ledkit-core`, `esp-idf-hal` |
| `ledkit-rp` | RP2040/RP2350 PIO backend (`no_std`; processor-fed and DMA-fed, blocking and async) | `ledkit-core`, `rp2040-hal` or `rp235x-hal` |
| `ledkit-nrf` | nRF52840 PWM + EasyDMA backend (`no_std`, blocking + async) | `ledkit-core`, `embassy-nrf` |
| `ledkit-sim` | Desktop simulator (`std`) | `ledkit-anim` |

Exist so far: `ledkit-color`, `ledkit-core`, `ledkit-layout`, `ledkit-anim`, `ledkit-spi`, `ledkit-uart`, `ledkit-esp`, `ledkit-esp-idf`, `ledkit-rp` and `ledkit-nrf`. The rest are planned.

`ledkit-esp`, `ledkit-esp-idf`, `ledkit-rp` and `ledkit-nrf` live outside the root workspace (they only build for their chips' targets), so `cargo test --workspace` doesn't touch them. They are built and linted through their examples (`examples/esp32c6-rmt`, `examples/esp32c6-esp-idf`, `examples/esp32c6-matrix`, the Xtensa `examples/esp32s3-*`, which use the `esp` toolchain from `espup`, `examples/rp2040-pio`, `examples/rp2350-pio`, `examples/nrf52840-pwm`) and in CI. `ledkit-rp` picks its HAL with a feature, `rp2040` or `rp235x` (exactly one); the RP2350 builds for both `thumbv8m.main-none-eabihf` and `riscv32imac-unknown-none-elf`. Its testable logic lives in `ledkit-core` (`encode::equal_period`, `encode::pio_timing`, `encode::pio`). Inside a `.claude/worktrees/` checkout, cargo finds the main checkout's workspace above, so build `ledkit-rp` and `ledkit-nrf` through their examples or a small wrapper project with an empty `[workspace]`. The ESP-IDF examples set a short `target-dir` (`C:/t/ledkit`, `C:/t/ledkit-s3`) in their `.cargo/config.toml`, because `esp-idf-sys` rejects output paths over 88 characters on Windows; on macOS/Linux (and in CI) set `CARGO_TARGET_DIR=target`. Hardware status is recorded in each crate's README.

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
- `encode`: timing to peripheral data: `SpiEncoding` (SPI), `UartEncoding` (serial), and for pulse peripherals `PulseTicks`, `BitPulse`, `ResetGap`, the RMT word helpers (`rmt_bit_word`, `fill_rmt_words`, ...), the equal-period search shared by PIO and PWM (`EqualPeriodTicks`, `ClocklessTiming::equal_period_ticks`), the PIO step timing with a whole clock divider (`PioTiming`, `ClocklessTiming::pio_timing`) and FIFO word packing (`pio_words`, `fill_pio_words`), and the nRF52 PWM duty-word helpers (`pwm_bit_word`, `fill_pwm_words`, ...).
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

## Docs: what lives where

- `docs/roadmap.md`: the single status page. Milestone status, decisions worth
  remembering, and the open follow-ups. Update it in the same PR as the work.
- `docs/specs/<crate>.md`: design decisions, one file per crate or big feature. Keep them
  when the work ships; edit them if the design changes.
- `docs/hardware/`: `test-plan.md` (the T1–T11 checks and per-board sheets),
  `findings.md` (board quirks and bugs real hardware found: the *why*), and
  `power-measurements.md` (bench data).
- Hardware pass/fail: each crate README's "Tested on hardware" table (mirrored in its
  `lib.rs` docs) and each example README's status line.
- Implementation plans are working documents. Keep them in `docs/superpowers/` (git-ignored)
  and don't commit them: once the work merges, the code, the spec and the roadmap are the
  record.
- Session notes, hand-overs and measurements kept outside the repo get lost. Anything a
  later session needs goes into one of the files above.

## Hardware testing

- The owner tests by eye on a bench (the strips and panels are listed in
  `docs/hardware/findings.md`). Agents build and flash; the owner reports what the LEDs do.
- Flashing: ESP boards with `espflash` (`cargo run --release` in the example); RP boards in
  BOOTSEL with `cargo run --release` (picotool), or copy the UF2; the XIAO nRF52840 by
  double-tapping RESET and copying the UF2.
- Record every result as described under "Recording results" in
  `docs/hardware/test-plan.md`.

## Git workflow

- Work on a branch and merge through a PR once CI passes. Delete the branch (locally and
  on GitHub) once it's merged.
- One branch per piece of work. Parallel work goes in its own git worktree; remove the
  worktree when its branch merges.
- Never leave finished work only on a local branch: push it and open a PR, even as a draft.
