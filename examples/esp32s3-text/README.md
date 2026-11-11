# ESP32-S3 text tour on an 8x8 DotStar (SPI)

A tour of `ledkit-text` on an 8x8 DotStar (APA102) panel: every font that fits 8 rows,
the smoothing (edge) settings, outlines, backgrounds, reversed text and effect-colored
letters, one scene at a time. Use it to pick a look for your own project.

> **Runs on hardware** (2026-11-10, Adafruit ESP32-S3 Feather, Adafruit DotStar HD 8x8):
> all 18 scenes play in order. Moved to the `ledkit` crate and `Frames` on 2026-11-11 and
> checked again: the same. If you try it on other hardware, please report what you
> see.

## What you'll see

Each scene's number shows in dim white for a second, then the scene scrolls its text
for 9 seconds. After scene 18 it starts again.

| # | Font | What to look at |
|---|---|---|
| 1 | 4x6 | the smallest X11 font, exactly as drawn |
| 2 | 4x6 at 1.25x, medium edges | smoothed (anti-aliased) to fill all 8 rows |
| 3 | 4x6 at 1.25x, soft edges | blurrier smoothing |
| 4 | 4x6 at 1.25x, very crisp edges | smoothing that stays close to on/off |
| 5 | 5x7 | exactly as drawn |
| 6 | 5x8 | exactly as drawn, all 8 rows |
| 7 | Tiny 3x5 | capitals and digits, 1 LED between letters |
| 8 | Tiny 3x5 at 1.5x | smoothed up to 8 rows |
| 9 | 7-segment, 8 rows | a clock, smoothed |
| 10 | 7-segment, 8 rows, hard edges | each LED fully on or off |
| 11 | 7-segment slanted, 8 rows | leaning right |
| 12 | 14-segment, 8 rows | every capital letter |
| 13 | 5x7, blue outline | a one-LED outline round white letters |
| 14 | 4x6, dark blue behind | a plain background color |
| 15 | 5x7, black letters on Plasma | the background is an effect |
| 16 | 5x7, reversed | letters and background swapped (white on red becomes red on white) |
| 17 | 5x8, Rainbow letters | the letters are an effect |
| 18 | 4x6, Plasma letters, white outline | effect letters with an outline |

The serial monitor prints `scene 1`, `scene 2` and so on as they start.

## Wiring

| Feather pin | Goes to |
|---|---|
| 5 (GPIO5, SPI MOSI) | 74AHCT125 1A; 1Y to the panel's DI (data in) |
| 6 (GPIO6, SPI SCK) | 74AHCT125 2A; 2Y to the panel's CI (clock in) |
| GND | the panel's GND and the 74AHCT125's GND |

Power the panel from 5 V. Tie 1OE and 2OE of the 74AHCT125 to GND, and power it from 5 V.

## Settings

At the top of `src/main.rs`:

- `LAYOUT`: how the panel is wired. If the text comes out mirrored or upside down,
  change the corner.
- `BRIGHTNESS`, `POWER_LIMIT`, `SCROLL_SPEED`, `SCENE_MS`.
- `scene()`: each scene's font and paints. Change them to try your own.

## Run it

Install the `esp` toolchain once with `espup install`, then:

```sh
cargo run --release
```

If `espflash` can't connect, hold BOOT and tap RESET on the Feather.
