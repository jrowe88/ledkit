# Feather RP2040 SCORPIO hardware check: `ledkit-rp` every way, plus DotStars

Drives the onboard NeoPixel of an Adafruit Feather RP2040 SCORPIO (#5650), then one
WS2812B ("NeoPixel") strip every way `ledkit-rp` can feed the PIO, one after another
on the same pin, then a DotStar (APA102) strip over SPI with `ledkit-spi`.

> **Tested on hardware (2026-11-08):** round 0 and rounds 1–4 with the length sweep pass on
> a SCORPIO with a 100-LED strip. The DotStar round is still to be tried.

## What you'll see

**Round 0, the onboard NeoPixel** (GPIO4, its own state machine, fed by the
processor): red, green, blue, white, one second each; red then blue back to back (it
must end **blue**); then a 10-second rainbow. It then goes dark.

There is no serial console, so each later round announces itself on the strip: the
first 1 to 5 LEDs **blink magenta three times** for round 1 to 5.

| Round | Output | How the data gets to the PIO |
|---|---|---|
| 1 | `PioOutput`, blocking | the processor pushes each LED into the state machine's FIFO |
| 2 | `PioDmaOutput`, blocking | DMA channel 0 streams the whole frame |
| 3 | `PioDmaOutput`, async | the same, awaited (`embassy_futures::block_on`) |
| 4 | `PioOutput`, async | the processor, yielding whenever the FIFO is full |
| 5 | DotStars: `ledkit-spi` `ClockedSpi` on SPI0 | the SPI peripheral, 4 MHz |

Rounds 1 to 4 (NeoPixels), about 21 seconds each:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.

After round 4, the **sweep**: DMA frames of every length from 1 to 150 LEDs, a dim
blue bar that grows along the strip. Then the NeoPixels go dark.

Round 5 shows the same pattern on the DotStars (64 by default, `DOTSTAR_COUNT`), then
a **clock sweep**: the counting pattern at 1, 2, 4, 8, 12, 16, 20, 24 and 32 MHz,
2 seconds each (the RP2040 divides 125 MHz, so the real clock is the nearest it can
make at or below each). Last, the NeoPixel rainbow runs forever on DMA. Press
**RESET** to run it all again.

A round **passes** when every color is right through to the last LED, the
back-to-back frames end blue, the counting marks are on LEDs 0, 10, 20 … 90 and 99,
and the rainbow moves smoothly. In the clock sweep, note the fastest clock at which
the counting pattern is still exact.

## Wiring

The SCORPIO lifts GPIO16 to GPIO23 to 5 V on its 8-channel NeoPixel header, so no
level shifter is needed:

| From | To |
|---|---|
| NeoPixel header channel **16** (GPIO16) | the NeoPixel strip's DIN |
| NeoPixel header channel **19** (GPIO19, SPI0 TX) | DotStar DI (data in) |
| NeoPixel header channel **18** (GPIO18, SPI0 SCK) | DotStar CI (clock in) |
| header GND | both strips' GND |

Power the strips from the SCORPIO's terminal block or their own 5 V supply, and join
the grounds. The program limits each strip to 500 mA (`POWER_LIMIT`). No DotStars?
Leave channels 18 and 19 empty; round 5 still runs, with nothing to see. Check the
pins against Adafruit's SCORPIO pinout before wiring.

## One-time setup

```sh
rustup target add thumbv6m-none-eabi
```

Install **picotool** (prebuilt binaries in the
[pico-sdk-tools releases](https://github.com/raspberrypi/pico-sdk-tools/releases)), or
`cargo install elf2uf2-rs --locked` and swap the `runner` line in
`.cargo/config.toml`.

## Run it

Hold **BOOT**, tap **RESET**, and let go of BOOT: the board shows up as a drive called
`RPI-RP2`. Then:

```sh
cargo run --release
```

That builds the program and loads it (picotool, or elf2uf2-rs copying a UF2 to the
drive). With a debug probe on the SWD pads, `probe-rs run --chip RP2040
target/thumbv6m-none-eabi/release/rp2040-pio` works too.

## Settings

At the top of `src/main.rs`: `LED_COUNT`, `SWEEP_MAX`, `LedType` (use `Sk6812` or
`Sk6812Rgbw` for those strips; the onboard NeoPixel uses the same type),
`DotStarType` (`Sk9822` for SK9822 strips), `DOTSTAR_COUNT`, `DOTSTAR_HZ`,
`DOTSTAR_CLOCKS`, `BRIGHTNESS` and `POWER_LIMIT`.

## Troubleshooting

- **Nothing lights**: check that the load worked (the drive disappears and the board
  restarts) and the grounds are joined.
- **Colors swapped (red shows green)**: `LedType` doesn't match the strip.
- **Round 1 or 4 shows a stray flash on long strips**: the processor fell behind the
  FIFO; rounds 2 and 3 (DMA) should be clean. Please report it.

## Status

Onboard NeoPixel and NeoPixel strip: **pass** (2026-11-08) on a Feather RP2040 SCORPIO with a 100-LED WS2812-type strip on output 0 (GPIO16): round 0, all four PIO rounds and the 1–150 LED sweep looked right. DotStars (round 5): pending hardware test.

The board's RESET button didn't always restart it during the test (holding it longer usually did; it worked as part of BOOT + RESET); replugging USB always does. It looked like the button rather than the program, but a known-good UF2 hasn't been tried to confirm.
