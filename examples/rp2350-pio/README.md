# Pico 2 W hardware check: every way `ledkit-rp` drives a strip, plus DotStars

Drives one WS2812B ("NeoPixel") strip from a Raspberry Pi Pico 2 W (RP2350) every way
`ledkit-rp` can feed the PIO, one after another, on the same pin, then a DotStar
(APA102) strip over SPI with `ledkit-spi`. The same program builds for the RP2350's
**Arm** cores and its **RISC-V** cores; run it once on each.

> **Pending hardware test.** This example builds in CI for both cores, but it has not
> been run on a real Pico 2 W yet. If you try it, please report what you see, round by
> round and core by core.

## What you'll see

There is no serial console (a UF2 flash has none), so each round announces itself on
the strip: the first 1 to 5 LEDs **blink magenta three times** for round 1 to 5.

| Round | Output | How the data gets to the PIO |
|---|---|---|
| 1 | `PioOutput`, blocking | the processor pushes each LED into the state machine's FIFO |
| 2 | `PioDmaOutput`, blocking | DMA channel 1 streams the whole frame |
| 3 | `PioDmaOutput`, async | the same, awaited (`embassy_futures::block_on`) |
| 4 | `PioOutput`, async | the processor, yielding whenever the FIFO is full |
| 5 | DotStars: `ledkit-spi` `ClockedSpi` on SPI0 | the SPI peripheral, 4 MHz |

Rounds 1 to 4 (NeoPixels), about 21 seconds each:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**; if it stays
   red, the gap between frames wasn't kept.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.

After round 4, the **sweep**: DMA frames of every length from 1 to 150 LEDs, a dim
blue bar that grows along the strip. Then the NeoPixels go dark.

Round 5 shows the same pattern on the DotStars (64 by default, `DOTSTAR_COUNT`),
then a **clock sweep**: the counting pattern at 1, 2, 4, 8, 12, 16, 20, 24 and 32 MHz, each
after a short dark gap and with its step number shown as LEDs 1 to 9 in green (step 1
is 1 MHz, step 9 is 32 MHz). Last, the NeoPixel rainbow runs forever on DMA. Unplug and
replug (or reset with the RUN pin) to run it all again.

A round **passes** when every color is right through to the last LED, the
back-to-back frames end blue, the counting marks are on LEDs 0, 10, 20 … 90 and 99,
and the rainbow moves smoothly with no flashes or stray colors. In the clock sweep,
note the fastest clock at which the counting pattern is still exact.

The program uses **PIO1**, state machine 0, and **DMA channel 1**, leaving PIO0 and
DMA channel 0 to the Wi-Fi chip's usual driver. The radio is not switched on here
(rp-hal has no driver for it); the Wi-Fi check (T11) waits for an Embassy version.

## Wiring

Everything through one 74AHCT125, which lifts the Pico's 3.3 V signals to the 5 V the
strips expect:

| From | To |
|---|---|
| Pico **GP16** (pin 21) | 74AHCT125 pin 2 (1A) |
| 74AHCT125 pin 3 (1Y) | 330 ohm resistor, then the NeoPixel strip's DIN |
| Pico **GP19** (pin 25, SPI0 TX) | 74AHCT125 pin 5 (2A) |
| 74AHCT125 pin 6 (2Y) | DotStar DI (data in) |
| Pico **GP18** (pin 24, SPI0 SCK) | 74AHCT125 pin 9 (3A) |
| 74AHCT125 pin 8 (3Y) | DotStar CI (clock in) |
| 74AHCT125 pins 1, 4, 10 (1OE, 2OE, 3OE) | GND (this switches the outputs on) |
| 74AHCT125 pin 14 (VCC) | Pico **VBUS** (pin 40), 5 V from USB |
| 74AHCT125 pin 7 (GND) | GND |
| Pico GND (pin 38) | both strips' GND and the 5 V supply's GND |

Power the strips from their own 5 V supply, not from the Pico, and join the grounds.
The program limits each strip to 500 mA (`POWER_LIMIT`). No DotStars? Leave GP18 and
GP19 unconnected; round 5 still runs, with nothing to see. Check the pin numbers
against the Pico 2 W pinout card before wiring.

## One-time setup

```sh
rustup target add thumbv8m.main-none-eabihf riscv32imac-unknown-none-elf
```

Install **picotool** (Raspberry Pi's loader; it makes RP2350 UF2 files with the right
family ID). Prebuilt Windows, macOS and Linux binaries are in the
[pico-sdk-tools releases](https://github.com/raspberrypi/pico-sdk-tools/releases); put
`picotool` on your `PATH`. On Windows, if picotool can't see the board in BOOTSEL
mode, install the WinUSB driver for "RP2 Boot" with Zadig.

## Run it

Hold **BOOTSEL** while plugging in the USB cable, then:

```sh
cargo run --release
```

That builds for the **Arm** cores and loads it with picotool (see `.cargo/config.toml`).
For the **RISC-V** cores, the same with a target:

```sh
cargo run --release --target riscv32imac-unknown-none-elf
```

Without picotool, make a UF2 and copy it to the `RP2350` drive that appears in BOOTSEL
mode: `picotool uf2 convert target/thumbv8m.main-none-eabihf/release/rp2350-pio -t elf
rp2350-pio.uf2` (elf2uf2-rs doesn't know the RP2350). With a Raspberry Pi Debug Probe
on the SWD pins, `probe-rs run --chip RP235x target/thumbv8m.main-none-eabihf/release/rp2350-pio`
works for the Arm build.

## Settings

At the top of `src/main.rs`: `LED_COUNT`, `SWEEP_MAX`, `LedType` (use `Sk6812` or
`Sk6812Rgbw` for those strips), `DotStarType` (`Sk9822` for SK9822 strips),
`DOTSTAR_COUNT`, `DOTSTAR_HZ`, `DOTSTAR_CLOCKS`, `BRIGHTNESS` and `POWER_LIMIT`.

## Troubleshooting

- **Nothing lights**: check that the board was in BOOTSEL mode and picotool reported a
  load, the 74AHCT125's OE pins are grounded, and the grounds are joined.
- **The first LED flickers or shows wrong colors**: the level shifter or the 330 ohm
  resistor is missing, or the data wire is long.
- **Colors swapped (red shows green)**: `LedType` doesn't match the strip.
- **Round 1 or 4 shows a stray flash on long strips**: the processor fell behind the
  FIFO; rounds 2 and 3 (DMA) should be clean. Please report it.

## Status

Pending hardware test (Arm and RISC-V cores, NeoPixel strip and DotStars).
