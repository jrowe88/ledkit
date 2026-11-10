# ESP32-S3 first light (RMT)

Drives the onboard NeoPixel of the Adafruit ESP32-S3 Feather (4 MB flash / 2 MB PSRAM,
product 5477) with `ledkit-core` and `ledkit-esp`. The NeoPixel's data pin (GPIO33) is
fed by the chip's RMT peripheral, which makes the precisely timed pulses a NeoPixel
needs, in hardware.

> **Passes on hardware** (2026-11-09, Adafruit ESP32-S3 Feather 4 MB flash / 2 MB
> PSRAM, product 5477, chip revision v0.2): colors in order, the back-to-back frames end
> blue, and a smooth rainbow. If you try it on other hardware, please report what you see.

## What you'll see

1. Red, green, blue, then white, one second each, dimly lit (20% brightness).
2. Two frames back to back: red, then blue. The red is far too short to see; the test
   passes if the LED ends up blue (not red) after the white.
3. A smooth, slow rainbow (about 5 seconds per cycle) that runs forever.

The Feather can switch its NeoPixel's power off to save battery (GPIO21). The program
switches it on first.

## One-time setup

The ESP32-S3 has Xtensa cores, which plain Rust can't build for yet. Espressif's
`espup` installs a Rust toolchain that can (named `esp`); this folder's
`rust-toolchain.toml` picks it for you.

```sh
cargo install espup espflash --locked
espup install --targets esp32s3
```

`espup install` takes a few minutes and writes an `export-esp` script to your home
folder. The examples here don't need it (they don't use C code); the ESP-IDF example
does.

## Run it

Plug the Feather into USB, then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output. Press
Ctrl+R to restart the board, Ctrl+C to quit. If more than one serial port is found,
espflash asks which to use.

If espflash can't connect, put the board in its download mode: hold **BOOT**, tap
**RESET**, let go of BOOT, and run again. Afterwards press RESET to start the program.

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

- **Wrong colors** (for example red shows as green): `LedType` in `src/main.rs`
  doesn't match your LED. Try another chip type from the `ledkit_core` catalogue.
- **Nothing lights up**: the NeoPixel's power pin (GPIO21) must be high, and the data
  pin is GPIO33 on this Feather. Other ESP32-S3 boards differ; change both in
  `src/main.rs`.
- **`ClockNotUsable` panic at start-up**: the clock passed to `RmtOutput::new` must be
  the one the RMT really runs at, and a multiple of 10 MHz (80 MHz is the usual choice).
  The example reads it back with `rmt.frequency().as_hz()`.
- **`error: toolchain 'esp' is not installed`**: run `espup install --targets esp32s3`.
- **Can't flash** ("port busy" or access denied): another program is holding the serial
  port, often a serial monitor left open. Close it and try again.
- **Flicker at low brightness, especially in slow fades**: call
  `strip.set_dithering(false)` or show frames faster (≥100 per second).

## Next

For a strip, run [`esp32s3-check`](../esp32s3-check): it drives a strip every way the
ESP32-S3 can (RMT, SPI with and without DMA, UART), one after another.
