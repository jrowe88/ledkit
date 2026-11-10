# Changelog

All the ledkit crates share one version number and are released together. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crates follow
[semantic versioning](https://semver.org/): while the version is 0.x, a minor release
(0.1 to 0.2) may change the API.

## [Unreleased]

The first release, 0.1.0, is being prepared. It will contain:

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
- `ledkit`: the one crate to add, with a prelude and a feature per backend.
