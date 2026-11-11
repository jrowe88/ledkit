# ledkit

Addressable LEDs in Rust that just work: NeoPixels, DotStars, strips, rings and
matrices. Name your LED type, pick your board, and ledkit handles the timing, the color
order, brightness and power. `no_std`, no heap.

`ledkit` is the one crate to add. It brings in the rest of the family (colors, the chip
catalogue, layouts, effects, text and the board drivers) behind one `use`, and adds
helpers that set a board up in a line, while every step stays yours to write out by hand.

## Add it

Add `ledkit` with your board's feature, next to your board's HAL:

```toml
[dependencies]
ledkit = { version = "0.1", features = ["esp32c6"] }
esp-hal = { version = "1.2", features = ["esp32c6", "unstable"] }
```

| Your board | Feature | Module |
|---|---|---|
| ESP32-C6, S3 (also ESP32, C3, S2) with esp-hal | `esp32c6`, `esp32s3`, ... | [`ledkit::esp`](https://docs.rs/ledkit/latest/ledkit/esp/) |
| ESP32 with ESP-IDF (`std`) | `esp-idf` | [`ledkit::esp_idf`](https://docs.rs/ledkit/latest/ledkit/esp_idf/) |
| nRF52840 | `nrf52840` (and `async`) | `ledkit::nrf` |
| Anything with an `embedded-hal` SPI bus (DotStars too) | `spi` | `ledkit::spi` |
| Anything with an `embedded-io` serial port | `uart` | `ledkit::uart` |

For a Raspberry Pi Pico (RP2040 or RP2350), add
[`ledkit-rp`](https://docs.rs/ledkit-rp) next to `ledkit` (no feature for it yet: its HAL
and esp-hal can't share a dependency graph).

The default features bring in layouts, effects and text. Others: `smart-leds` (work with
the `smart-leds` crates) and `embedded-graphics` (draw shapes and text with
`embedded-graphics`).

## Light up

On an ESP32-C6 with esp-hal, a rainbow on 30 NeoPixels:

```rust,ignore
use esp_hal::delay::Delay;
use ledkit::prelude::*;

const LED_COUNT: usize = 30;
type LedType = Ws2812b; // printed on the strip or in the product listing

let peripherals = esp_hal::init(esp_hal::Config::default());
let mut strip = ledkit::esp::strip!(LedType, LED_COUNT, peripherals.RMT, peripherals.GPIO4)?;
strip.set_brightness(Brightness::from_percent(30));
strip.set_power_limit(Milliamps(1500));

let mut frames = Frames::new(ledkit::esp::Clock::start(), Delay::new());
let rainbow = Rainbow::new();
loop {
    let now = frames.next_frame(); // about 60 times a second
    rainbow.render(&mut strip, now);
    strip.show()?;
}
```

- **`strip!`** sets up the RMT peripheral and gives back a `Strip`. Its docs show the
  same thing written out by hand, for when you want to change a step.
- **`Frames`** waits until the next frame is due and gives the time. Animations are drawn
  from the time, so they run at the right speed however fast the loop goes. Write your
  own loop with `clock.now()` and a delay whenever you like.
- **Async:** `strip!(.., async)` and `strip.show_async().await`, with
  `FramesAsync` for the loop.

## What's in the prelude

`use ledkit::prelude::*` gives you the chip types (`Ws2812b`, `Sk6812Rgbw`, `Apa102`,
...), `Strip`, `Rgb`, `Hsv`, `Brightness`, `Milliamps`, the layout types
(`MatrixLayout`, `Corner`, `Canvas`), the effects (`Rainbow`, `Comet`, `Plasma`, ...),
`Frames`, and the text types (`Font`, `TextArea`, `Ticker`):

```rust
use ledkit::prelude::*;

let panel = MatrixLayout::new(8, 8).starting_at(Corner::BottomLeft);
let mut ticker = Ticker::new(Font::X11_5X8).letters(Rainbow::new());
ticker.set_messages(&["Hello"]);
```

Everything else is a path away: `ledkit::color`, `ledkit::core`, `ledkit::layout`,
`ledkit::anim`, `ledkit::text`, and the board modules.

## The crates underneath

`ledkit` re-exports these, so you don't add them yourself. Each has its own docs with the
details:

| Module | Crate | What it does |
|---|---|---|
| `ledkit::color` | [`ledkit-color`](https://docs.rs/ledkit-color) | Colors, hues, gradients, gamma, palettes |
| `ledkit::core` | [`ledkit-core`](https://docs.rs/ledkit-core) | The chip catalogue, `Strip`, brightness, dithering, the power limit |
| `ledkit::layout` | [`ledkit-layout`](https://docs.rs/ledkit-layout) | Matrices, segments, `Canvas` |
| `ledkit::anim` | [`ledkit-anim`](https://docs.rs/ledkit-anim) | Effects, combinators, the frame loop |
| `ledkit::text` | [`ledkit-text`](https://docs.rs/ledkit-text) | Fonts, text areas, a ticker |
| `ledkit::esp` | [`ledkit-esp`](https://docs.rs/ledkit-esp) | ESP32 RMT, esp-hal |
| `ledkit::esp_idf` | [`ledkit-esp-idf`](https://docs.rs/ledkit-esp-idf) | ESP32 RMT, ESP-IDF |
| `ledkit::nrf` | [`ledkit-nrf`](https://docs.rs/ledkit-nrf) | nRF52840 PWM |
| `ledkit::spi` | [`ledkit-spi`](https://docs.rs/ledkit-spi) | Any SPI bus |
| `ledkit::uart` | [`ledkit-uart`](https://docs.rs/ledkit-uart) | Any serial port |

## Tested on hardware

| Board | What ran | LEDs | Result |
|---|---|---|---|
| Adafruit ESP32-S3 Feather | `esp::strip!` + `esp::Clock` + `Frames` (the `esp32c6-start` program, built for the S3 on GPIO33) | the onboard NeoPixel | pass (2026-11-11), blocking and async (`strip!(.., async)`, `FramesAsync` with an async `OneShotTimer`): a smooth color cycle at 60 frames a second |
| ESP32-C6-DevKitC-1 | `examples/esp32c6-start`, blocking and `--features async` | WS2812B strip (64 of 100 LEDs) through a 74AHCT125 | pass (2026-11-11): a smooth rainbow at 60 frames a second; a faint shimmer at color blends at 20% brightness is low-brightness stepping (it fades at 50%, and turning dithering off barely changes it) |
| Adafruit ESP32-S3 Feather | `examples/esp32s3-text` on the facade (`spi` feature, `Frames` with `restart()` per scene) | Adafruit DotStar HD 8×8 | pass (2026-11-11): all 18 scenes play in order and scroll smoothly, as before the facade |

Not yet run: `examples/esp32s3-start-idf` (the ESP-IDF helpers). If you try `ledkit` on other hardware, please report what you see.

## Examples

Complete projects for real boards are in the
[examples folder](https://github.com/jrowe88/ledkit/tree/main/examples). Start with
`esp32c6-start` (esp-hal) or `esp32s3-start-idf` (ESP-IDF).

## License

Licensed under either of the Apache License, Version 2.0 or the MIT license, at your
option. See `NOTICE` for attributions.
