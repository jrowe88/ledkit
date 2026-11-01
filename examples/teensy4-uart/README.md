# Teensy 4 NeoPixel strip from a serial port (UART)

Drives a WS2812B ("NeoPixel") strip from a Teensy 4.0 or 4.1 serial port with `ledkit-core`
and `ledkit-uart`. The strip's data wire is fed from serial port 1's transmit pin with the
output inverted, so the port's hardware does the timing. One round writes to the port from
the processor, the next feeds it by DMA.

> **Status: tested on hardware.** Run on a Teensy 4.0 with a 100-LED WS2812-type strip
> through an SN74LVC245AN buffer, on 2026-11-01: both rounds (processor and DMA) cycled
> through the colors, counting and rainbow correctly.

Use a 74AHCT125 (see Wiring) or an HCT/AHCT '245 to lift the signal to 5 V. The tested
SN74LVC245AN is really a 3.3 V part (rated 1.65 to 3.6 V), so it buffers but doesn't lift
the signal; the strip happened to read it. A '245's pins: A1 (pin 2) from
the Teensy, B1 (pin 18) to the strip, DIR (pin 1) to VCC, OE (pin 19) to GND, VCC (5 V) pin 20,
GND pin 10, unused A inputs to GND. Mind the notch: powered the wrong way round it gets hot
and dies.

## What you'll see

The program repeats two rounds forever. The orange LED is off during the first (the
processor writes to the port) and on during the second (DMA feeds the port). Each round:

1. The whole strip red, green, blue, then white, one second each.
2. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue.
3. A fast rainbow, 50 frames a second, for 10 seconds.

The settings at the top of `src/main.rs` are `LED_COUNT` (100), `LedType` (`Ws2812b`),
`BAUD` (4 Mbaud), `BRIGHTNESS` (20%) and `POWER_LIMIT` (500 mA).

## Wiring

| From | To |
|---|---|
| Teensy pin 1 (serial 1 transmit) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | the strip's DIN (data in) |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| 74AHCT125 other OE pins (4, 10, 13) | 5 V (switched off) |
| 74AHCT125 other inputs (5, 9, 12) | GND |

- A **5 V** supply to the strip's 5 V and the 74AHCT125's supply.
- **Common ground**: the supply, the Teensy, the 74AHCT125 and the strip all share it.
- Pin 0 (serial 1 receive) is claimed by the program but not connected.
- The Teensy 4's pins are **not** 5 V tolerant: never connect a 5 V signal to one.

The output inversion is done inside the Teensy's serial port, so no extra chip is needed.

## One-time setup

```sh
rustup target add thumbv7em-none-eabihf
cargo install cargo-binutils
rustup component add llvm-tools
```

You also need a way to load the program onto the Teensy: either the
[Teensy Loader](https://www.pjrc.com/teensy/loader.html) application (it comes with
Teensyduino) or the command-line
[`teensy_loader_cli`](https://github.com/PaulStoffregen/teensy_loader_cli).

## Build and load

In this folder:

```sh
cargo build --release
cargo objcopy --release -- -O ihex teensy4-uart.hex
```

Then load `teensy4-uart.hex` with the Teensy Loader (File, Open HEX File, then press the
white button on the board), or with
`teensy_loader_cli --mcu=TEENSY40 -w -v teensy4-uart.hex` (`TEENSY41` for a 4.1).

## The serial port's clock

The Teensy's serial ports run from its 24 MHz crystal. The board support package divides
that by 3, which tops out at 2 Mbaud, too slow for NeoPixels. So the program sets the
divider to 1 (24 MHz) before it sets up the port, and 24 MHz / 6 gives exactly 4 Mbaud.
Changing that divider changes it for all eight serial ports, so set the speed of any other
port you use afterwards.

## DMA

The DMA round wraps the port in a small `DmaUart` adapter (in `src/main.rs`) that
implements `embedded_io_async::Write` with `imxrt-hal`'s `Lpuart::dma_write`. It is driven
with `embassy_futures::block_on`, which keeps checking the transfer until it is done, so no
DMA interrupt is needed. In a program with an async runtime, you'd wire up the DMA interrupt
and `await` instead. The frame buffer lives on the stack, which the board support package
puts in the chip's uncached DTCM memory, so the DMA engine sees exactly what ledkit wrote.

## Troubleshooting

- **Nothing lights up**: check the 74AHCT125's 5 V supply, the common ground, that you used
  pin 1, and that the data goes into the strip's DIN, not DOUT.
- **Wrong colors** (for example red shows as green): `LedType` doesn't match your LEDs.
- **Random colors everywhere**: the output isn't inverted. Keep
  `set_inversion(Direction::Tx, true)`.
- **The DMA round never starts (the orange LED stays on, the strip freezes)**: the DMA
  transfer isn't completing. Report it, with your Teensy model.
- **A build error about the baud**: that baud can't drive your LED type. `ledkit-uart`'s
  README lists the bauds that work for each one.
