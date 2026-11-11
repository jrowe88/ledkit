# ESP32-S3 DotStar strip: a relaxing light show (SPI)

A slow, calm show for a DotStar (APA102) strip, made for dense strips (144 LEDs per
metre and up), set up for 150 LEDs, on the Adafruit ESP32-S3 Feather. It uses the
`ledkit` crate, and the strip's data and clock come from the chip's SPI peripheral.

> **Runs on hardware** (2026-11-11, Adafruit ESP32-S3 Feather, a dense DotStar strip of
> about 150 LEDs): the show plays, with no stray LEDs once `LED_COUNT` matched the strip.
> With it set to 100, LEDs 101 and 102 lit bright and stuck (see "Troubleshooting").
> If you try it on other hardware, please report what you see.

## What you'll see

Three scenes, 45 seconds each, each fading into the next over 4 seconds, forever:

1. **Aurora**: curtains of green, teal and violet drift along the strip, with soft
   ripples running through them. Stars twinkle in the dark gaps.
2. **Fireflies**: warm amber and yellow-green glows fade in at random places, drift a
   few LEDs, and fade out, over a dim sky that runs from deep blue to plum.
3. **Silk**: a band of rainbow colors slowly turning, folded back and forth by two
   soft waves, with a sheen of light sliding along it every 10 seconds.

Nothing hops from LED to LED: positions are worked out in 1/256ths of an LED, so a
firefly or the sheen glides between them. On a dense strip that makes the movement
look smooth.

## Make it yours

The scenes are in [`src/scenes.rs`](src/scenes.rs), the board setup in
[`src/main.rs`](src/main.rs). Things to try:

- **One scene only**: in `main.rs`, delete the other entries from `scenes`.
- **Timing**: `SCENE_TIME` and `FADE_TIME` in `main.rs`.
- **Colors**: `AURORA_COLORS`, `STAR`, `DUSK`, the colors in `FIREFLIES`, and
  `SILK_SPAN` (how much of the color wheel is on the strip at once).
- **Speeds**: most speeds are "milliseconds per cycle": bigger is slower. In the
  aurora, the numbers that divide `t` set the speed (`t / 40`: bigger is slower).
- **Sizes**: `FIREFLY_RADIUS`, `FIREFLY_DRIFT`, `SHEEN_RADIUS` (in LEDs), and the
  `/ 30`, `/ 13` in the aurora (how many LEDs a color patch and a curtain span).
- **Your own scene**: write a struct with `impl Animation`, whose `color_at` gives
  the color of one LED (`at.x`) at one moment (`frame.millis`), and add it to the list.
  ledkit's own effects (`Rainbow`, `Plasma`, `Comet` and the rest) can go in the
  list too.

## Wiring

The same wiring as [`esp32s3-dotstar-hd`](../esp32s3-dotstar-hd): GPIO5 (data) and
GPIO6 (clock), marked "5" and "6" on the Feather, through a **74AHCT125** level
shifter (the ESP32's 3.3 V signals are too weak for 5 V LEDs to read reliably) to the
strip's **DI** and **CI**:

| From | To |
|---|---|
| ESP32-S3 GPIO5 (data) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | the strip's DI (data in) |
| ESP32-S3 GPIO6 (clock) | 74AHCT125 pin 5 (2A, input) |
| 74AHCT125 pin 6 (2Y, output) | the strip's CI (clock in) |
| 74AHCT125 pins 1 and 4 (1OE, 2OE) | GND (this switches those outputs on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (9, 12) | GND |

- A **5 V** supply to the strip's 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the board, the 74AHCT125 and the strip all share it.
  A missing ground link gives random bright pixels or a dark strip, and looks like a
  software bug.
- Use the strip's **input** end (DI and CI; the arrows on the strip point away from it).

## Power

`BRIGHTNESS` is 25% and `POWER_LIMIT` is `Milliamps(500)`, so the show is safe even
with the strip powered from USB. At this brightness it stays well under the limit. A
150-LED DotStar strip at full white draws about 9 A, so to go much brighter, power the
strip from a 5 V supply rated for it and raise `POWER_LIMIT` with `BRIGHTNESS`.

## One-time setup

The ESP32-S3 has Xtensa cores, which plain Rust can't build for yet. Espressif's
`espup` installs a Rust toolchain that can (named `esp`); this folder's
`rust-toolchain.toml` picks it for you.

```sh
cargo install espup espflash --locked
espup install --targets esp32s3
```

## Run it

Plug the board into its USB port, then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit. If espflash can't connect, hold
**BOOT**, tap **RESET**, let go of BOOT and run again; afterwards press RESET to start
the program.

## Expected serial output

```text
DotStar show on 150 LEDs: aurora, fireflies, silk
```

## Troubleshooting

- **Random colors or flicker everywhere**: data and clock are swapped, a 74AHCT125
  OE pin is floating, or the grounds aren't joined. See "Wiring".
- **Nothing lights up**: check the 74AHCT125's 5 V supply, the common ground, and that
  you used the strip's DI and CI, not DO and CO.
- **Wrong colors** (the aurora's blues come out red, say): `LedType` in `src/main.rs`
  doesn't match your LEDs.
- **One or two LEDs just past the show are bright white (or cyan) and never change**:
  `LED_COUNT` is lower than the number of LEDs on your strip. To end each frame,
  ledkit sends a few bytes of all ones, which give the strip the extra clock pulses
  it needs to pass the data along to the last LED. The LED after the last one
  reads those bytes as its own color: full white at full brightness. Set
  `LED_COUNT` to your strip's length (or a little more).
- **The show only covers part of the strip, or looks squashed**: set `LED_COUNT` to
  the number of LEDs on your strip.
- **Can't flash** ("port busy" or access denied): another program holds the serial
  port. Close any serial monitor and try again.
