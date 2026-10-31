# ESP32-C6 LED matrix: text and shapes (RMT)

Draws shapes and scrolling text on a WS2812-style LED matrix with `ledkit-layout`,
`ledkit-core` and `ledkit-esp`. The matrix's data pin (GPIO4) is fed by the chip's
RMT peripheral, which makes the precisely timed pulses the LEDs need, in hardware.
`ledkit-layout` turns "column 3, row 5" into the right LED for your panel's wiring,
and its `embedded-graphics` feature lets you draw text and shapes with the usual
`embedded-graphics` tools.

> **Status: pending hardware test.** It builds, but has not been run on a matrix yet.

## What you'll see

The program repeats three modes forever:

1. **Wiring finder** (3 seconds): LED 0 red, the rest of the first line green, and
   the first three LEDs of the second line blue. See "Finding your wiring" below.
2. **Shapes** (5 seconds): a red border, a green diagonal line and a blue filled circle.
3. **Scrolling text**: "ledkit" in `FONT_6X10`, one pixel per frame (about 30 ms),
   moving right to left, with the hue shifting along the letters.

The settings at the top of `src/main.rs` are placeholders (8 by 8, `Ws2812b`,
progressive, rows first, starting top-left). Set `WIDTH`, `HEIGHT`, `LedType` and
the wiring to match your panel. On a panel shorter than 10 rows the 10-pixel-tall
letters are cut off at the top and bottom; that is expected.

## Wiring

- ESP32-C6 **GPIO4** to the input of a **74AHCT125** level shifter (the ESP32's 3.3 V
  signal is too weak for 5 V LEDs to read reliably; 5 V in, 5 V out).
- The 74AHCT125 output through a **330 Ω** resistor to the matrix's **DIN**.
- A **5 V** supply to the matrix's 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the board, the 74AHCT125 and the matrix all share it.
- Put a **1000 µF capacitor** across the matrix's 5 V and GND, close to the matrix, so
  switching on does not spike the supply. Connect the supply before the data line.

## Power

`POWER_LIMIT` is `Milliamps(500)` by default, with `BRIGHTNESS` at 10%. That is safe on
a USB port. A matrix draws a lot: each LED can pull about 60 mA at full white, so a
16x16 (256 LEDs) can draw over 10 A, far more than USB can give. `ledkit` scales the
picture down whenever it would pass the limit, so a dim matrix is the limit working.

To raise it, use a proper 5 V supply rated for the current you set, with thick enough
wires, then raise `POWER_LIMIT` and `BRIGHTNESS` in `src/main.rs` together.

## Finding your wiring

Panels differ in where the data wire goes in and which way the chain snakes, and
unbranded ones rarely say. Run the program and watch the wiring finder. It lights LED 0
red, LEDs 1 to `WIDTH - 1` green, and the first 3 LEDs of the next line (LEDs `WIDTH` to
`WIDTH + 2`) blue. Red plus green together cover `WIDTH` LEDs; the green alone is
`WIDTH - 1`. Look at the panel:

- Red to the end of the green is where the first line runs.
- Where the blue starts is where the second line starts. Blue next to the end of the
  green means serpentine; blue back at the red's side means progressive.

The finder assumes the panel is wired rows first. If the green runs down a column
instead, or wraps onto a second line, the panel is wired columns first (or `WIDTH` is
wrong). Settle rows versus columns before you judge serpentine from the blue LEDs.

Then use the table:

| What you see | Builder call |
|---|---|
| Red is in the top-left corner | nothing (the default) |
| Red is in the top-right corner | `.starting_at(Corner::TopRight)` |
| Red is in the bottom-left corner | `.starting_at(Corner::BottomLeft)` |
| Red is in the bottom-right corner | `.starting_at(Corner::BottomRight)` |
| Red to the end of the green runs along a row | nothing (rows first) |
| Red to the end of the green runs down a column | `.columns_first()` |
| Blue starts next to the end of the green | `.serpentine()` |
| Blue starts back at the red's side | nothing (progressive) |

Use them together, for example
`MatrixLayout::new(8, 32).columns_first().serpentine().starting_at(Corner::BottomLeft)`.
Put the result in `LAYOUT` in `src/main.rs` (and `use` `Corner` from `ledkit_layout`).

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
Press Ctrl+R to restart the board, Ctrl+C to quit.

## Expected serial output

```text
ledkit matrix: 8 x 8 LEDs
wiring finder
red = LED 0 (data in); green = the rest of the first line; blue = where the second line starts
shapes
scrolling text
wiring finder
...
```

The last four lines repeat forever.

## Troubleshooting

- **Scrambled text or shapes**: the wiring is wrong. Run the wiring finder again and
  follow the table above.
- **Mirrored text**: the starting corner is wrong. Try `.starting_at(...)` with the
  corner on the other side (left and right swapped).
- **A dim matrix**: this is the power limit at work. See "Power".
- **Wrong colors** (for example red shows as green): `LedType` in `src/main.rs`
  doesn't match your LEDs.
- **Nothing lights up**: check the 74AHCT125 wiring and its 5 V supply, the common
  ground, and that the data goes into the matrix's DIN, not DOUT.
- **`ClockNotUsable` panic at start-up**: the clock passed to `RmtOutput::new` must be
  the one the RMT really runs at, and a multiple of 10 MHz. The example reads it back
  with `rmt.frequency().as_hz()`.
- **Can't flash** ("port busy" or access denied): another program holds the serial port.
  Close any serial monitor and try again.
