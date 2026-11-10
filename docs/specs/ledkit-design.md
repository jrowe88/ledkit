# ledkit: design

*Date: 2026-10-25. Status: first cut for review. "ledkit" is a working name (free on crates.io).*

## Intent

An Adafruit-quality family of Rust crates for addressable LEDs (strips, rings and matrices),
aimed at **hobbyists**: people with roughly Arduino-level experience who are curious about
Rust or have chosen it for a project. It should "just work": low friction, low research,
learn by doing. The library worries about chip-level details so the user doesn't have to.

The existing Rust ecosystem (`smart-leds`, per-chip drivers, `blinksy`, `smart-leds-fx`)
has many of the pieces, but they are scattered across crates with different conventions,
thin on documentation, and missing several features. ledkit **works alongside** that
ecosystem rather than replacing it.

**Success = usage, and other people wanting to contribute.** The owner is fine with the
project not taking off ("nothing ventured, nothing gained"), so the design doesn't need to
hedge for scale. It should be good, clear, and easy to contribute to.

## What "easy" means (the owner's words, lightly edited)

1. A working, documented walkthrough on the web, with pictures.
2. Easy-to-find, working, well-commented examples.
3. Clear constants and config that make setup easy.
4. APIs that read clearly, not cryptic, jargon-heavy or overly technical.
5. Examples that build on each other and combine concepts.

## Goals

- **Hardware-agnostic by technique, not by board.** Support is organized around the handful
  of ways a microcontroller can generate LED signals, so new boards need little or no new code.
- **Comprehensive low level:** every LED type Adafruit and SparkFun sell, with all color
  orders and correct timing.
- **High-level color utilities and animation**, usable on any hardware, and reusable later
  for other display types (see phase 2).
- **First-class strips and matrices** of any size. A single onboard LED is just a strip of 1.
- **AI-friendly:** the project ships instructions and tooling so Claude, ChatGPT and similar
  tools give correct answers about ledkit (see AI-assisted development).

## Non-goals (for now)

- **HUB75-style RGB matrix panels** (row/column scanned with R/G/B data lines). A different
  beast, planned for **phase 2**. The color, layout and animation crates must not assume a
  one-wire strip, so they can drive those panels later.
- **Classic AVR Arduino** (Uno/Nano, ATmega328): 2 KB of RAM and niche Rust support.
- 3D layouts, network or remote control (WLED-style), and audio-reactive effects: possible
  later, not in the first release.

## Constraints

- **License:** MIT OR Apache-2.0, the Rust ecosystem standard. Code from copyleft projects
  (`blinksy` is EUPL-1.2, Adafruit_NeoPixel is LGPL-3.0) must not be copied; their ideas
  and APIs can be reimplemented. MIT/BSD code (Adafruit CircuitPython libraries,
  rpi_ws281x) may be ported with attribution.
- **`no_std` and no heap allocation** for everything except the simulator and the
  `std`-only ESP-IDF backend. An optional `alloc` feature may add conveniences
  (for example boxed animations).
- **No nightly features** in any `no_std` crate. RISC-V and Arm targets (ESP32-C3/C6,
  RP2040/RP2350, nRF, SAMD, STM32) then build on stock stable Rust. Xtensa ESP32s
  (original, S2, S3) need Espressif's toolchain (installed with `espup`), and the
  ESP-IDF backend needs nightly. Both are upstream requirements, and the guide covers them.
- **Interop with `smart-leds-trait`** (the de facto standard driver trait, about 200k
  downloads per 90 days): ledkit drivers implement it, and any existing `smart-leds`
  driver can be used as a ledkit output.
- Both **blocking and async** (Embassy-compatible) use must be supported.

## Hardware scope

