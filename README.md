# ledkit

An Adafruit-quality family of Rust crates for addressable LEDs (strips, rings
and matrices), aimed at hobbyists: people with roughly Arduino-level experience
who are curious about Rust or have chosen it for a project. It should "just
work", with low friction and low research, so you can learn by doing. The
library worries about chip-level details so you don't have to. ledkit works
alongside the existing Rust LED ecosystem (`smart-leds` and friends) rather
than replacing it.

## Crates

| Crate | Purpose | Status |
|---|---|---|
| `ledkit` | The one crate hobbyists add; re-exports everything with a prelude | planned |
| `ledkit-color` | Color types and utilities | available (0.1, unreleased) |
| `ledkit-core` | LED chip catalogue, color orders, encoders, pixel buffer, output traits | available (0.1, unreleased) |
| `ledkit-layout` | Segments, matrices and coordinate mapping | planned |
| `ledkit-anim` | Animation trait, built-in effects and combinators | planned |
| `ledkit-spi` | Universal backend over `embedded-hal` SPI | first light on ESP32-C6; examples for [Raspberry Pi](examples/linux-spidev) and [Teensy 4](examples/teensy4-spi) (see also the [ESP32-C6 example](examples/esp32c6-spi)) |
| `ledkit-esp` | ESP32 RMT backend for esp-hal | planned |
| `ledkit-esp-idf` | ESP32 RMT backend for esp-idf-hal | planned |
| `ledkit-rp` | RP2040/RP2350 PIO backend | planned |
| `ledkit-sim` | Desktop simulator | planned |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. See [NOTICE](NOTICE) for attributions.
