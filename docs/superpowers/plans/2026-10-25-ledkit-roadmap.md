# ledkit roadmap: plan of attack

**Spec:** `docs/superpowers/specs/2026-10-25-ledkit-design.md`

The spec covers about ten crates plus docs and tooling, so it is delivered as a series of
milestones. Each milestone gets its **own detailed plan**, written when the milestone
starts, so it can build on the real interfaces the previous milestone produced. Each one
ends with something that works and is tested on its own.

| # | Milestone | Delivers | Proves | Needs hardware? |
|---|---|---|---|---|
| **1** | **Foundation + `ledkit-color`** | Cargo workspace, CI, licenses, README/AGENTS/CONTRIBUTING stubs; the complete color crate | The crate conventions, `no_std` builds on a no-FPU Arm target and on RISC-V, and the readable-API style | No |
| **2** | **`ledkit-core`** | LED chip catalogue (all phase 1 chips), color orders, encoders for every technique (pulse pairs, SPI with clock validation, clocked frames), `Strip<Led, N>`, the output pipeline (brightness, gamma, power limit, dithering), blocking + async output traits, `smart-leds-trait` interop | The central "define a chip once, encode for any technique" idea, entirely in host-side unit tests | No |
| **3** | **First light: `ledkit-spi`** | `ledkit-spi` (blocking + async, `smart-leds` impls) plus first light on the ESP32-C6 onboard LED; Raspberry Pi (spidev) and Teensy 4 (LPSPI) examples built in CI | NeoPixel via SPI encoding on real hardware (ESP32-C6 passed 2026-10-30); the Teensy 4 (2026-10-31) and Pi Zero W (2026-11-01) examples passed on hardware too | ESP32-C6 (done); Pi Zero / 3 A+, Teensy 4, WS2812B + APA102 strips (follow-ups) |
| **4** | **ESP32 backends** | Delivered: `ledkit-esp-idf` (ESP-IDF, `std`; the pulse project's RMT `Output` grown into a crate, with the reset gap enforced) and `ledkit-esp` (esp-hal RMT, `no_std`, blocking + async); core additions `BitPulse`, `ResetGap` and the RMT word helpers in `ledkit_core::encode`; examples `esp32c6-esp-idf` and `esp32c6-rmt` built in CI; pulse switched to `ledkit-esp-idf` and passed on hardware (pulse commit awaiting the owner) | Both ESP32 worlds on the C6 DevKit. `ledkit-esp-idf` passed on hardware 2026-10-30 (colors, back-to-back frames; the rainbow needed dithering off); `ledkit-esp` passed on the onboard LED 2026-10-30 (long strips untested); S3 (Xtensa) still to do | ESP32-C6 (done for both), ESP32-S3, strips |
| **5** | **`ledkit-rp`** | RP2040/RP2350 PIO backend | Pico 2 | Pico 2, strips |
| **5b** | **`ledkit-nrf`** | Delivered (builds; hardware pending): nRF52840 PWM + EasyDMA backend (`embassy-nrf`, blocking + async); core `equal_period_ticks` and PWM duty words; examples `nrf52840-pwm`, `nrf52840-spi`, `nrf52840-dotstar` built in CI | Every native nRF path on the Seeed XIAO nRF52840 | XIAO nRF52840, strips, DotStars |
| **6** | **`ledkit-layout`** | Delivered: `Layout`, `MatrixLayout` (serpentine, columns first, any starting corner), `Rotated` and `Tiled`, `Canvas`, `Segment`, and `embedded-graphics` support (a `Canvas` is a `DrawTarget`, with `rgb_from_rgb888` / `rgb888_from_rgb`); example `esp32c6-matrix` (wiring finder, shapes, scrolling text) builds in CI. No panel presets (see follow-ups) | Text and shapes on a matrix: tested on an Adafruit NeoPixel 8×8 on the ESP32-C6 (2026-10-31) | Adafruit NeoPixel 8×8 |
| **7** | **`ledkit-anim`** | Delivered: the `Animation` trait (a pure function of position and time) and `Surface`; 16 effects (the phase 1 list plus swirl, rings, pinwheel, plasma and rain for matrices) and `Effect` by name; modifiers, layers with blend modes, transitions (fade, fade through black, wipe, dissolve, iris), `then`/crossfade, playlists and segments; `ColorMap` and named palettes in `ledkit-color`; examples `esp32c6-anim` and `anim-preview`, built in CI | Combined effects on strips and matrices: host-tested; hardware check on the 8x8 and a strip pending | Strips, matrix |
| **8** | **"Just works" packaging** | The `ledkit` facade + prelude, `cargo generate` board templates, the numbered example ladder, the mdBook guide with photos, `AGENTS.md` + `llms.txt` | A hobbyist going from nothing to a lit strip by following the guide | All first-class boards |
| **9** | **Release 0.1** | Publish all phase 1 crates to crates.io in dependency order, docs.rs config, guide on GitHub Pages, announcement | Others can depend on it | No |
| 10 | Phase 1.5 | `ledkit-sim` desktop simulator, Claude Code plugin, remaining effects | | No |

**Ordering rationale:**
- Milestones 1–2 need no hardware, so they can start before your new boards arrive.
- Milestone 3 uses the cheapest, most universal backend, on hardware you already own, to
  validate the core design before the chip-specific backends depend on it.
- Milestones 6–7 can run in parallel with 4–5 once milestone 2 is done, since they depend
  only on the core and color crates.

**Repository:** `C:\Dev\jrowe88\ledkit`, to be pushed to `github.com/jrowe88/ledkit` once
milestone 1 is done.

## Notes for the milestone 3 plan (from the milestone 2 final review)

- **SPI, byte alignment:** encodings with 3, 5, 6 or 7 bits per LED bit put high pulses across byte boundaries. Blocking (non-DMA) SPI often leaves small gaps between bytes, which stretch a high pulse and corrupt the bit. Add a byte-aligned option (N ∈ {4, 8}) and use it whenever gap-free transfer isn't guaranteed. `encode` always adds the lead-in and reset bytes, so for small-RAM streaming, add a framing-free `encode_bits` or a stateful encoder.
- **RMT:** `PulseTicks` is enough (one symbol per bit, plus an end marker; split long resets over 32767 ticks). Compute it once in the constructor. Buffers are sized with a user-side `const` (encoders are `const fn`) or a caller-provided `&'static mut` buffer.
- **PIO:** the standard ws2812 program needs zero and one bits to have the same period (T1/T2/T3). `PulseTicks` doesn't guarantee that (WS2812B at 10 MHz is 13 vs 14 ticks), so add an equal-period search; it also serves PWM+DMA later.
- **Chip kind at compile time:** use `const { ClocklessTiming::of::<C>() }` in backends so a DotStar chip on a clockless backend is a compile error. Optionally add `ClocklessChip` / `ClockedChip` sub-traits.
- **Reset gap:** RMT and PIO backends must keep consecutive `show()` calls at least `reset_us` apart. SPI includes the reset bytes.
- **Clocked chips:** add a `MAX_CLOCK_HZ` (about 20 MHz for APA102, about 15 MHz for SK9822).
- **Error types:** consider an optional `defmt::Format` on error types for Embassy and esp-hal users.
- **Hardware verification checklist:**
  - WS2812B (V5 and older) at 2.3, 2.4, 3.2, 4.0 and 6.4 MHz SPI
  - the zero-high upper bound (450 ns) and the one-low lower bound (450 ns)
  - the SK9822 end frame
  - RGBW byte order
  - backfill datasheet citations in `chips.rs`
- **Constructors:** give backends one-line constructors so a hobbyist's first program stays short.
- **Power limiter cost:** with a limit set, `show()` runs the pipeline over all pixels about three or more extra times. Measure it on a Cortex-M0 with 300 LEDs. If it matters, process once at full brightness to get per-channel sums, use them to pick a close candidate, then verify only once or twice. (From the milestone 2 code review, item 4.)

## Hardware follow-ups after milestone 3

Milestone 3 shipped with first light on the ESP32-C6 only. These items need hardware and
were deferred:

- ~~Teensy 4 hardware test~~: NeoPixel pass 2026-10-31 on an Adafruit 8×8, and on a 100-LED WS2812-type strip 2026-11-01 (three 512-byte writes per frame; the pauses between them did no harm). It needed two LPSPI settings in the example (tristate the data pin between transfers, no PCS-to-SCK delay); without them the first LED of each transfer reads a stray "1". DotStar pass 2026-10-31 on an Adafruit DotStar HD 8×8 (APA102-2020, 64 LEDs). It exposed an end-frame bug: the old zero-byte end frame left LEDs 60 to 63 unlit, and `0xFF` end bytes fixed it.
- ~~Pi hardware test~~: pass 2026-11-01 on a Raspberry Pi Zero W (32-bit Pi OS trixie, built on the Pi in about 22 minutes) with a 100-LED WS2812-type strip through a 74AHCT125 on GPIO10: colors and a flicker-free rainbow, with `dtparam=spi=on` and `core_freq=250`
- WS2812B strip at 2.3, 2.4, 3.2, 4.0 and 6.4 MHz
- ~~APA102/SK9822 end frame~~: done. The APA102 end frame is now `ceil(n / 16)` bytes of `0xFF` (as in Adafruit's library), verified on an APA102-2020 on 2026-10-31, and on a 300-LED APA102-type 5050 strip on 2026-11-01 (ESP32-C6, 4 MHz, frame sized to exactly 300: every frame latched at once through to the last LED). The SK9822 keeps a 4-byte zero reset frame plus `ceil(n / 16)` zero bytes (`ClockedFormat::Sk9822`); that SK9822 end frame passed on a 144-LED strip on 2026-10-31 (ESP32-C6, 4 MHz, frame count set to exactly 144): every frame latched at once, through to the last LED, with HD brightness on.
- DotStar on the ESP32-C6: pass 2026-10-31 (SPI2 at 4 MHz, and a later sweep of 1, 2, 4, 8, 12, 16, 20, 26, 32 and 40 MHz requested all ran clean through a 74AHCT125 on a breadboard; data GPIO4, clock GPIO6, Adafruit DotStar HD 8×8). At 10% brightness nothing seemed to light: ledkit dims DotStars only through the 8-bit color values, so 10% green is 3/255, invisible on these LEDs; 40% looked right.
- ~~DotStar "HD" brightness~~: done (`Strip::set_hd_brightness`, on by default since 2026-10-31 at the owner's call; `Output::set_global_brightness` lets a backend take the 5-bit level, and `ClockedSpi` does). It lowers the 5-bit level only below raw level 64 (about 52%); above that it sends the same bytes as without HD, because on an APA102-2020 a lowered 5-bit level gives less light than the math says (tested on an ESP32-C6, 2026-10-31). Fixed after review (PR #8): the split now matches the pipeline's `(level + 1) / 256` scale, and the power limit counts the HD frame. Side-by-side check 2026-11-01 (`examples/esp32c6-dotstar-hd`, plain and HD rows in one frame): HD lights from 10%, plain only from 20% (about as bright as HD at 10%), roughly equal at 40% and equal at 60%. Later, if wanted: a per-LED split (FastLED-style) for dim pixels in a bright frame
- `MAX_CLOCK_HZ` for the clocked chips
- RGBW byte order
- datasheet citations in `chips.rs`
- `defmt` on the error types

## Notes from the first consumer (esp32-c6-pulse-rust, 2026-10-30)

The pulse project switched its status LED to `ledkit-core` with a ~60-line RMT `Output`
(`src/rmt_output.rs` there; the seed for `ledkit-esp-idf`). It built first try on
`riscv32imac-esp-espidf`. What came up:

- **Per-pixel perceptual dimming** (done: `Dim`, milestone 3 and the quick-fix commit): the breathe effect had to
  animate the strip's global `set_brightness` every frame. That's fine for one LED, but
  per-pixel fades on a strip (twinkle, comet tails) need something like
  `Rgb::dimmed(Brightness)` on the same perceptual curve; today only the linear
  `Rgb::scale(u8)` exists. Also document whether changing brightness every frame is an
  intended use (it interacts with dithering and the power limit). (Done: the brightness-every-frame docs.)
- **Imports** (done for the docs: prelude-first crate docs and README; the rest is milestone 8): `Rgb` is at the crate root but `Hue` is under
  `ledkit_core::color`; the consumer found both by reading source. The prelude has
  everything needed, so lead the crate docs and README with `use ledkit_core::prelude::*;`.
- **Reset gap (done: `ResetGap` and enforcement in `ledkit-esp-idf`, milestone 4; the `Output` docs are done):** nothing enforces the gap between frames; pulse only meets the
  WS2812B's 280 µs because it sends every 10 ms. `ledkit-esp-idf` should remember when it
  last sent and wait out the rest, and the `Output` docs should say backends own this.
- **RMT tick type (done in milestone 4):** `pulse_ticks` took and returned `u32`, but the RMT wants
  `u16` (max 32 767), so the consumer passed 32 767 and cast. Now `BitPulse` (from
  `PulseTicks::zero_pulse` / `one_pulse`) holds `u16` ticks, and the backends pass the RMT's
  maximum once in their constructors.
- **Effects (milestone 7, still open):** pulse's breathe (raised cosine, elapsed-time phase) and rainbow
  are exactly the planned `ledkit-anim` effects; once they exist, `status_led.rs` should
  shrink to almost nothing, which makes pulse a good acceptance test.
- **Pulse moves to `ledkit-esp-idf` (done, milestone 4):** pulse now uses `ledkit-esp-idf` instead of its own `rmt_output.rs`, and passed on hardware 2026-10-30. Porting its minimum brightness surfaced that perceptual brightness below about 6% is off; now documented on `Brightness`.

## Notes from milestone 4 hardware checks (2026-10-30)

- **Dithering at low frame rates (decided 2026-10-30: keep it on and document it):** the esp32c6-esp-idf rainbow flickered visibly, worst from blue to magenta. Cause: at 20% brightness each channel reaches the LED as only about 0 to 9, the loop ran at about 45 fps (20 ms delay), so the 8-frame dither cycle repeated at about 5 to 6 Hz, which the eye sees. `set_dithering(false)` fixed it. All five examples now turn dithering off in their slow rainbows, and the docs and example READMEs have a troubleshooting line. The owner decided to keep dithering on by default and document the trade-off (done in the core docs, `set_dithering`, and every example README). Alternatives considered and set aside: a 4-frame cycle, skipping dithering at very low output levels, and off by default.
- **Hardware status:** `ledkit-spi` on ESP32-C6: pass (2026-10-30; the slow rainbow example now turns dithering off). `ledkit-esp-idf` on the ESP32-C6 DevKit (chip rev v0.2): colors pass, back-to-back frames latch pass, rainbow steady with dithering off. Pulse on `ledkit-esp-idf`: pass. `ledkit-esp` (`esp32c6-rmt`): pass on the onboard LED (colors, back-to-back frames, rainbow with dithering on).

## Hardware follow-ups after milestone 4

- Xtensa build and test (ESP32-S3) for both ESP crates
- an external strip on the RMT at 300 LEDs (buffer RAM and timing), for both crates
- async `ledkit-esp` on hardware with Embassy
- async for `ledkit-esp-idf`
- ESP32-H2 support needs a per-chip RMT tick (its RMT runs from a 32 MHz clock, which can't make the 10 MHz tick)
- ~~check long strips on the esp-hal RMT for underruns~~: done 2026-10-31 on an ESP32-C6. A 100-LED WS2812-type strip ran clean with the blocking and the async driver (async polled with `embassy_futures::block_on`), at 50 frames a second, and the 64-LED matrix ran clean blocking. Still open: async on a real executor with other busy tasks; later consider `TxChannelConfig::with_memsize`
- `ClocklessSpi` long strip on the ESP32-C6: pass 2026-10-31. A 100-LED WS2812-type strip at 3.2 MHz ran clean with `new` and with `new_packed` (esp-hal blocking SPI, refilling its 64-byte FIFO mid-frame), so the FIFO refills leave no gap the LEDs notice. One board; keep the `new_packed` warning for other HALs
- `ledkit-esp-idf` long strip on the ESP32-C6: found and worked around an esp-idf-hal 0.47 bug (2026-11-01). Its RMT encoder bridge panics on the combined state "complete + memory full" (`Unknown rmt_encode_state_t value: 3`, `src/rmt/encoder.rs:85`), which ESP-IDF's bytes encoder returns whenever a frame ends exactly on a refill block: every WS2812B frame of 4+ LEDs whose bit count is a multiple of 48 on the C6. `RmtOutput` now pads even-length frames with one `0x00` byte; verified with every length from 1 to 150 LEDs over serial, and a 100-LED strip running colors, counting and a rainbow that look right by eye. Already fixed upstream in esp-rs/esp-idf#600 (merged 2026-09-23, not in a release yet). Follow-up: drop the padding once an esp-idf-hal release after 0.47.0 is out
- ~~use `--locked` in the CI example builds~~ (done in the milestone 4 review fixes)

## Follow-ups after milestone 6 (`ledkit-layout`)

- ~~**Hardware test of `esp32c6-matrix`**~~ (done 2026-10-31: Adafruit NeoPixel 8×8, `.columns_first().starting_at(Corner::BottomLeft)` with the data-in pin at the bottom left)
- add verified presets: the Adafruit NeoPixel 8×8 is now confirmed (above), but its layout depends on which way the panel is held, so a preset needs a stated orientation
- move `Rgb` ↔ `Rgb888` conversions to `From` impls in `ledkit-color` behind an `embedded-graphics` feature (today they are the functions `rgb_from_rgb888` / `rgb888_from_rgb`, because a `From` impl isn't possible in `ledkit-layout`)
- an image (BMP) drawing example
- layouts for rings (circle coordinates), if wanted later

## Text support (in design, 2026-11-01)

Fonts, digit sets and a scrolling-text effect for matrices, from the 8×8 up to three
64×64 HUB75 panels (192×64). Decided so far with the owner, using the preview page:

- **Fonts:** the public-domain X11 misc-fixed bitmap fonts (4x6 to 10x20, in
  `assets/fonts/x11-misc-fixed`), plus faces drawn for ledkit: a 3×5 (capitals and digits),
  7-segment digits (numbers and clock labels only) and 14-segment (every capital letter).
- **Sizes:** 1x exactly as drawn, or 1.25x / 1.5x / 2x anti-aliased with adjustable edge
  sharpness; never shrunk (it blurs). The segment faces are drawn at any height.
- **Colour:** text is a stencil. The letters, an optional one-LED outline and the background
  are each any animation (a solid colour is the simple case); reverse swaps letters and
  background.
- **Preview:** `site/text-preview` (built by `build.py` from the vendored fonts), published
  to <https://jrowe88.github.io/ledkit/text-preview/> by the Pages workflow. Later it should
  run ledkit's own code compiled to WebAssembly, so it can't drift from the library.
- Still open: scrolling behaviour, where the code lives (a module or a new crate), and the API.

## Notes on Teensy 4 WS2812 techniques (research, 2026-11-01)

How PJRC's C++ libraries drive WS2812s on the Teensy 4, and what exists in Rust. The serial
technique shipped as `ledkit-uart` (design `specs/2026-11-01-ledkit-uart-design.md`,
plan `plans/2026-11-01-ledkit-uart.md`; results in the next section); parallel strips are paused by
the owner until `ledkit-core` has a multi-strip output. These notes are background for both.

- **WS2812Serial** (the model for `ledkit-uart`): an LPUART at 4 Mbaud with inverted output
  and DMA; ten 250 ns slots per byte carry 2 LED bits. Its timing is fixed: a "1" is 1 µs
  high and 0.25 µs low, outside ledkit's WS2812B window, which is why `ledkit-uart` picks the
  high-slot counts from each chip's timing instead (anti-goal in the design). 12 buffer bytes
  per RGB LED, 300 µs latch, UART TX pins only, best under about 600 LEDs per pin.
- **OctoWS2811** (`OctoWS2811_imxrt.cpp`, MIT; parallel strips, paused): QuadTimer 4 channels
  0 to 2 mark the bit start, the "0" point (0.30 µs) and the "1" point (0.75 µs); XBAR turns
  each into a DMA request, and three DMA channels write the standard GPIO1–GPIO4 set/clear registers
  (set every LED pin, clear the pins whose bit is 0, clear the rest). Any pins on up to four
  GPIO ports (8 by default); pixels are transposed so each word holds one bit per pin,
  double-buffered in about 10 KB with an interrupt refilling the idle half. Seen in passing:
  the BGRW case uses blue twice, and `busy()` never really checks the DMA.
- **Rust on the Teensy 4:** `imxrt-hal` 0.6.0 has `lpuart` (with inversion and `dma_write`),
  `lpspi`, `dma`, `gpio`, `flexpwm`, `gpt` and `pit`, but no FlexIO, QuadTimer or XBAR
  drivers, so an OctoWS2811-style output would first need those on `imxrt-ral`.
  [`ws2812-flexio`](https://github.com/finomnis/ws2812-flexio) (MIT/Apache, 0.3.0, June 2024)
  already drives up to 3 parallel strips per FlexIO block, blocking or DMA, with Teensy 4.0
  examples (fixed 800 kHz, RGB only, quiet since 2024).
- **When parallel strips resume:** look at wrapping or contributing to `ws2812-flexio` before
  writing FlexIO support from scratch; an OctoWS2811-style QuadTimer/XBAR output is the most
  work and only worth it for many strips.

## Serial-port technique (`ledkit-uart`, 2026-11-01)

- Shipped: `ledkit-core`'s `UartEncoding` (2 LED bits per inverted 8N1 serial byte, computed from the chip's windows at the real baud, with nearest-baud errors) and the `ledkit-uart` crate (`ClocklessUart`, blocking over `embedded-io`, async over `embedded-io-async`, a reset pause after each frame). Design: `docs/superpowers/specs/2026-11-01-ledkit-uart-design.md`.
- Hardware 2026-11-01, 100-LED WS2812-type strip: ESP32-C6 (UART1, GPIO4 inverted in the GPIO matrix, blocking and async) pass; Teensy 4 (LPUART6 on pin 1, blocking and DMA) pass. The Teensy BSP clocks its UARTs at 8 MHz (max 2 Mbaud), so the example raises the UART clock to 24 MHz for 4 Mbaud.
- Still paused: parallel strips (OctoWS2811-style GPIO + DMA, or FlexIO). They need a multi-strip output in `ledkit-core` first. `ws2812-flexio` 0.3 is on `imxrt-ral` 0.5 / `imxrt-dma` 0.1, so it can't take the peripherals from `teensy4-bsp` 0.6 without a port.

## Milestone 7 (`ledkit-anim`, 2026-11-01)

- Design: `docs/superpowers/specs/2026-11-01-ledkit-anim-design.md`; plan: `docs/superpowers/plans/2026-11-01-ledkit-anim.md`.
- Hardware check still to do (by eye): `esp32c6-anim` on the Adafruit NeoPixel 8x8 (`MATRIX = true`) and the 100-LED WS2812 strip (`MATRIX = false`).
- Follow-ups:
  - text and fonts (the owner asked): a couple of small fonts, digit sets in several sizes (3x5 up), and a scrolling-text effect
  - effects that need memory between frames (Game of Life, flocking, particles, FastLED's Fire2012): needs a second, stateful trait with a caller-sized buffer
  - a border marquee (lights running round the edge of a matrix), rectangular regions as segments, and mirroring by chain order on matrices
  - a generic (non-`'static`) palette option for effects, if palettes built at run time are wanted
  - pulse's `status_led.rs` can now use `Breathe` and `Rainbow` (the acceptance test noted under "first consumer")

## Milestone 5b (`ledkit-nrf`, 2026-11-02)

- Design: `docs/superpowers/specs/2026-11-02-ledkit-nrf-design.md`; plan: `docs/superpowers/plans/2026-11-02-ledkit-nrf.md`.
- Shipped: `ClocklessTiming::equal_period_ticks` (shared with `ledkit-rp`'s PIO), `encode::pwm` duty words, `ledkit-nrf`'s `PwmOutput` (one sequence per frame, up to 32 767 words, reset gap as trailing low words). The WS2812B runs at 22 ticks (1.375 us) per bit, not nrf-smartled's 20, to keep its `1` low at or above 450 ns.
- Hardware check still to do (by eye): `nrf52840-pwm` (blocking, async with a busy task, 1–150 sweep), `nrf52840-spi` (SPIM3 `new_packed` and `new` at 4 MHz, `ledkit-uart` refusing 1 Mbaud), `nrf52840-dotstar` (SPIM3 clock sweep 1–32 MHz, HD steps). Record results in each README and in `crates/ledkit-nrf/README.md`.
- Follow-ups: I2S + EasyDMA output; frames over ~1350 LEDs (both sequences, or refill); recovering from a cancelled async frame; an interrupt-driven async wait; several strips per PWM instance; BLE during a frame (T11); `nrf52840-hal` support; other nRF52 chips.
