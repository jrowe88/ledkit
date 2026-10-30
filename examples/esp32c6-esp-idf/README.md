# ESP32-C6 first light (ESP-IDF)

Drives the onboard RGB LED of the ESP32-C6-DevKitC-1 with `ledkit-core` and
`ledkit-esp-idf`. The LED's data pin (GPIO8) is fed from the chip's RMT
peripheral, which generates the exact pulse train a NeoPixel wants. This is the
`std` flavour of Rust on ESP32, running on ESP-IDF, so Wi-Fi, threads and
`println!` all work. (For bare-metal `esp-hal` see `examples/esp32c6-spi`.)

> **Tested on hardware:** pending.

## What you'll see

1. Red, green, blue, then white, one second each, dimly lit (20% brightness).
2. Two frames sent back to back (red, then blue). The LED should end on blue;
   that shows the reset gap is honoured between frames.
3. A smooth, slow rainbow (about 5 seconds per cycle) that runs forever.

## One-time setup

You need the pinned nightly Rust (chosen for you by `rust-toolchain.toml`, with
`rust-src`), `ldproxy` to link, and `espflash` to flash:

```sh
cargo install ldproxy espflash --locked
```

On the first build, ESP-IDF and its tools are downloaded into `~/.espressif`
(shared by all your ESP-IDF projects). That takes a while (10+ minutes); later
builds are quick. Installing Python 3, git, cmake and ninja beforehand helps.
`ESP_IDF_VERSION` and the target are set in `.cargo/config.toml`.

### Windows

- **Short build folder.** `esp-idf-sys` refuses to build when its output folder path is
  longer than 87 characters. Point cargo at a short folder outside the project:
  `set CARGO_TARGET_DIR=C:/t/ledkit` (cmd) or `$env:CARGO_TARGET_DIR = "C:/t/ledkit"`
  (PowerShell), or uncomment `target-dir` in `.cargo/config.toml`. A folder name like
  `C:/t/ledkit-idf` is already one character too long.
- Don't use `subst` drives; they don't help.
- Set `LIBCLANG_PATH` to the `bin` folder of an LLVM install if the build can't find
  libclang.

### Why is the toolchain pinned?

`rust-toolchain.toml` pins `nightly-2026-10-01`. Nightlies from 2026-10-24 on break
`std` for ESP-IDF (libc 0.2.191 renamed the `stat` time fields). Try plain `nightly`
again once that is fixed upstream.

## Run it

Plug the board into its USB port, then, in this folder:

```sh
cargo run --release
```

This builds, flashes the board with `espflash`, and shows the serial output.
Press Ctrl+R to restart the board, Ctrl+C to quit.

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

## Troubleshooting

- **Wrong colors** (for example red shows as green): `LedType` in `src/main.rs`
  doesn't match your LED. Try another chip type from the `ledkit_core` catalogue.
- **Nothing lights up**: check the data pin. The onboard LED is on GPIO8 on the
  DevKitC-1, but other boards differ. Change `peripherals.pins.gpio8` in `src/main.rs`.
- **Can't flash** ("port busy" or access denied): another program is holding
  the serial port; close any serial monitor.
- **`Too long output directory`**: see "Windows" above.

## Dependency note

`ledkit-esp-idf` depends on the published `esp-idf-hal` 0.47 (a normal crates.io dependency; no `[patch.crates-io]` git redirect is needed, unlike
projects that track the esp-rs git repository). Cargo.lock pins `esp-idf-hal` 0.47.0 and
`esp-idf-sys` 0.38.1.
