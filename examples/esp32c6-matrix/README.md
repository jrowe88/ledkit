# ESP32-C6 LED matrix: text and shapes (RMT)

Draws shapes and scrolling text on a WS2812-style LED matrix with `ledkit-layout`,
`ledkit-core` and `ledkit-esp`. The matrix's data pin (GPIO4) is fed by the chip's
RMT peripheral, which makes the precisely timed pulses the LEDs need, in hardware.
`ledkit-layout` turns "column 3, row 5" into the right LED for your panel's wiring,
and its `embedded-graphics` feature lets you draw text and shapes with the usual
`embedded-graphics` tools.

> **Status: tested on hardware.** Run on an ESP32-C6 DevKit with an Adafruit
> NeoPixel 8×8 (WS2812B) through a 74AHCT125: the finder, the shapes and the
> text all came out right.

## What you'll see

The program repeats three modes forever:

1. **Wiring finder** (3 seconds): LED 0 red, the rest of the first line green, and
   the first three LEDs of the second line blue. See "Finding your wiring" below.
2. **Shapes** (5 seconds): a red border, a green diagonal line and a blue filled circle.
3. **Scrolling text** (8 seconds): "ledkit", a different color per letter, one pixel per
   frame (100 ms), moving right to left. The font is `FONT_5X8` below 10 rows and
   `FONT_6X10` otherwise.

The settings at the top of `src/main.rs` are defaults for a generic panel (8 by 8,
`Ws2812b`, progressive, rows first, starting top-left). Set `WIDTH`, `HEIGHT`,
`LedType` and the wiring to match your panel. The tested Adafruit NeoPixel 8×8, held
with its data-in pin at the bottom left, is
`MatrixLayout::new(8, 8).columns_first().starting_at(Corner::BottomLeft)`. An 8-row panel fits the 5×8 font exactly; below 8 rows the text is clipped.

## Wiring

- ESP32-C6 **GPIO4** goes to a **74AHCT125** level shifter (the ESP32's 3.3 V signal is
  too weak for 5 V LEDs to read reliably), and its output goes through a **330 Ω**
  resistor to the matrix's **DIN**:

| From | To |
|---|---|
| ESP32-C6 GPIO4 | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | 330 ohm resistor, then the matrix's DIN |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (4, 10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (5, 9, 12) | GND |

- A **5 V** supply to the matrix's 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the board, the 74AHCT125 and the matrix all share it.
- Put a **1000 µF capacitor** across the matrix's 5 V and GND, close to the matrix, so
  switching on does not spike the supply. Connect the supply before the data line.

## Power

`POWER_LIMIT` is `Milliamps(500)` by default, with `BRIGHTNESS` at 10%. A matrix draws a
lot: each LED can pull about 60 mA at full white, so a 16x16 (256 LEDs) can draw over
10 A, far more than USB can give. `ledkit` scales the picture down whenever it would
pass the limit, so a dim matrix is the limit working.

The limit caps the LEDs' estimated current, not the board's: it leaves out the
microcontroller. Each LED also draws a little even when it is off (ledkit models about
1 mA per LED), so a big panel's idle draw alone can exceed USB's budget, and no
software setting goes below it. A 32x32 panel is modelled at 1024 mA while showing
nothing. Small panels (say up to 8x8) are usually fine on USB with this limit. For
anything bigger, use a 5 V supply rated for the panel, and join its ground to the
board's.

To raise the limit, use a proper 5 V supply rated for the current you set, with thick
enough wires, then raise `POWER_LIMIT` and `BRIGHTNESS` in `src/main.rs` together.

## Finding your wiring

Panels differ in where the data wire goes in and which way the chain
snakes, and unbranded ones rarely say. Run the program and watch the wiring finder. It lights LED 0 red, LEDs 1 to
`LINE - 1` green, and the first 3 LEDs of the next line (LEDs `LINE` to
`LINE + 2`) blue. A line is a row, or a column once the layout is columns
first, so `LINE` is `WIDTH` or `HEIGHT`. Red plus green together cover `LINE`
LEDs; the green alone is `LINE - 1`. Find your wiring in three steps:

1. Run the finder with the default rows-first layout.
2. If the green runs down a column instead of along a row, the panel is
   columns first. Add `.columns_first()` and run it again; the finder then
   follows the columns.
3. Now judge serpentine from the blue: blue next to the end of the green means
   serpentine, and blue back by the red means progressive.

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
