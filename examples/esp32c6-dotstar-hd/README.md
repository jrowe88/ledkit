# ESP32-C6 DotStar: HD brightness side by side (SPI)

Shows ledkit's DotStar "HD" brightness next to plain dimming, on an 8x8 DotStar
(APA102) panel, so you can see the difference for yourself. The panel's data and clock
come from the chip's SPI peripheral, through a level shifter.

A DotStar can dim in two ways: with its 8-bit colors, or with a 5-bit "global" level
that each LED also takes. Plain dimming lowers only the colors, so a dim color has
only a few steps left, and on some DotStars the shortest color pulses hardly light at
all. HD brightness, ledkit's default for DotStars, lowers the 5-bit level as well and
keeps the colors high, so dim colors keep their steps and really light up.

> **Status: tested on hardware.** Run on an ESP32-C6 DevKit with an Adafruit DotStar
> HD 8x8 (APA102-2020) through a 74AHCT125, on 2026-11-01. See "What we saw" below.

## What you'll see

Three rows of the panel are used:

- **Bottom row**: a still rainbow with **plain** dimming.
- **Third row from the bottom**: the same rainbow at the same brightness, with **HD**.
- **Top row**: a dim white **counter**, so you know which step you're looking at.

The program repeats forever:

1. **Start**: the top row blinks three times.
2. **Seven steps**, 5 seconds each: 5%, 10%, 15%, 20%, 30%, 40% and 60% brightness.
   The counter shows the step number: 1 LED for step 1, 2 for step 2, and so on.
3. **A sweep** from 1% up to 60% and back down, about 7 seconds. The counter follows
   along: it lights one LED for each step's brightness the sweep has passed, so you
   can match the sweep to the steps.

## What we saw

On the Adafruit DotStar HD 8x8:

| Step | Brightness | HD row | Plain row |
|---|---|---|---|
| 1 | 5% | off | off |
| 2 | 10% | lit | off |
| 3 | 15% | lit | off |
| 4 | 20% | lit | lit, about as bright as HD at step 2 |
| 5 | 30% | lit | lit |
| 6 | 40% | lit | about equal, plain a little dimmer |
| 7 | 60% | lit | equal |

At 5% both rows are off: 5% is below the first brightness level that ledkit sends.
From 10% to 15% the plain row's colors are only 3 to 5 out of 255, and pulses that
short don't light this panel's LEDs. HD sends the same light as a lower 5-bit level
with much higher colors, which does light them. From a little over 50% up, HD leaves
the 5-bit level at full (31), so the rows are the same.

Your panel may differ: other DotStars (and the SK9822) light short pulses better.

## How it works

ledkit's `Strip` sends one 5-bit level for the whole frame, but the LEDs read it per
LED. So this program runs two 8-LED strips, one with HD off and one with HD on, each
into a small "capture" output that keeps the colors and the 5-bit level ledkit chose.
It then sends both rows in one frame, each LED with its own level. Everything the
LEDs get comes from ledkit's own pipeline and frame builder.

In your own projects you don't need any of this: `Strip` with `ClockedSpi` (see the
[`esp32c6-spi`](../esp32c6-spi) example and `ledkit-spi`) uses HD by default, and
`strip.set_hd_brightness(false)` turns it off.

## Wiring

- ESP32-C6 **GPIO4** (data) and **GPIO6** (clock) go through a **74AHCT125** level
  shifter (the ESP32's 3.3 V signals are too weak for 5 V LEDs to read reliably) to the
  panel's **DI** and **CI**:

| From | To |
|---|---|
| ESP32-C6 GPIO4 (data) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | the panel's DI (data in) |
| ESP32-C6 GPIO6 (clock) | 74AHCT125 pin 5 (2A, input) |
| 74AHCT125 pin 6 (2Y, output) | the panel's CI (clock in) |
| 74AHCT125 pins 1 and 4 (1OE, 2OE) | GND (this switches those outputs on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (9, 12) | GND |

- A **5 V** supply to the panel's 5 V and the 74AHCT125's supply. USB is enough for
  this program (see "Power").
- **Common ground**: the supply, the board, the 74AHCT125 and the panel all share it.
- Use the panel's **input** side (DI and CI), not DO and CO on the far end.

Swapping data and clock gives random colors, so check those two first if the panel
shows garbage.

## Power

At most 16 LEDs are lit, at up to 60% brightness, plus a dim counter row: well inside a
USB port's 500 mA. If you raise the steps above 60%, mind the current: a DotStar at
full white draws about 60 mA.

## One-time setup

```sh
rustup target add riscv32imac-unknown-none-elf
cargo install espflash --locked
```

You need stable Rust 1.95 or newer, which esp-hal requires (`rustup update stable`).
The ESP32-C6 is a RISC-V chip, so ordinary stable Rust works and you do **not** need
the `espup` toolchain.

## Run it

Plug the board into its USB port, then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit. On a DevKit with two USB ports, use
the one wired to the chip's own USB (on our second DevKit, the other port's
USB-to-serial chip would not flash).

## Expected serial output

```text
ledkit DotStar HD side by side: bottom row plain, third row HD
start
step 1: 5%  plain: level 31, red 0  HD: level 31, red 0
step 2: 10%  plain: level 31, red 3  HD: level 2, red 47
step 3: 15%  plain: level 31, red 5  HD: level 3, red 52
step 4: 20%  plain: level 31, red 8  HD: level 4, red 62
step 5: 30%  plain: level 31, red 19  HD: level 10, red 59
step 6: 40%  plain: level 31, red 35  HD: level 17, red 64
step 7: 60%  plain: level 31, red 84  HD: level 31, red 84
sweep 1% -> 60% -> 1%
start
...
```

"Level" is the 5-bit level (31 is full) and "red" is the red byte sent to the
rainbow's first LED, which is pure red. Each row's light is about level / 31 x red.

## Troubleshooting

- **Random colors or flicker everywhere**: data and clock are swapped, or a 74AHCT125
  OE pin is left floating. See "Wiring".
- **Nothing lights up**: check the 74AHCT125's 5 V supply, the common ground, and that
  you used the panel's DI and CI, not DO and CO.
- **The rows are on the wrong lines**: this program assumes LED 0 is at the bottom
  left and each row runs left to right. Change `PLAIN_ROW`, `HD_ROW` and
  `COUNTER_ROW` (counted from LED 0's row) for your panel.
- **Wrong colors** (for example red shows as blue): `LedType` in `src/main.rs`
  doesn't match your LEDs.
- **Can't flash** ("port busy" or access denied): another program holds the serial port.
  Close any serial monitor and try again.
