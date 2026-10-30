# Teensy 4 LED strip

Drives a WS2812B ("NeoPixel") strip from a Teensy 4.0 or 4.1 with `ledkit-core`
and `ledkit-spi`. The strip's data wire is fed from the chip's SPI hardware
(LPSPI4), so no special LED peripheral is needed.

> **Pending hardware test.** This example builds in CI, but it has not been
> run on a real Teensy 4 yet. If you try it, please report what you see.

## What you'll see

1. The whole strip red, then green, blue, then white, one second each, dimly
   lit (20% brightness).
2. A rainbow that moves along the strip, forever.

The orange LED on the Teensy glows while the program runs. Pin 13 is the SPI
clock, and it shares that pin. This is harmless.

## Wiring

The Teensy 4's pins work at 3.3 V and are **not** 5 V tolerant. A strip wants a
5 V data signal, so use a level shifter. A 74AHCT125 is a good choice, powered
from 5 V.

| From | To |
|---|---|
| Teensy pin 11 (MOSI) | 74AHCT125 pin 1A (input) |
| 74AHCT125 pin 1Y (output) | 330 ohm resistor, then the strip's DIN |
| 74AHCT125 pin 1OE | GND (this switches the output on) |
| 74AHCT125 VCC | 5 V |
| 74AHCT125 GND | GND |
| 74AHCT125 other OE pins (2OE, 3OE, 4OE) | 5 V (switched off) |
| 74AHCT125 other inputs (2A, 3A, 4A) | GND |
| Teensy GND | strip GND (one common ground for everything) |

Power the strip from a 5 V supply. **Never** power it from the Teensy's 3.3 V
pin. The program limits the strip to 500 mA, which is safe for USB power. If
you give the strip its own 5 V supply, join that supply's ground to the
Teensy's ground, and raise `POWER_LIMIT` in `src/main.rs`.

Pin 12 (MISO) is claimed by the SPI setup but not used. Leave it unconnected.

## One-time setup

```sh
rustup target add thumbv7em-none-eabihf
cargo install cargo-binutils
rustup component add llvm-tools
```

You also need a way to load the program onto the Teensy: either the
[Teensy Loader](https://www.pjrc.com/teensy/loader.html) application (it comes
with Teensyduino) or the command-line
[`teensy_loader_cli`](https://github.com/PaulStoffregen/teensy_loader_cli).

## Build and load

In this folder:

```sh
cargo build --release
cargo objcopy --release -- -O ihex teensy4-spi.hex
```

The first command builds the program. The second converts it to a `.hex` file,
the format the Teensy loader understands.

Then load it, either way:

- **Teensy Loader app**: open it, choose File, Open HEX File, pick
  `teensy4-spi.hex`, plug in the Teensy, and press the white button on the
  board (or click Program in the app). The loader programs it and reboots it.
- **Command line**: `teensy_loader_cli --mcu=TEENSY40 -w -v teensy4-spi.hex`
  (use `--mcu=TEENSY41` for a Teensy 4.1), then press the button on the board.

The strip should light up right after the Teensy reboots.

## How fast the SPI really runs

`SPI_HZ` asks for 3.2 MHz, but the Teensy makes its SPI clock by dividing a
132 MHz clock by an even number. The closest it can get without going over is
132 MHz / 42 = 3,142,857 Hz. `src/main.rs` works that number out and gives it
to ledkit, so ledkit builds its pulses for the speed the bus really runs at.
Being 1.8% slow is well within what a WS2812B accepts.

`actual_spi_hz` in `src/main.rs` mirrors how imxrt-hal 0.6 rounds that divider,
so check it again if you change the HAL version.

## Notes

- **Long strips are fine.** The Teensy's SPI driver sends at most 512 bytes
  per write, so the example tells ledkit to send longer strips in several
  writes (`with_max_write_len(512)`). Change `LED_COUNT` to any length.
- The example sets up LPSPI4 for a Teensy 4.0. On a Teensy 4.1 or MicroMod,
  change `board::t40` to `board::t41` or `board::tmm` in `src/main.rs`.

## Troubleshooting

- **Wrong colors** (for example red shows as green): `LedType` in
  `src/main.rs` doesn't match your LED. Try another chip type from the
  `ledkit_core` catalogue.
- **The first LED is wrong, or the strip flickers**: check the level shifter,
  the 330 ohm resistor and the common ground. These are the usual causes.
- **Nothing lights up**: check that the data goes to the strip's DIN (not
  DOUT), that you used pin 11, that the grounds are joined, and that the strip
  has 5 V power.
- **The program doesn't start after loading**: press the board's button again
  and make sure the loader says the upload finished.

## How it fits together

`src/main.rs` is commented step by step. The settings at the top (`LED_COUNT`,
`LedType`, `SPI_HZ`, `BRIGHTNESS`, `POWER_LIMIT`) are the ones you are most
likely to change. `.cargo/config.toml` tells the linker which memory layout to
use for the Teensy 4.
