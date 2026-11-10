# ledkit-esp-idf

A ledkit backend that drives NeoPixels with the ESP32's RMT peripheral, through
`esp-idf-hal` (the `std` flavour of Rust on ESP32, running on top of ESP-IDF).

These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
and the pulse widths tell `0` from `1`. The RMT (Remote Control Transceiver)
peripheral generates these pulses in hardware, so the timing is unaffected by Wi-Fi
interrupts or task scheduling.

`ledkit-esp-idf` is for projects built on ESP-IDF. For bare-metal projects with
`esp-hal`, use `ledkit-esp` instead.

## Which ESP crate do I use?

| Your project | Use |
|---|---|
| esp-hal: bare metal, `no_std`, stable Rust, works with Embassy | `ledkit-esp` |
| ESP-IDF: `std`, Wi-Fi through `esp-idf-svc`, nightly Rust | `ledkit-esp-idf` |
| Any board with an SPI bus (including ESP32 boards, no RMT needed) | `ledkit-spi` |

## Quick tour

This can only run on an ESP32, so the snippet is not run by `cargo test`:

```rust
use esp_idf_hal::peripherals::Peripherals;
use ledkit_core::prelude::*;
use ledkit_esp_idf::RmtOutput;

const LED_COUNT: usize = 8;

esp_idf_hal::sys::link_patches();
let peripherals = Peripherals::take().unwrap();

// The C6 DevKit's onboard LED is on GPIO8; use your strip's data pin.
let output = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8).unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Rgb::BLUE);
strip.show().unwrap();
```

## Wiring

One data wire from a GPIO pin to the LED's data-in, a shared ground, and 5 V power
for the LEDs. The board's 3.3 V signal is fine for most WS2812B strips; if the first
LED flickers, add a level shifter. On the ESP32-C6-DevKitC-1 the onboard LED is on
GPIO8.

`RmtOutput::new` takes the pin and asks ESP-IDF for any free RMT transmit channel,
so you never pick a channel number. You only choose the LED type and the pin.

On the ESP32-S3, `RmtOutput::new_with_dma` feeds the RMT through DMA instead, which
keeps long strips clean even when the processor is busy. Other chips (except the
ESP32-P4) have no RMT DMA, and there it returns an error.

## The reset gap

The LEDs latch the new colors once the data line has been idle for the chip's reset
time (280 us for the WS2812B). `RmtOutput` remembers when the last frame ended and,
if `show()` is called again too soon, waits out the rest of the gap before sending.
Two `show()` calls back to back therefore both take effect, instead of the second
frame being glued onto the first and showing the wrong colors. The wait uses
`std::thread::sleep`. Waits shorter than one FreeRTOS tick are busy-waited
precisely; only a chip with a reset of a tick or more would yield to the
scheduler, which is still safe.

## Coming from `smart-leds`?

With the `smart-leds` feature, `RmtOutput` implements `smart-leds`'
`SmartLedsWrite`, so code written for `ws2812-esp32-rmt-driver` needs only a new
constructor. Colors are sent exactly as given: no brightness, gamma or dithering is
added, so keep using `smart-leds`' own `brightness()` and `gamma()` adaptors.

```rust
use ledkit_core::Ws2812b;
use ledkit_esp_idf::RmtOutput;
use smart_leds_trait::{SmartLedsWrite, RGB8};

let mut ws = RmtOutput::<Ws2812b>::new(peripherals.pins.gpio8).unwrap();
ws.write([RGB8::new(255, 0, 0); 8]).unwrap();
```

## Building

This crate only builds for ESP-IDF targets (for example `riscv32imac-esp-espidf`), with
the setup `esp-idf-sys` needs: a nightly Rust with `rust-src`, `ldproxy`, and a
`.cargo/config.toml` that sets the target, `build-std` and `ESP_IDF_VERSION`.
Copy the setup of `examples/esp32c6-esp-idf` (RISC-V chips) or
`examples/esp32s3-esp-idf` (the Xtensa ESP32-S3); their READMEs walk through it.
On Windows, keep the build folder short (the example's `.cargo/config.toml` sets
`target-dir = "C:/t/ledkit"`), because `esp-idf-sys` rejects output paths over 88 characters.

It depends on the published `esp-idf-hal` 0.47 from crates.io and needs no
`[patch.crates-io]` section. ESP-IDF projects that already depend on `esp-idf-svc` can
use it as is, as long as both resolve to the same `esp-idf-hal`.

## Long strips and an esp-idf-hal 0.47 bug

ESP-IDF's RMT driver fills the RMT memory in blocks while it sends (on the
ESP32-C6, 96 symbols and then 48 at a time). When a frame's last bits fill a
block exactly, the encoder reports "complete" and "memory full" together.
esp-idf-hal 0.47 has no value for that combination and panics ("Unknown
rmt_encode_state_t value: 3"), which aborts the program. On the ESP32-C6 that hit
every WS2812B frame of 4 or more LEDs whose bit count is a multiple of 48, such as
100 LEDs.

`RmtOutput` works around it: when a frame has an even number of bytes, it adds one
`0x00` byte after the last LED. Every block size is a multiple of 16 bits and an
odd number of bytes never is, so a frame never ends exactly on a block. The LEDs
ignore the extra byte (an LED only takes a color from a full 24 or 32 bits). The
workaround can go once an esp-idf-hal release after 0.47.0 includes the upstream
fix ([esp-rs/esp-idf#600](https://github.com/esp-rs/esp-idf/pull/600), merged but
not yet released).


## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| ESP32-C6 dev kit (chip rev v0.2), 2026-10-30 | the onboard LED | pass: colors in order, back-to-back frames latch |
| ESP32-C6 dev kit (chip rev v0.2), 2026-10-30 | the onboard LED, slow rainbow | flickered with dithering on (20% brightness, about 45 frames a second), steady with it off; the example now turns it off, see its troubleshooting |
| ESP32-C6, the pulse project (first consumer) | its status LED | pass (2026-10-30): breathes as before after pulse raised its own minimum brightness |
| ESP32-C6 dev kit, 2026-10-31 | WS2812-type strip, 100 LEDs, through a 74AHCT125 | runs after the padding fix below: frames of every length from 1 to 150 LEDs, then colors, a counting pattern and a rainbow on 100 LEDs, with no crash, and all 100 LEDs look right. Before the fix, every frame of 4 or more LEDs whose bit count was a multiple of 48 crashed |
| Adafruit ESP32-S3 Feather (chip rev v0.2, ESP-IDF v5.5.3), 2026-11-09 | WS2812B strip, 100 LEDs, data straight from GPIO5 on a 4.3 V supply | pass (`examples/esp32s3-esp-idf`): `new` and `new_with_dma`, each with Wi-Fi off and with Wi-Fi scanning nonstop: colors, back-to-back frames, counting, a rainbow, and every frame length from 1 to 150 LEDs with no panic |

The example is `examples/esp32c6-esp-idf` in the repository. If you try it on your
hardware, please report what you see.

## Features

- `smart-leds`: the `smart-leds` write trait for `RmtOutput`.
