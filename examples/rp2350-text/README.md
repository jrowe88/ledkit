# Pico 2 W text tour on an 8x8 DotStar (SPI)

The same tour of `ledkit-text` as [esp32s3-text](../esp32s3-text), on a Raspberry Pi
Pico 2 W (RP2350): every font that fits 8 rows, the smoothing settings, outlines,
backgrounds, reversed text and effect-colored letters, on an 8x8 DotStar (APA102)
panel. The scene table is in that example's README and at the top of `src/main.rs`.

> **Status: builds, not yet tested on hardware.** It builds for both the Arm and the
> RISC-V cores. The DotStar SPI setup is the one that passed in
> [rp2350-pio](../rp2350-pio).

## Wiring

| Pico 2 W pin | Goes to |
|---|---|
| GPIO19 (pin 25, SPI0 TX) | 74AHCT125 1A; 1Y to the panel's DI (data in) |
| GPIO18 (pin 24, SPI0 SCK) | 74AHCT125 2A; 2Y to the panel's CI (clock in) |
| GND | the panel's GND and the 74AHCT125's GND |

Power the panel from 5 V. Tie 1OE and 2OE of the 74AHCT125 to GND, and power it from 5 V
(VBUS, pin 40, is fine).

## Run it

Hold BOOTSEL while plugging the board in, then:

```sh
cargo run --release
```

picotool loads the program. For the RISC-V cores, add
`--target riscv32imac-unknown-none-elf`.
