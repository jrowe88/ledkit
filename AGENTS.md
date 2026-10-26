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
| `ledkit-anim` | `Animation` trait, effects, combinators | `ledkit-color`, `ledkit-layout` |
| `ledkit-spi` | Backend over `embedded-hal` 1.0 SPI | `ledkit-core` |
| `ledkit-esp` | ESP32 RMT backend (esp-hal, `no_std`) | `ledkit-core`, `esp-hal` |
| `ledkit-esp-idf` | ESP32 RMT backend (esp-idf-hal, `std`) | `ledkit-core`, `esp-idf-hal` |
| `ledkit-rp` | RP2040/RP2350 PIO backend | `ledkit-core`, an RP HAL |
| `ledkit-sim` | Desktop simulator (`std`) | `ledkit-anim` |

Only `ledkit-color` exists so far; the rest are planned.

## Conventions

- `no_std` and no `alloc` in the library crates (only `ledkit-esp-idf` and `ledkit-sim` use `std`).
- Integer math per pixel; floating point only where it is computed once (for example tables).
- Readable names over terse ones. This library is for beginners.
- Every public item has docs and an example.
- Tests live in the module they test (`#[cfg(test)] mod tests`).
- Before committing run `cargo test --workspace` and the CI commands:
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features -- -D warnings`,
  and the `no_std` builds for `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf`.
- Never copy code from LGPL/EUPL projects. MIT/Apache sources may be used with attribution in `NOTICE`.
