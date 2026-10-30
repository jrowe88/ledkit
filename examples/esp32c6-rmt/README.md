# ESP32-C6 first light (RMT)

Drives the onboard RGB LED of the ESP32-C6-DevKitC-1 with `ledkit-core` and
`ledkit-esp`. The LED's data pin (GPIO8) is fed by the chip's RMT peripheral,
which makes the precisely timed pulses a NeoPixel needs, in hardware.

> **Tested on hardware:** pending. Not flashed yet.

## SPI or RMT?

The RMT is the ESP32's purpose-built peripheral for this job, and using it
leaves the SPI bus free for other things. SPI (see `examples/esp32c6-spi`)
works on any board, so choose it if you want code that moves between chips.

## What you'll see

1. Red, green, blue, then white, one second each, dimly lit (20% brightness).
2. Two frames back to back: red, then blue. The LED should end up blue.
3. A smooth, slow rainbow (about 5 seconds per cycle) that runs forever.

## One-time setup

```sh
rustup target add riscv32imac-unknown-none-elf
cargo install espflash --locked
```

That is all. You need stable Rust 1.95 or newer, which esp-hal requires
(`rustup update stable`). The ESP32-C6 is a RISC-V chip, so ordinary stable Rust works and
you do **not** need the `espup` toolchain.

## Run it

Plug the board into its USB port (the one labelled UART or USB, either works
on the DevKitC-1), then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit. If more than one serial
port is found, espflash asks which to use.

## Expected serial output

```text
ledkit first light: 1 LED
showing red
showing green
showing blue
showing white
two frames back to back: red, then blue
rainbow
```

The LED should show each color as its name is printed.

## Troubleshooting

- **Wrong colors** (for example red shows as green): `LedType` in
  `src/main.rs` doesn't match your LED. Try another chip type from the
  `ledkit_core` catalogue.
- **Nothing lights up**: check the data pin. The onboard LED is on GPIO8 on the
  DevKitC-1, but other boards differ. Change `peripherals.GPIO8` in
  `src/main.rs`.
- **`ClockNotUsable` panic at start-up**: the clock passed to `RmtOutput::new`
  must be the one the RMT really runs at, and a multiple of 10 MHz (80 MHz is
  the usual choice). Keep `RMT_HZ` the same in both places.
- **Can't flash** ("port busy" or access denied): another program is holding
  the serial port. On Windows this is often a serial monitor left open on the
  same port (for example COM5). Close it and try again.

## How it fits together

`src/main.rs` is commented step by step. The settings at the top (`LED_COUNT`,
`LedType`, `RMT_HZ`, `BRIGHTNESS`) are the ones you are most likely to change.
