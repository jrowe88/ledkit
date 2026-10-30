# Raspberry Pi (Linux spidev) LED strip

Drives a WS2812B ("NeoPixel") strip from a Raspberry Pi, or any Linux board
with an SPI device file, using `ledkit-core` and `ledkit-spi`. The strip's data
wire is fed from the Pi's SPI hardware, so no special LED driver is needed.

> **Untested on hardware.** This example compiles in CI, but nobody has run it
> on a real Pi yet. If you try it, please report what you see.

## What you'll see

1. The whole strip red, then green, then blue, one second each, dimly lit
   (20% brightness).
2. A rainbow that moves along the strip, until you press Ctrl-C. (The LEDs keep
   their last colors after the program ends.)

## Wiring

| Pi | Strip |
|---|---|
| GPIO10 / MOSI (physical pin 19) | DIN (data in) |
| Any GND pin (for example pin 20) | GND |

Power the strip from its own 5 V supply and join that supply's ground to the
Pi's ground. Do not power a long strip from the Pi's 5 V pin.

The Pi's data line is 3.3 V and the strip expects 5 V logic. On short leads it
usually works anyway, but a level shifter such as a 74AHCT125 makes it
reliable. Keep the data wire short.

## Turn SPI on

SPI is off by default. Either run `sudo raspi-config` and choose
Interface Options, then SPI, or add this line to `/boot/firmware/config.txt`
(`/boot/config.txt` on older systems) and reboot:

```text
dtparam=spi=on
```

You should then see `/dev/spidev0.0`. If you get "permission denied", add your
user to the `spi` group (`sudo usermod -aG spi $USER`) and log in again.

## Keep the SPI clock steady

The LED timing comes from the SPI clock, so that clock must not change while
the strip is being updated. On Pi models whose core clock speeds up and slows
down with load, the SPI speed changes with it, and the LEDs flicker or show
wrong colors. Pin the core clock in the same `config.txt`:

```text
core_freq=250
```

`core_freq=250` is the usual value for the Pi 3 and Zero. The Pi 4's core
clock varies too, so on a Pi 4 pin it with both of these lines instead:

```text
core_freq=500
core_freq_min=500
```

The Pi 5 has its SPI on the RP1 chip and needs no change. A steady clock is
the first thing to check if the LEDs flicker.

## Longer strips

`LED_COUNT` in `src/main.rs` is 30. Linux spidev limits one write to 4096 bytes
by default (`bufsiz`), which is about 330 WS2812B LEDs at 3.2 MHz. For more,
raise the limit with the `spidev.bufsiz=65536` kernel parameter (add it to
`/boot/firmware/cmdline.txt` on one line and reboot). Do not use
`with_max_write_len` as a fix here: the pauses between user-space writes can be
longer than the LED reset time, which would split the frame into several.

## Build and run

Install Rust with rustup (https://rustup.rs); Raspberry Pi OS's packaged
`rustc` is too old (this example needs Rust 1.95 or newer).

On the Pi, in this folder:

```sh
cargo run --release
```

To cross-compile from another computer instead (for a 64-bit Pi OS):

```sh
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

You will need a linker for that target (on Ubuntu, `gcc-aarch64-linux-gnu`,
set as `linker` in `.cargo/config.toml`). Copy
`target/aarch64-unknown-linux-gnu/release/linux-spidev` to the Pi and run it.
Building on the Pi itself is simpler.

## Troubleshooting

- **"No such file or directory"**: SPI is not enabled. See above.
- **Wrong colors** (for example red shows as green): `LedType` in
  `src/main.rs` doesn't match your LED. Try another chip type from the
  `ledkit_core` catalogue.
- **Flicker, or the first LED is wrong**: see "Keep the SPI clock steady", and
  check the ground connection. A level shifter helps.
- **Nothing lights up**: check that data goes to DIN (not DOUT), the grounds
  are joined, and the strip has power.
- **Flicker at low brightness, especially in slow fades**: call
  `strip.set_dithering(false)` or show frames faster (≥100 per second).

## How it fits together

`src/main.rs` is commented step by step. The settings at the top (`LED_COUNT`,
`LedType`, `SPI_HZ`, `SPI_DEVICE`) are the ones you are most likely to change.
