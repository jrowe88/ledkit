# ESP32-S3 animations: effects on a matrix or a strip (RMT)

Runs `ledkit-anim` effects on a WS2812-style 8x8 matrix or strip, with
`ledkit-core`, `ledkit-layout` and `ledkit-esp`. The data pin (GPIO5) is fed by
the chip's RMT peripheral, which makes the precisely timed pulses the LEDs need, in
hardware.

> **Pending hardware test.** This example builds in CI, but it has not been run on a
> real ESP32-S3 yet. It is a port of the tested `esp32c6-anim` example. If you try it,
> please report what you see.

## What you'll see

`cargo run --release` builds the show for the 8x8; `cargo run --release --features strip`
builds it for a strip of `STRIP_LEDS` LEDs (100). Each animation plays for 8 seconds and the
show loops forever.

On the **matrix**:

1. **Swirl**: a rainbow spiral turning clockwise.
2. **Rings**: ocean-colored rings spreading out from the middle (fades in).
3. **Pinwheel**: four orange arms spinning, a rotating X (opens in as a circle from the middle).
4. **Plasma**: flowing party-colored blobs (dissolves in, LED by LED).
5. **Flames**: fire rising from the bottom row (fades in).
6. **Rain**: green drops falling down the columns (wipes in from the top).

On the **strip**:

1. **Rainbow**: a moving rainbow.
2. **Comet over a gradient**: a white comet flying over a slow, dim sunset gradient.
3. **Chase**: red pairs of lights marching along, like a theater sign.
4. **Sparkle**: white twinkles over a dim blue.
5. **Flames**: fire burning from the first LED.
6. **Split in three**: a bouncing comet, gold sparkles and a breathing purple glow,
   each on its own third of the strip.

Each one fades into the next.

The matrix layout is the tested Adafruit NeoPixel 8x8, held with its data-in pin at
the bottom left: `MatrixLayout::new(8, 8).columns_first().starting_at(Corner::BottomLeft)`.
For another panel, find its wiring with the wiring finder in
[`esp32s3-matrix`](../esp32s3-matrix), and change `LAYOUT`. If the matrix shows the
animations turned or flipped, the layout is the thing to fix.

## Wiring

- ESP32-S3 **GPIO5** (marked "5" on the Feather) goes to a **74AHCT125** level shifter (the ESP32's 3.3 V signal is
  too weak for 5 V LEDs to read reliably), and its output goes through a **330 Ω**
  resistor to the LEDs' **DIN**:

| From | To |
|---|---|
| ESP32-S3 GPIO5 | 74AHCT125 pin 2 (1A, input) |
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
8x8. A 100-LED strip at full white could draw about 6 A; ledkit scales the picture
down to stay under the limit, so a dim strip is the limit working. To go brighter,
power the strip from a 5 V supply rated for it, join its ground to the board's, and
raise `POWER_LIMIT` and `BRIGHTNESS` together.

## One-time setup

The ESP32-S3 has Xtensa cores, which plain Rust can't build for yet. Espressif's
`espup` installs a Rust toolchain that can (named `esp`); this folder's
`rust-toolchain.toml` picks it for you.

```sh
cargo install espup espflash --locked
espup install --targets esp32s3
```

## Run it

Plug in the board's USB port, then from this folder:

```bash
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit.

## Expected serial output

```text
ledkit-anim: 64 LEDs, matrix
```

## Changing the show

The two shows are built near the top of `main` as lists of `Entry`s: an animation,
how long it plays, and how it comes in. Swap effects, change their settings, or add
your own; every effect in `ledkit-anim` works on both the matrix and the strip. To
see effects without flashing a board, run the `anim-preview` example on your
computer.

## Troubleshooting

- **Nothing lights:** check the 74AHCT125's pin 1 is on GND and pin 14 on 5 V, and that
  all the grounds are joined.
- **Colors are wrong:** check `LedType`; it decides the color order.
- **Animations look turned or mirrored on the matrix:** `LAYOUT` doesn't match the
  panel's wiring; see "What you'll see" above.
- **Flicker in dim colors:** dithering is already off in this example (it needs about
  100 frames a second); the loop runs at about 60.
