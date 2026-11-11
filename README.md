<div align="center">

<img src="assets/hero.svg" alt="ledkit: a row of LEDs cycling through a rainbow" width="760">

### Addressable LEDs in Rust that just work.

NeoPixels, DotStars, strips, rings and matrices. Name your chip, pick your board,
and ledkit handles the timing, color order, brightness and power.

[![CI](https://github.com/jrowe88/ledkit/actions/workflows/ci.yml/badge.svg)](https://github.com/jrowe88/ledkit/actions/workflows/ci.yml)
![no_std](https://img.shields.io/badge/no__std-yes-informational)
![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![Boards](https://img.shields.io/badge/boards-ESP32%20%C2%B7%20Pico%20%C2%B7%20nRF52%20%C2%B7%20Raspberry%20Pi%20%C2%B7%20Teensy%20%C2%B7%20any%20SPI-orange)

**[Get started](#get-started)** · **[Examples](#pick-your-board)** · **[Supported LEDs](crates/ledkit-core/README.md#which-chip-type-do-i-use)** · **[Roadmap](docs/roadmap.md)**

</div>

## Light up in a dozen lines

```rust
use esp_hal::delay::Delay;
use ledkit::prelude::*;

// An ESP32-C6 with esp-hal, and 30 NeoPixels on GPIO4.
let peripherals = esp_hal::init(esp_hal::Config::default());
let mut strip = ledkit::esp::strip!(Ws2812b, 30, peripherals.RMT, peripherals.GPIO4)?;

strip.set_brightness(0.3);              // looks 30% as bright
strip.set_power_limit(Milliamps(1500)); // caps the LEDs' estimated current, not the board's

let mut frames = Frames::new(ledkit::esp::Clock::start(), Delay::new());
let rainbow = Rainbow::new();
loop {
    let now = frames.next_frame();      // about 60 times a second
    rainbow.render(&mut strip, now);
    strip.show()?;
}
```

Every shortcut here is made of public pieces you can write out yourself; the
[`ledkit` crate's README](crates/ledkit/README.md) shows how.

## Why ledkit

<table>
<tr>
<td width="33%" valign="top">

**Say the chip, not the timings**<br>
Write `Ws2812b` or `Apa102`. ledkit knows each chip's pulse timing and
color order, and checks them against the datasheet.

</td>
<td width="33%" valign="top">

**Brightness that looks right**<br>
`0.5` looks half as bright. Dithering smooths dim fades, and colors go
out exactly as you set them.

</td>
<td width="33%" valign="top">

**Won't cook your USB port**<br>
Set a milliamp limit, and ledkit dims the whole strip just enough to
stay under it.

</td>
</tr>
<tr>
<td valign="top">

**Mistakes caught at build time**<br>
A DotStar on a NeoPixel driver, or an SPI clock your LEDs can't use,
fails to compile with a clear message.

</td>
<td valign="top">

**One API, many boards**<br>
ESP32 (esp-hal or ESP-IDF), Raspberry Pi Pico (RP2040/RP2350), nRF52840,
Raspberry Pi, Teensy 4, or anything with an `embedded-hal` SPI bus or an
`embedded-io` serial port.

</td>
<td valign="top">

**Plays well with others**<br>
Works with the `smart-leds` ecosystem, needs no heap, and supports
blocking or async (Embassy).

</td>
</tr>
</table>

## Pick your board

Every board drives LEDs at least one way. Its own pulse peripheral (RMT, PIO, PWM) is
the usual pick for NeoPixels; SPI drives DotStars, and NeoPixels too; a serial port is a
third way to drive NeoPixels when the others are busy.

| Board | Own peripheral | SPI | Serial port | Start with |
|---|---|---|---|---|
| ESP32-C6, esp-hal | ✅ RMT | ✅ | ✅ | [esp32c6-start](examples/esp32c6-start)* |
| ESP32-S3, esp-hal | ✅ RMT | ✅ ± DMA | ✅ | [esp32s3-check](examples/esp32s3-check) |
| ESP32-C6 / S3, ESP-IDF (`std`, Wi-Fi) | ✅ RMT (± DMA on S3) | | | [esp32s3-start-idf](examples/esp32s3-start-idf)* |
| Raspberry Pi Pico, Feather RP2040 | ✅ PIO ± DMA, async | ✅ | | [rp2040-pio](examples/rp2040-pio) |
| Pico 2 (RP2350, Arm or RISC-V) | ✅ PIO ± DMA, async | ✅ | | [rp2350-pio](examples/rp2350-pio) |
| nRF52840 (Seeed XIAO) | ✅ PWM + EasyDMA, async | ✅ | ✗ (can't invert) | [nrf52840-pwm](examples/nrf52840-pwm) |
| Teensy 4 | | ✅ | ✅ DMA | [teensy4-spi](examples/teensy4-spi) |
| Raspberry Pi (Linux) | | ✅ | | [linux-spidev](examples/linux-spidev) |
| Anything else | | `embedded-hal` SPI | `embedded-io` serial | |
| Crate | `ledkit-esp`, `ledkit-esp-idf`, `ledkit-rp`, `ledkit-nrf` | `ledkit-spi` | `ledkit-uart` | |

✅ = tested on hardware. A blank cell hasn't been tried yet. * builds; not yet run on
hardware. All of these come through the one `ledkit` crate except `ledkit-rp`, which you
add next to it.

### Examples

Each example is a complete project, with the settings you'd change (`LED_COUNT`, the
pin, the LED type, brightness) at the top of `main.rs` and wiring, setup and
troubleshooting in its README.

| Board | Drivers | Matrices, effects, text |
|---|---|---|
| ESP32-C6 | [esp32c6-start](examples/esp32c6-start)*, [esp32c6-rmt](examples/esp32c6-rmt), [esp32c6-esp-idf](examples/esp32c6-esp-idf), [esp32c6-spi](examples/esp32c6-spi), [esp32c6-uart](examples/esp32c6-uart) | [esp32c6-matrix](examples/esp32c6-matrix), [esp32c6-anim](examples/esp32c6-anim), [esp32c6-dotstar-hd](examples/esp32c6-dotstar-hd), [esp32c6-text](examples/esp32c6-text)* |
| ESP32-S3 Feather | [esp32s3-start-idf](examples/esp32s3-start-idf)*, [esp32s3-rmt](examples/esp32s3-rmt), [esp32s3-esp-idf](examples/esp32s3-esp-idf), [esp32s3-check](examples/esp32s3-check) (RMT, SPI ± DMA, UART) | [esp32s3-matrix](examples/esp32s3-matrix), [esp32s3-anim](examples/esp32s3-anim), [esp32s3-dotstar-hd](examples/esp32s3-dotstar-hd), [esp32s3-text](examples/esp32s3-text) |
| RP2040 / RP2350 | [rp2040-pio](examples/rp2040-pio), [rp2350-pio](examples/rp2350-pio) (both with DotStars) | [rp2350-text](examples/rp2350-text)* |
| nRF52840 | [nrf52840-pwm](examples/nrf52840-pwm), [nrf52840-spi](examples/nrf52840-spi), [nrf52840-dotstar](examples/nrf52840-dotstar) | |
| Teensy 4 | [teensy4-spi](examples/teensy4-spi), [teensy4-uart](examples/teensy4-uart) | [teensy4-anim](examples/teensy4-anim) |
| Raspberry Pi | [linux-spidev](examples/linux-spidev) | |
| No LEDs | | [anim-preview](examples/anim-preview) (effects in your terminal) |

## Get started

1. **Find your LED type.** Look for the name printed on the product or in its
   listing, then find it in the
   ["which chip type do I use?"](crates/ledkit-core/README.md#which-chip-type-do-i-use) table.
2. **Copy the example for your board** (the "Start with" column above), and change
   `LED_COUNT`, the pin and the LED type at the top of `main.rs`.
3. **Run it** with `cargo run --release`. If the colors are wrong or nothing
   lights up, the example's README has a troubleshooting section.

ledkit isn't on crates.io yet, so for now depend on it from git or a local
checkout, as the examples do.

## The crates

**Add this one.** It brings in the rest:

| Crate | What it does |
|---|---|
| `ledkit` | Everything re-exported, a prelude, ESP board helpers and a frame loop |

**What you draw with:**

| Crate | What it does |
|---|---|
| `ledkit-color` | Colors, hues, gradients, gamma, palettes |
| `ledkit-core` | The LED chip catalogue, the `Strip` buffer, brightness, dithering and power limiting |
| `ledkit-layout` | Segments, matrices, text and shapes |
| `ledkit-anim` | Effects (rainbow, comet, flames, swirl, …), ways to combine them, and the frame loop |
| `ledkit-text` | Fonts (X11 sizes, tiny, 7-segment, 14-segment), text areas and a scrolling ticker |

**How the LEDs are driven** (pick by board, see [Pick your board](#pick-your-board)):

| Crate | What it does |
|---|---|
| `ledkit-esp` | ESP32 RMT, esp-hal (`no_std`, blocking or async) |
| `ledkit-esp-idf` | ESP32 RMT, ESP-IDF (`std`) |
| `ledkit-rp` | Raspberry Pi Pico (RP2040/RP2350) PIO, processor- or DMA-fed, blocking or async |
| `ledkit-nrf` | nRF52840 PWM + EasyDMA, blocking or async |
| `ledkit-spi` | Any `embedded-hal` SPI bus, blocking or async |
| `ledkit-uart` | NeoPixels from any `embedded-io` serial port with an inverted output, blocking or async (DMA) |

All of these work and are tested on hardware, but aren't published to crates.io yet.
Planned: `ledkit-sim`, a desktop simulator to try effects without hardware.

## Status and roadmap

ledkit is young, but every crate above marked available works and is tested on real
hardware: the core, the SPI and serial drivers, both ESP32 drivers, the Pico PIO
driver, the nRF52840 driver, matrices (`ledkit-layout`), effects (`ledkit-anim`, on an
8x8 matrix and a 100-LED strip) and text (`ledkit-text`, on an 8x8 DotStar; try the
fonts in the [text preview](https://jrowe88.github.io/ledkit/text-preview/)). The `ledkit`
crate ties them together. Next up: the first release on crates.io. See the
[roadmap](docs/roadmap.md) for the details.

Want to help? [CONTRIBUTING.md](CONTRIBUTING.md) has short guides to adding an
LED type, a new board, a panel preset or an effect.

## License

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or
the [MIT license](LICENSE-MIT), at your option. See [NOTICE](NOTICE) for
attributions.
