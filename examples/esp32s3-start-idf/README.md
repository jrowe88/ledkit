# ESP32-S3 first program on ESP-IDF: a rainbow with the `ledkit` crate

The shortest complete ledkit program for an ESP32-S3 with ESP-IDF (`std`): a rainbow on
a NeoPixel strip, about 60 frames a second. It depends only on `ledkit` and esp-idf-hal:

- `ledkit::esp_idf::strip` sets up the RMT peripheral and gives back a `Strip`
  (`strip_with_dma` lets the RMT read the frame through DMA, which only the S3 has).
- `Frames` waits until each frame is due and gives the time to draw it at.

> **Passes on hardware** (2026-11-11, Adafruit ESP32-S3 Feather, a 100-LED WS2812B strip
> through a 74AHCT125, as in [esp32s3-esp-idf](../esp32s3-esp-idf)): a smooth rainbow
> along the whole strip. If you try it on other hardware, please report what you see.

## What you'll see

A rainbow moving along the strip, at 20% brightness. The serial monitor prints
`Rainbow on 100 LEDs, ESP-IDF`.

## Wiring

| Feather pin | Goes to |
|---|---|
| 5 (GPIO5) | 74AHCT125 1A; 1Y to the strip's DIN (data in) |
| GND | the strip's GND and the 74AHCT125's GND |

Power the strip from 5 V. Tie the 74AHCT125's 1OE to GND and power it from 5 V.

## Settings

At the top of `src/main.rs`: `LedType`, `LED_COUNT`, `BRIGHTNESS`, `POWER_LIMIT` and
`FRAMES_PER_SECOND`. To use another pin, change `gpio5`.

## Run it

Install the `esp` toolchain once with `espup install`, and `ldproxy` with
`cargo install ldproxy`. Then:

```sh
cargo run --release
```

The first build downloads ESP-IDF and takes a while. On Windows the build goes to
`C:/t/ledkit-s3` (see `.cargo/config.toml`); on macOS and Linux set
`CARGO_TARGET_DIR=target`.
