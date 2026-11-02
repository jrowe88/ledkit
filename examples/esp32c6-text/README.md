# ESP32-C6 text: a scrolling ticker on an 8x8 matrix (RMT)

Scrolls text across a WS2812-style 8x8 matrix with `ledkit-text`, `ledkit-core`,
`ledkit-layout` and `ledkit-esp`. The data pin (GPIO4) is fed by the chip's RMT
peripheral, which makes the precisely timed pulses the LEDs need, in hardware.

> **Hardware status: not yet tested.** It builds, but nobody has flashed it yet.
> Written for an ESP32-C6 DevKit with an Adafruit NeoPixel 8x8 (WS2812B), through
> a 74AHCT125.

## What you'll see

A `Ticker` scrolls three messages in a loop, in 5x7 letters colored by a moving
rainbow:

1. `ledkit`
2. `up 12s`: how many seconds the board has been running. The example builds this
   text itself (into a small array, since there is no heap) and hands the ticker
   the list again every second. A new list takes effect at the next gap between
   messages, so the number can be a few seconds behind.
3. `Hello!`

The matrix layout is the tested Adafruit NeoPixel 8x8, held with its data-in pin at
the bottom left, the same as in [`esp32c6-anim`](../esp32c6-anim):
`MatrixLayout::new(8, 8).columns_first().starting_at(Corner::BottomLeft)`. If the text
comes out turned or mirrored, the layout is the thing to fix; find your panel's wiring
with the wiring finder in [`esp32c6-matrix`](../esp32c6-matrix) and change `LAYOUT`.

## The speed test

When the board starts, before the ticker, it draws a few big pictures into memory
(a 192x64 picture that is never sent to any LEDs) and prints how long one picture
takes, on average:

```text
ledkit-text speed test, 192x64 LEDs in memory:
14-segment, 40 rows: ... ms per frame
7-segment, 40 rows: ... ms per frame
10x20 at 2x: ... ms per frame
ledkit-text: ticker on 64 LEDs
```

What the numbers mean: **under about 33 ms per frame is 30 frames a second or better**,
which looks smooth. The first two are a 40-LED-tall "12:34" in a segment font with
`Plasma` letters and a black outline; the third is a bitmap font drawn at twice its
size. They tell you how big a panel this chip can drive with moving text. Sending the
pixels to real LEDs takes extra time on top (about 30 microseconds per LED).

Don't want it? Set `RUN_BENCHMARK` to `false` at the top of `src/main.rs`, or delete
the `benchmark` function.

## Wiring

- ESP32-C6 **GPIO4** goes to a **74AHCT125** level shifter (the ESP32's 3.3 V signal is
  too weak for 5 V LEDs to read reliably), and its output goes through a **330 Ω**
  resistor to the LEDs' **DIN**:

| From | To |
|---|---|
| ESP32-C6 GPIO4 | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | 330 ohm resistor, then the LEDs' DIN |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (4, 10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (5, 9, 12) | GND |

- A **5 V** supply to the LEDs' 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the board, the 74AHCT125 and the LEDs all share it.
- Put a **1000 µF capacitor** across the LEDs' 5 V and GND, close to the LEDs. Connect
  the supply before the data line.

## Power

`POWER_LIMIT` is `Milliamps(500)` and `BRIGHTNESS` is 20%. That is fine on USB for an
8x8; ledkit scales the picture down to stay under the limit. To go brighter, raise
`BRIGHTNESS` (and `POWER_LIMIT` with it) and power the panel from a 5 V supply rated
for it, with its ground joined to the board's.

## One-time setup

```bash
rustup target add riscv32imac-unknown-none-elf
```

```bash
cargo install espflash --locked
```

You need stable Rust 1.95 or newer, which esp-hal requires (`rustup update stable`).
The ESP32-C6 is a RISC-V chip, so ordinary stable Rust works and you do **not** need
the `espup` toolchain.

## Run it

Plug in the board's USB port, then from this folder:

```bash
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit.

## Changing the show

Edit the messages in the `set_messages` call in `src/main.rs`, change the font
(`Font::X11_5X7`; the 4x6 and 5x8 fonts also fit an 8-row panel), the speed
(`SCROLL_SPEED`) or the paint: `Rainbow::new()` can be a plain `Rgb::GREEN` or any
other effect. To see text without flashing a board, run the `anim-preview`
example on your computer (`cargo run -- text "hello"`).

## Troubleshooting

- **Nothing lights:** check the 74AHCT125's pin 1 is on GND and pin 14 on 5 V, and that
  all the grounds are joined.
- **Colors are wrong:** check `LedType`; it decides the color order.
- **Text is turned or mirrored:** `LAYOUT` doesn't match the panel's wiring.
- **The board resets or panics at boot:** set `RUN_BENCHMARK` to `false`, which skips
  the 37 KB picture the speed test keeps in memory, and tell us.
