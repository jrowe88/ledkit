# Every animation on a Teensy 4

Plays every `ledkit-anim` effect, one after another, on an 8x8 matrix or a strip
driven from a Teensy 4.0 or 4.1. Handy as a hardware test of the whole animation
library. The LED data goes out on pin 11 through the SPI hardware, wired exactly
as in the [teensy4-spi](../teensy4-spi) example.

> **Tested on hardware (2026-11-02):** Teensy 4.0 and an Adafruit NeoPixel 8x8
> through a 74AHCT125: the check and every animation and transition look right.
> The strip build (`--features strip`) hasn't been tried on hardware yet.

## What you'll see

1. **The check**: the whole panel red, green, blue, then white, one second each.
   On the matrix, then 3 seconds of the top row red and the left column green,
   yellow where they meet in the top-left corner. If they show up anywhere else,
   `LAYOUT` in `src/main.rs` doesn't match how your panel is wired or held.
2. **The show**: 6 seconds each, with default settings, in this order:

   | # | Animation | Comes in with |
   |---|---|---|
   | 1 | solid (white) | fade |
   | 2 | blink | wipe from the left |
   | 3 | breathe | dissolve |
   | 4 | rainbow | iris |
   | 5 | color cycle | fade through black |
   | 6 | gradient scroll | wipe from the top |
   | 7 | color wipe | cut |
   | 8 | chase | fade |
   | 9 | comet | wipe from the left |
   | 10 | sparkle | dissolve |
   | 11 | flames | iris |
   | 12 | swirl | fade through black |
   | 13 | rings | wipe from the top |
   | 14 | pinwheel | cut |
   | 15 | plasma | fade |
   | 16 | rain | wipe from the left |
   | 17 | a white comet over a dim rainbow (a layer) | dissolve |
   | 18 | a rainbow, reversed, half speed, mirrored | iris |

   Then it starts again from 1 (about 2 minutes a round). The first round starts
   with solid straight away; later rounds fade into it.

Brightness is 20%, limited to 500 mA, which is safe on USB power.

## Wiring

| From | To |
|---|---|
| Teensy pin 11 (MOSI) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | 330 ohm resistor, then the LEDs' DIN |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (4, 10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (5, 9, 12) | GND |
| Teensy GND | LED GND (one common ground for everything) |

The Teensy 4's pins are **not** 5 V tolerant, so never connect a 5 V signal to
one. Pins 12 and 13 are claimed by the SPI setup but not used; pin 13 is also the
orange LED, which glows while data is sent.

## Build and load

You need the same one-time setup as the [teensy4-spi](../teensy4-spi) example
(the `thumbv7em-none-eabihf` target, `cargo-binutils`, and the Teensy Loader or
`teensy_loader_cli`). In this folder, for the 8x8 matrix:

```sh
cargo build --release
cargo objcopy --release -- -O ihex teensy4-anim-matrix.hex
```

For a 100-LED strip, add `--features strip` to both commands:

```sh
cargo build --release --features strip
cargo objcopy --release --features strip -- -O ihex teensy4-anim-strip.hex
```

Load the `.hex` with the Teensy Loader (File, Open HEX File, then press the
button on the board), or with
`teensy_loader_cli --mcu=TEENSY40 -w -v teensy4-anim-matrix.hex` (use
`--mcu=TEENSY41` for a Teensy 4.1).

## Settings

At the top of `src/main.rs`: `LAYOUT` (how the matrix is wired), `STRIP_LEDS`
(100), `LedType` (`Ws2812b`), `BRIGHTNESS`, `POWER_LIMIT`, `STEP` (how long each
animation plays) and `CHANGE` (how long each transition takes). On a Teensy 4.1
or MicroMod, change `board::t40` to `board::t41` or `board::tmm`.

## How it keeps time

Animations are drawn from the time since the start, so they run at the right
speed however long a frame takes. The time comes from the chip's general purpose
timer GPT1, counting the 1 MHz peripheral clock divided by 1000: its count is
the milliseconds since the show started.

For trouble with colors, the first LED or flicker, see the troubleshooting list
in the [teensy4-spi](../teensy4-spi) example. Everything about the wiring and
the SPI setup is the same.
