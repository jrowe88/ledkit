# ledkit roadmap

**Design:** [specs/ledkit-design.md](specs/ledkit-design.md). The other files in
[specs/](specs/) hold each crate's design decisions; [hardware/](hardware/) holds the test
plan, what the boards taught us, and power measurements.

This file is the project's status: where each milestone stands, the decisions worth
remembering, and what's still open. Hardware pass/fail details live in each crate
README's "Tested on hardware" table, not here.

## Milestones

| # | Milestone | Status |
|---|---|---|
| 1 | Foundation + `ledkit-color` | ✅ done |
| 2 | `ledkit-core`: chip catalogue, color orders, encoders, `Strip`, the output pipeline (brightness, gamma, power limit, dithering), output traits, `smart-leds` interop | ✅ done |
| 3 | First light: `ledkit-spi` (clockless and clocked, blocking and async) | ✅ passes on the ESP32-C6, Teensy 4, Pi Zero W, ESP32-S3, RP2040, RP2350 and nRF52840 |
| 4 | ESP32 RMT backends: `ledkit-esp-idf` (`std`) and `ledkit-esp` (esp-hal, `no_std`) | ✅ passes on the ESP32-C6 DevKit and the ESP32-S3 Feather |
| 5 | `ledkit-rp`: RP2040/RP2350 PIO, processor-fed and DMA, blocking and async | ✅ passes on the Feather RP2040 SCORPIO and the Pico 2 W (Arm and RISC-V cores) |
| 5b | `ledkit-nrf`: nRF52840 PWM + EasyDMA, blocking and async | ✅ passes on the Seeed XIAO nRF52840 |
| – | `ledkit-uart`: NeoPixels from a serial port | ✅ passes on the ESP32-C6 and Teensy 4 (blocking, async and DMA) |
| 6 | `ledkit-layout`: matrices, `Canvas`, segments, `embedded-graphics` | ✅ passes on an Adafruit NeoPixel 8×8 |
| 7 | `ledkit-anim`: the `Animation` trait, 16 effects, combinators, palettes | ✅ passes on the 8×8 and a 100-LED strip (C6, Teensy 4, S3) |
| 7b | `ledkit-text`: X11 bitmap fonts, a Tiny 3×5, 7- and 14-segment faces at any height, stencil colours, text areas and a ticker | ✅ built and host-tested; the 8×8 text tour runs on the ESP32-S3 |
| 8 | "Just works" packaging: the `ledkit` facade + prelude, `cargo generate` board templates, the numbered example ladder, an mdBook guide with photos, `llms.txt` | 🚧 the `ledkit` facade passes on hardware ([spec](specs/ledkit.md)): prelude, ESP board helpers (esp-hal and ESP-IDF), `Frames`, start examples for the C6 and S3. Templates, guide and `llms.txt` to come |
| 9 | Release 0.1: publish to crates.io in dependency order, docs.rs config, the guide on GitHub Pages | ✅ all 13 crates published at 0.1.0 (2026-11-11) |
| 10 | Phase 1.5: `ledkit-sim` desktop simulator, a Claude Code plugin, remaining effects | not started |

## Decisions worth remembering

- **Dithering stays on by default** (2026-10-30), documented with its low-frame-rate
  flicker and turned off in the slow-rainbow examples.
- **DotStar HD brightness is on by default** (2026-10-31), and only lowers the 5-bit level
  below about 52%.
- **rp-hal, not embassy-rp,** for `ledkit-rp`: embassy-rp can't build for the RP2350's
  RISC-V cores.
- **An animation is a pure function of position and time** (`ledkit-anim`): no state
  between frames, so combinators need no buffers. Stateful effects need a second trait.
- **Parallel strips wait** for a multi-strip output in `ledkit-core` (owner's call,
  2026-11-01).
- **Text fonts:** the public-domain X11 misc-fixed fonts plus faces drawn for ledkit
  (3×5, 7- and 14-segment); 1x exactly as drawn, or 1.25x–2x anti-aliased, never shrunk.
  The [preview page](https://jrowe88.github.io/ledkit/text-preview/) is how font
  choices get made; later it should run ledkit's own code compiled to WebAssembly.
- **The `ledkit` facade** (2026-11-10): users add the HAL themselves (no HAL
  re-exports); board helpers are shortcuts made of public pieces, never a second API; the
  esp-hal helper is a macro (`strip!`) because the pulse buffer's length can't be
  computed from `N` in a generic function on stable Rust. No `ledkit-rp` in the facade
  yet: rp235x-hal and esp-hal pin different `riscv-rt` versions, and Cargo won't put both
  in one dependency graph, even as optional features.

## Open follow-ups

### Release 0.1 (milestone 9)

- Published 2026-11-11: all 13 crates at 0.1.0, and the GitHub Release `v0.1.0`.
- How a release goes (for 0.2): the owner publishes from a clean `main` with their own
  crates.io token. `cargo publish --workspace` first (color, core, layout, spi, uart,
  anim, text), then each board crate from its folder, naming its chip, since they only
  build for it:
  - `crates/ledkit-esp`: `cargo publish --features esp32c6 --target riscv32imac-unknown-none-elf`
  - `crates/ledkit-nrf`: `cargo publish --features nrf52840 --target thumbv7em-none-eabihf`
  - `crates/ledkit-rp`: `cargo publish --features rp2040 --target thumbv6m-none-eabi`
  - `crates/ledkit-esp-idf`: `cargo publish --no-verify` (verifying needs a full ESP-IDF
    build; the examples already build it in CI)
  - `crates/ledkit` last: `cargo publish --features esp32c6 --target riscv32imac-unknown-none-elf`

  Then date the CHANGELOG and publish the GitHub Release (publishing a draft creates
  the tag). crates.io allows a burst of five *new* crate names, then about one every 10
  minutes (a 429 says when to retry); new versions of existing crates aren't limited
  that way. If `--workspace` stops partway, publish the rest with `-p <crate>`, since
  rerunning `--workspace` would try the published ones again.
- Check each docs.rs page. `ledkit-esp-idf`'s docs will likely fail there (its build
  downloads ESP-IDF); if so, say so in its README and point at the facade's docs or a
  GitHub Pages copy.
