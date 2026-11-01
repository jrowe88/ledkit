# ESP32-C6 NeoPixel strip from a serial port (UART)

Drives a WS2812B ("NeoPixel") strip from the ESP32-C6's serial port with `ledkit-core` and
`ledkit-uart`. The strip's data wire is fed from the port's transmit pin with its output
inverted, so the port's hardware does the timing.

> **Status: tested on hardware.** Run on an ESP32-C6 DevKit with a 100-LED WS2812-type
> strip through a 74AHCT125, on 2026-11-01: both rounds showed stable colors, the counting
> pattern through to LED 99, and a clean rainbow.

## What you'll see

The program repeats two rounds forever, first with the blocking serial port, then with the
async one:

1. The whole strip red, green, blue, then white, one second each.
2. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue.
3. A fast rainbow, 50 frames a second, for 10 seconds.

The settings at the top of `src/main.rs` are `LED_COUNT` (100), `LedType` (`Ws2812b`),
`BAUD` (4 Mbaud), `BRIGHTNESS` (20%) and `POWER_LIMIT` (500 mA).

## Wiring

| From | To |
|---|---|
| ESP32-C6 GPIO4 (serial transmit) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | the strip's DIN (data in) |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (4, 10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (5, 9, 12) | GND |

- A **5 V** supply to the strip's 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the board, the 74AHCT125 and the strip all share it.
- No receive wire: only the port's transmit side is used.

The output inversion is done inside the C6 (its GPIO matrix), so no extra chip is needed.

## One-time setup

```sh
rustup target add riscv32imac-unknown-none-elf
cargo install espflash --locked
```

You need stable Rust 1.95 or newer, which esp-hal requires (`rustup update stable`).

## Run it

Plug the board into its USB port, then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output. Press Ctrl+R
to restart the board, Ctrl+C to quit. On a DevKit with two USB ports, use the one wired to
the chip's own USB if the other won't flash.

## Expected serial output

```text
ledkit on a serial port: 100 LEDs at 4000000 baud
BLOCKING
  red
  green
  blue
  white
  counting: LED 0 red, every 10th white, last blue
  rainbow
ASYNC
  red
...
```

## Troubleshooting

- **Nothing lights up**: check the 74AHCT125's 5 V supply, the common ground, and that the
  data goes into the strip's DIN, not DOUT.
- **Wrong colors** (for example red shows as green): `LedType` doesn't match your LEDs.
- **Random colors everywhere**: the output isn't inverted. Keep `with_output_inverter(true)`.
- **A build error about the baud**: that baud can't drive your LED type. `ledkit-uart`'s
  README lists the bauds that work for each one.
- **LED 0 shows a stray color at power-up**: the pin can pulse before the inversion takes
  effect. It clears with the first frame.
