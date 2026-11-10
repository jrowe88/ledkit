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
| 8 | "Just works" packaging: the `ledkit` facade + prelude, `cargo generate` board templates, the numbered example ladder, an mdBook guide with photos, `llms.txt` | not started |
| 9 | Release 0.1: publish to crates.io in dependency order, docs.rs config, the guide on GitHub Pages | not started |
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

## Open follow-ups

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

- Hardware checks still to do: `examples/esp32c6-text` (the 8×8 ticker) and `examples/rp2350-text`.
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

### Consumers

- The pulse project (`esp32-c6-pulse-rust`) can replace its `status_led.rs` with `Breathe`
  and `Rainbow`: a good acceptance test for `ledkit-anim`.