- README photos: the placeholder gallery was dropped (2026-11-10); add real bench photos
  or GIFs when there are good ones (`assets/README.md` has tips).

### Brightness and power (next up)

- A per-chip (or per-strip) LED response model, so the brightness curve and the power
  estimate know when an LED applies its own gamma (the APA102-2020 does, about 2.6).
  Measure the APA102 5050 and SK9822 strips first. Data:
  [hardware/power-measurements.md](hardware/power-measurements.md).
- A way to set a measured power model on a `Strip` (the WS2812B strip draws about 11 mA per
  channel, not the assumed 20).
- Power limiter cost: with a limit set, `show()` runs the pipeline over all pixels three or
  more extra times. Time it on a Cortex-M0 with 300 LEDs; if it matters, compute
  per-channel sums once and verify a close candidate.
- Later, if wanted: a per-LED HD split (FastLED-style) for dim pixels in a bright frame.

### Hardware rounds not run yet

From [hardware/test-plan.md](hardware/test-plan.md), on every board where they apply:
busy CPU (T8), non-blocking frame (T9), long frame (T10), and radio on (T11: Pico 2 W
CYW43, nRF BLE). Also: a 300-LED strip on the RMT for both ESP crates, async `ledkit-esp`
on a real Embassy executor with other busy tasks, WS2812B over SPI at 2.3, 2.4, 3.2, 4.0
and 6.4 MHz, and RGBW byte order.

### Core

- `MAX_CLOCK_HZ` for the clocked chips (about 20 MHz APA102, 15 MHz SK9822).
- Datasheet citations in `chips.rs`.
- Optional `defmt::Format` on the error types.
- A multi-strip output (unblocks parallel strips on the SCORPIO and the Teensy 4).

### Backends

- **ESP:** drop the `ledkit-esp-idf` even-length padding once an esp-idf-hal release after
  0.47.0 is out; async for `ledkit-esp-idf`; `TxChannelConfig::with_memsize` for
  `ledkit-esp` if T8 shows refill glitches; ESP32-H2 needs a per-chip RMT tick.
- **RP:** an embassy-rp flavour (Arm only: interrupt-driven async, CYW43 alongside); a
  "start the frame, wait later" API on `PioDmaOutput`; one program shared by several state
  machines and the SCORPIO's 8 parallel channels; a clocked PIO output for DotStars if SPI
  pins are ever a constraint; matrix and anim examples; portable `rp2040-spi`,
  `rp2040-uart`, `rp2350-spi` and `rp2350-uart` examples.
- **nRF:** I2S + EasyDMA output; frames over about 1350 LEDs; recovering from a cancelled
  async frame; an interrupt-driven async wait; several strips per PWM instance;
  `nrf52840-hal` support; other nRF52 chips.
- **Teensy 4 parallel strips:** see the research notes in
  [hardware/findings.md](hardware/findings.md).

### Text

- Hardware check still to do: `examples/esp32c6-text` (the 8×8 ticker). `examples/rp2350-text`
  builds but won't be run on a board (the same tour passed on the ESP32-S3).
- Smoothing at 1x (the preview page's "smoothed 1x") isn't in the crate yet.
- On-target timing for 192×64: how long one frame takes on a real chip.
- The preview running the real crate compiled to WebAssembly, so it can't drift. Today the
  bitmap faces use the crate's generated tables, but the 7- and 14-segment faces are a
  JavaScript copy checked against the crate's results (`reference.mjs --check` in CI).

### Layout and animation

- Panel presets, each with a stated orientation (the Adafruit NeoPixel 8×8 is confirmed).
- `From` impls for `Rgb` ↔ `Rgb888` in `ledkit-color` behind an `embedded-graphics`
  feature.
- A BMP image example; ring layouts.
- Stateful effects (Game of Life, particles, Fire2012) through a second trait with a
  caller-sized buffer.
- A border marquee, rectangular regions as segments, mirroring by chain order.
- Non-`'static` palettes, if palettes built at run time are wanted.

### The `ledkit` facade

- Hardware: all facade checks pass (2026-11-11; see `crates/ledkit/README.md`):
  `esp32c6-start` on the C6, blocking and async; `esp32s3-start-idf` (the ESP-IDF
  helpers) on the S3 Feather; the esp-hal helpers and `Frames` on the S3's onboard
  NeoPixel; and the `esp32s3-text` tour on the facade.
- Board helpers for the other boards, following the ESP pattern: Pico (once `ledkit-rp`
  can join the facade, see the decisions), nRF52840, an ESP SPI helper for DotStars.
- A `ledkit-rp` feature, when rp235x-hal and esp-hal agree on `riscv-rt`.
- An async ESP-IDF output, if wanted.

### Consumers

- The pulse project (`esp32-c6-pulse-rust`) can replace its `status_led.rs` with `Breathe`
  and `Rainbow`: a good acceptance test for `ledkit-anim`.
