# ESP32-C6 first program: a rainbow with the `ledkit` crate (esp-hal)

The shortest complete ledkit program for an ESP32-C6 with esp-hal: a rainbow on a strip
or matrix of NeoPixels, about 60 frames a second. It depends only on `ledkit` and
esp-hal:

- `ledkit::esp::strip!` sets up the RMT peripheral and gives back a `Strip`.
- `Frames` waits until each frame is due and gives the time to draw it at.

Build it with `--features async` for the same program written with `async`/`.await`
(`strip!(.., async)`, `show_async().await` and `FramesAsync`).

> **Passes on hardware** (2026-11-11, ESP32-C6-DevKitC-1, a WS2812B strip through a
> 74AHCT125): a smooth rainbow on the first 64 LEDs, blocking and async. If you try it on
> other hardware, please report what you see.

## What you'll see

A rainbow moving along the LEDs, at 20% brightness. The serial monitor prints
`Rainbow on 64 LEDs, blocking` (or `async`).

Look closely where one color blends into the next and you may see a faint shimmer. That's
normal at low brightness: at 20% each color has only about 50 steps, and a slow blend
moves through them visibly. It fades as you turn the brightness up. (Dithering, on by
default, makes little difference here.)

## Wiring

| DevKit pin | Goes to |
|---|---|
| GPIO4 | 74AHCT125 1A; 1Y to the LEDs' DIN (data in) |
| GND | the LEDs' GND and the 74AHCT125's GND |

Power the LEDs from 5 V. Tie the 74AHCT125's 1OE to GND and power it from 5 V. A short
strip often works without the level shifter; if the first LED flickers, add it.

## Settings

At the top of `src/main.rs`: `LedType`, `LED_COUNT`, `BRIGHTNESS`, `POWER_LIMIT` and
`FRAMES_PER_SECOND`. To use another pin, change `peripherals.GPIO4`.

## Run it

```sh
cargo run --release
cargo run --release --features async
```

`ledkit::esp::strip!` is a shortcut. Its docs show the same setup written out by hand
(`Rmt::new`, `RmtOutput::new` and `Strip::new`), for when you want to change a step.
