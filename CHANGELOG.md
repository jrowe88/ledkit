# Changelog

All the ledkit crates share one version number and are released together. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow
[semantic versioning](https://semver.org/): while the version is 0.x, a minor release
(0.1 to 0.2) may change the API.

## [Unreleased]

## [0.1.0] - 2026-11-11

The first release. It contains:

- `ledkit-color`: RGB and RGBW colors, hues, gradients, gamma, color temperature, named
  colors and palettes.
- `ledkit-core`: the LED chip catalogue (WS2812B, SK6812, APA102, SK9822 and more),
  color orders, the `Strip` buffer, perceptual brightness, dithering, color correction,
  the power limit, the `Output` and `OutputAsync` traits, and `smart-leds` interop.
- `ledkit-layout`: segments, matrices, `Canvas`, and an `embedded-graphics` draw target.
- `ledkit-anim`: the `Animation` trait, 16 effects, combinators (layers, crossfades,
  playlists, segments) and easing.
- `ledkit-text`: X11 bitmap fonts, a Tiny 3x5, 7- and 14-segment faces at any height,
  text areas and a scrolling ticker.
- Backends: `ledkit-spi` (any `embedded-hal` SPI bus), `ledkit-uart` (any `embedded-io`
  serial port), `ledkit-esp` (ESP32 RMT, esp-hal), `ledkit-esp-idf` (ESP32 RMT,
  ESP-IDF), `ledkit-rp` (RP2040/RP2350 PIO) and `ledkit-nrf` (nRF52840 PWM).
- `ledkit`: the one crate to add, with a prelude and a feature per backend, and board
  helpers for the ESP32: `esp::strip!` (esp-hal, blocking or async) and `esp_idf::strip`
  (ESP-IDF), each with a `Clock`. Every helper is made of public pieces you can write out
  by hand.
- `Frames` and `FramesAsync` (in `ledkit-anim`): a frame loop that waits until the next
  frame is due and gives the time, from any clock and any `embedded-hal` delay.
- Examples for real boards (ESP32-C6, ESP32-S3, RP2040, RP2350, nRF52840, Teensy 4,
  Raspberry Pi), each with a hardware status line; start with `esp32c6-start` (esp-hal)
  or `esp32s3-start-idf` (ESP-IDF).