| Tier | Chip families | Why |
|---|---|---|
| **First-class** (examples, guide pages, tested on hardware) | **ESP32** (original ESP32, S2, S3, C3, C6; `no_std` via esp-hal *and* `std` via ESP-IDF) and **RP2040 / RP2350** (Pico, Pico 2) | The large majority of hobby boards: Adafruit Feather, Seeed XIAO, Arduino Nano ESP32 and Raspberry Pi Pico all use these. |
| **Supported** (works via the universal SPI technique, community-tested) | nRF52840, SAMD21/51, STM32, **Teensy 4.x** (via `teensy4-rs` / `imxrt-hal`), **Raspberry Pi computers running Linux** (via `linux-embedded-hal`'s spidev SPI), anything with an `embedded-hal` 1.0 SPI bus | Covers the remaining Feather/XIAO variants. The owner's Pi Zero and Pi 3 A+ provide early test hardware for `ledkit-spi`. A Teensy FlexIO backend is a natural later addition. |
| **Out of scope** | AVR; Teensy 3.x (NXP Kinetis: no maintained Rust HAL, last activity 2019); ESP8266 (no maintained Rust HAL; dropped by esp-rs) | See non-goals. |

**LED types (phase 1):**

| Family | Chips | Wiring |
|---|---|---|
| NeoPixel / one-wire ("clockless") | WS2812, WS2812B, WS2813, WS2815 (12 V), WS2811 (800 kHz and 400 kHz), SK6812 (RGB), SK6812 RGBW, SK6812MINI, SK6805, APA106 | Data only |
| DotStar / two-wire ("clocked") | APA102, APA102-2020 (SparkFun LuMini), SK9822 | Data + clock |

*Builder's call (settled during review):* the extra clockless chips differ from WS2812 only
in timing, so they are cheap to include. Legacy clocked chips (WS2801, LPD8806) and unusual
RGBW chips (TM1814) are deferred, but the clocked-protocol trait and the color-order types
must be general enough to add them later without breaking changes.

## Decided: who sets up the hardware

**The user's program does the normal HAL setup and hands ledkit a peripheral and a pin.**
ledkit does not initialize chips, clocks or buses itself. (The owner chose this option.)
This keeps backends small and robust against HAL changes.

The "only edit a pin number" experience comes from the **board templates and examples**
instead: each one ships with the chip setup already written and commented, and configuration
constants (`LED_PIN`, `LED_COUNT`, `LED_TYPE`, brightness, power limit) at the top of the file.
Backends should still offer a convenient one-line constructor per HAL, for example taking
the RMT peripheral and a pin, so the setup a hobbyist sees is as short as the HAL allows.

## Test hardware

| Chip family | Board | Status |
|---|---|---|
| ESP32-C6 (RISC-V, newer RMT) | Espressif DevKit | Owned |
| ESP32-S3 (Xtensa, PSRAM) | Adafruit ESP32-S3 Feather 4 MB + 2 MB PSRAM (#5477) | To buy |
| Classic ESP32 (older RMT) | Adafruit ESP32 Feather V2 | Optional |
| RP2350 | Raspberry Pi Pico 2 | To buy |
| RP2040 (8 PIO strip outputs, 5 V level shifting) | Adafruit Feather RP2040 SCORPIO | To buy |
| SAMD21 (Cortex-M0+, no FPU) | Adafruit Feather M0 WiFi (#3010) | Owned |
| STM32F2 (Cortex-M3) | Adafruit WICED WiFi Feather (#3056); flashing via SWD | Owned |
| i.MX RT1062 (Cortex-M7) | Teensy 4 | Owned |
| nRF52840 (PWM + EasyDMA reference) | Seeed XIAO nRF52840 or Adafruit Feather nRF52840 Express | Optional |
| Linux SPI | Raspberry Pi Zero, Pi 3 A+ | Owned |
| Debug probe | Raspberry Pi Debug Probe (`probe-rs`) | To buy (recommended) |

The owner expects to already own WS2812B, SK6812 RGBW and APA102/SK9822 strips (inventory
to be confirmed). Anything not tested on real hardware is labeled "untested on hardware" in
the docs until someone confirms it.

## Notes carried into milestone 2 (from the milestone 1 final review)

- `Strip` should hold a `&'static GammaTable` rather than a 256-byte table by value.
- Brightness convention: the API sketch above shows `set_brightness(0.3)` (a float), while
  `ledkit-color` scales with `u8` (0–255). `ledkit-core` must pick one convention for the
  public API and convert internally.
- `smart-leds-trait` color conversions live in `ledkit-color` (feature `smart-leds`),
  because the orphan rule prevents `ledkit-core` from implementing `From` between two
  foreign types.

## Key design decision: support techniques, not boards

Every one-wire LED encodes each data bit as a timed high pulse followed by a low pulse.
Microcontrollers can produce that in a few distinct ways:

| Technique | Peripherals | ledkit encoding |
|---|---|---|
| Pulse generator | ESP32 RMT, RP2040/RP2350 PIO, NXP FlexIO | (level, duration) pairs |
| SPI | Almost every MCU | Each LED bit becomes N SPI bits, **computed for the actual SPI clock** |
| PWM + DMA | STM32, nRF | One PWM duty value per bit |
| Clocked SPI (DotStar) | Any SPI | Native: start frame, per-LED brightness + color, end frame |
| Serial port (UART) | Teensy 4 LPUART, ESP32 UART, any UART with an inverted output | 2 LED bits per serial byte, computed for the actual baud (`ledkit-uart`, added 2026-11-01) |

The core crate holds **one definition per LED chip** (timing with tolerances, color order,
channels, frame format) and **computes each technique's encoding from it**, at compile time
where possible. It validates the result against the chip's tolerances and reports a clear
error instead of producing a flickering strip. (For example: "an SPI clock of 1 MHz is too
slow for WS2812B; use 2.4 to 3.8 MHz.") A backend for a new chip or HAL then only has to hand
pre-computed data to its peripheral, typically a few dozen lines.

The existing `ws2812-spi` crate hard-codes 4-bit patterns that are only valid for one SPI
frequency range; `esp-hal-smartled` has configurable timing but only for esp-hal. Neither
does this, and it's the main technical gap ledkit fills.

## Crates

All crates live in one GitHub repository (a Cargo workspace) and are versioned independently.

| Crate | Purpose | Depends on |
|---|---|---|
| **`ledkit`** | The one crate hobbyists add. Re-exports everything with a `prelude`; Cargo features pick the backend (`esp`, `esp-idf`, `rp`, `spi`). | All below (optional) |
| **`ledkit-color`** | Color types and utilities. | nothing (optional `rgb` interop) |
| **`ledkit-core`** | LED chip catalogue, color orders, technique encoders, the pixel buffer (`Strip`), output traits. | `ledkit-color` |
| **`ledkit-layout`** | Segments of strips; matrices and coordinate mapping; `embedded-graphics` support. | `ledkit-core` |
| **`ledkit-anim`** | Animation trait, built-in effects, and combinators. | `ledkit-color`, `ledkit-layout` |
| **`ledkit-spi`** | Universal backend over `embedded-hal` 1.0 SPI (blocking and async). Clockless via SPI encoding; clocked natively. | `ledkit-core` |
| **`ledkit-esp`** | ESP32 RMT backend for **esp-hal** (`no_std`). | `ledkit-core`, `esp-hal` |
| **`ledkit-esp-idf`** | ESP32 RMT backend for **esp-idf-hal** (`std`). | `ledkit-core`, `esp-idf-hal` |
| **`ledkit-rp`** | RP2040/RP2350 PIO backend. | `ledkit-core`, an RP HAL |
| **`ledkit-sim`** | Desktop simulator (`std`): shows a strip or matrix in a window, so effects can be developed and tested without hardware. | `ledkit-anim` |

Backends are separate crates (not just features) because HALs release breaking changes
often; a separate crate lets one backend update without touching the rest.

### `ledkit-color`

- `Rgb` and `Rgbw` (8-bit), `Hsv`, conversions to and from the `rgb` crate and
  `smart-leds-trait` types.
- Named colors (`Color::RED`, `Color::WARM_WHITE`, …) and a rainbow palette.
- Hue wheel (`Color::from_hue`), full HSV, `lerp`/`blend` between colors.
- Gradients and palettes (fixed-size, no allocation).
- **Gamma correction** with selectable gamma, using lookup tables generated at compile
  time by `const fn` (not copied from Adafruit).
- **Color temperature / white balance** correction.
- **RGB to RGBW conversion** (extracting the white channel), for SK6812 RGBW.

### `ledkit-core`

- **LED chip catalogue:** one type per chip (`Ws2812b`, `Sk6812Rgbw`, `Apa102`, …), each
  defining its protocol: clockless (bit timings with tolerances, reset time) or clocked
  (frame format), plus channels and default color order. Users can define custom chips
  with the same trait.
- **Color orders:** all 6 RGB orders and the RGBW orders in actual use, as types.
- **`Strip<Led, N>`:** a fixed-size pixel buffer with readable methods: `set(i, color)`,
  `get(i)`, `fill(color)`, `fill_range(range, color)`, `clear()`, slice access, `len()`.
- **Output-time processing**, applied when the buffer is sent, so stored colors are never
  degraded (unlike Adafruit's lossy `setBrightness`):
  - global brightness
  - gamma
  - **power limiting**: given a current budget, for example "max 2 A at 5 V", scale
    brightness so the strip never exceeds it (like FastLED; missing from the Rust ecosystem)
  - optional **temporal dithering** for smooth fades at low brightness
- **Encoders** for each technique (see above), with tolerance validation.
- **Output traits**, blocking and async, implemented by every backend. ledkit outputs also
  implement `SmartLedsWrite`, and any `SmartLedsWrite` driver can act as a ledkit output.

### `ledkit-layout`

- **Segments:** treat part of a strip as its own strip, optionally reversed.
- **Matrices:** width × height, progressive or serpentine ("zig-zag") wiring, choice of
  starting corner, rotation, and tiling several panels into one larger canvas. Covers
  Adafruit's 8×8, 16×16 and 8×32 flexible NeoPixel matrices out of the box.
- A 2D `Canvas` (`set(x, y, color)`), plus an `embedded-graphics` `DrawTarget`
  implementation, so text, shapes and images work on matrices (like Adafruit_NeoMatrix).

### `ledkit-anim`

- **Time-based, not frame-based:** an animation renders "what the LEDs look like at time
  *t*", given elapsed time by the caller. Speed stays correct regardless of frame rate.
  The pulse project learned this the hard way.
- One small `Animation` trait that renders onto a strip or canvas.
- **Built-in effects (phase 1):** solid, blink, breathe/pulse, rainbow, color cycle,
  chase/theater chase, comet (moving dot with fading tail), sparkle/twinkle, fire,
  gradient scroll.
- **Combinators**, so effects can be combined without writing new ones:
  - `sequence` / playlist: one after another, on a timer
  - `crossfade` transition between two animations
  - `layer` with blend modes
  - per-segment: different effects on different parts of a strip
- Effects may be ported with attribution from MIT-licensed sources such as Adafruit's
  CircuitPython LED_Animation and WS2812FX (MIT).

## API feel (illustrative, not final)

The target reading level for a hobbyist's first program:

```rust
use ledkit::prelude::*;

const LED_PIN: u8 = 8;                  // the strip's data pin
const LED_COUNT: usize = 30;            // number of LEDs on the strip
type LedType = Ws2812b;                 // printed on the strip or in the product listing

let mut strip = Strip::<LedType, LED_COUNT>::new(/* backend + pin */);
strip.set_brightness(0.3);
strip.set_power_limit(Milliamps(1500));

strip.fill(Color::BLUE);
strip.show()?;

let mut rainbow = Rainbow::new().speed(Seconds(5.0));
loop {
    rainbow.render(&mut strip, clock.elapsed());
    strip.show()?;
}
```

Principles: plain-English names, units in the type (`Seconds`, `Milliamps`), sensible
defaults, everything configurable once at the top of the file, and a technical term only
where unavoidable, explained in its docs.

## Documentation and examples (part of the product, not an afterthought)

- **Guide website** (mdBook on GitHub Pages): a step-by-step walkthrough with photos and
  wiring diagrams, starting from a blank machine. It covers both ESP32 setups and the Pico,
  and explains the "bare-metal vs `std`" choice for ESP32 in plain words.
- **Example ladder:** numbered examples that build on each other, for example
  `01_light_one_led`, `02_fill_strip`, `03_rainbow`, `04_breathe`, `05_matrix_text`,
  `06_combine_effects` and `07_power_limit`. Each is heavily commented and runnable as-is.
- **Board examples:** each first-class board (at minimum a Pico 2 and an ESP32-C6/S3
  Feather or DevKit) gets a ready-to-run project, generated from a `cargo generate`
  template, with a board table of LED pins.
- **API docs** on docs.rs with an example on every public type.

## AI-assisted development

Hobbyists will often develop with an AI assistant. The project should make those assistants
give *correct* ledkit answers, not plausible ones mixed up from other crates or from the
`esp-hal` vs `esp-idf` confusion.

- `AGENTS.md` (and a `CLAUDE.md` that points to it) at the repo root: how the crates fit
  together, conventions, and how to add an LED type, backend or effect.
- `llms.txt` on the guide website: a compact, machine-readable map of the docs.
- A **Claude Code plugin** with skills, for example "start a new ledkit project for my
  board", "add an effect", and "debug my strip" (flicker, wrong colors, nothing lights up).
- Docs written so the same material serves humans and AI tools.

## Contributor-friendliness

- A clear crate split, so a contributor can work on one area without learning everything.
- CI: build all crates, run clippy and tests, build examples for each first-class target.
- Host-side unit tests for all logic (encoders, color math, layouts, animations), which
  are possible because the core crates are hardware-free.
- The simulator enables effect development and snapshot tests without hardware.
- `CONTRIBUTING.md` with short guides: "add an LED type", "add a backend", "add an effect".

## Phasing (my proposal)

1. **Phase 1, first release:** `ledkit-color`, `ledkit-core`, `ledkit-spi`, `ledkit-esp`,
   `ledkit-esp-idf`, `ledkit-rp`, `ledkit-layout` (segments + matrices), `ledkit-anim` with
   the core effects, the `ledkit` facade, the guide (both ESP32 setups and the Pico), the
   example ladder, board templates for the first-class boards, `AGENTS.md` and `llms.txt`.
   *Builder's call:* `ledkit-esp-idf` is in phase 1 because it's listed as first-class, the
   owner's current project uses it, and a working ESP-IDF RMT driver already exists in the
   `esp32-c6-pulse-rust` repository to start from.
2. **Phase 1.5:** `ledkit-sim`, the Claude Code plugin, and the remaining effects and combinators.
3. **Phase 2:** HUB75 RGB matrix panels (reusing color, layout and animation), PWM+DMA
   backends for STM32/nRF, and possibly audio-reactive effects.

## Left to the builder

- Exact type and function names, beyond the principles above.
- Which RP HAL(s) `ledkit-rp` targets first (embassy-rp, rp2040-hal, rp235x-hal).
- The simulator's GUI technology.
- Internal module structure, test strategy details, and CI layout.
- Whether `ledkit-layout` and `ledkit-anim` start as one crate and split later.

## Calls made without the owner (please check)

- The name `ledkit`.
- The crate split (above), especially separate backend crates and a facade crate.
- Building on and interoperating with `smart-leds-trait`.
- `no_std` + no-alloc + stable Rust for the core.
- Phase ordering: ESP-IDF backend in phase 1; simulator and Claude plugin in phase 1.5.
- Extra clockless LED types (WS2813, WS2815, SK6805, APA106) in phase 1; legacy clocked
  chips deferred.
- All ESP32 variants, including Xtensa chips with Espressif's toolchain.
- Teensy 4.x and Linux Raspberry Pis listed as "Supported" through `ledkit-spi`.
- The specific effect list and the example ladder.
- Power limiting and temporal dithering as headline features.
