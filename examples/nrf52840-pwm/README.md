# XIAO nRF52840 hardware check: NeoPixels from the PWM (`ledkit-nrf`)

Drives one WS2812B ("NeoPixel") strip from a Seeed XIAO nRF52840 with the chip's PWM
peripheral and EasyDMA: the PWM plays the whole frame from RAM by itself, one duty
value per LED bit, so the processor is free while it goes out. It runs blocking, then
async while another task keeps the processor busy, then a frame-length sweep.

> **Status: pending hardware test.** This example builds in CI, but it has not been run
> on a real XIAO nRF52840 yet. If you try it, please report what you see, round by round.

## What you'll see

The XIAO's own RGB LED shows which part is running, because the board has no serial
port for messages:

| Onboard LED | Part | What it checks |
|---|---|---|
| blue | Round 1: blocking `show()` | the PWM path, back-to-back frames (T2, T3) |
| green | Round 2: async `show_async()`, with a task hogging the processor 5 ms of every 6 | the frame doesn't care how busy the processor is (T8, T9) |
| cyan | Sweep: frames of every length from 1 to 150 LEDs, a dim blue bar growing along the strip | every frame length works (T4) |
| off | A rainbow forever | – |
| **red** | Something failed; the program has stopped | – |

Each round, about 21 seconds:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**; if it stays
   red, the gap between frames wasn't kept.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.

A round **passes** when every color is right through to the last LED, the back-to-back
frames end blue, the counting marks are on LEDs 0, 10, 20 … 90 and 99, and the rainbow
moves smoothly with no flashes or stray colors. Round 2 should look exactly like
round 1. Press **RESET** once to run the whole check again.

Settings at the top of `src/main.rs`: `LED_COUNT`, `LedType` (`Ws2812b`; try `Sk6812`
for SK6812 strips), `BRIGHTNESS` and `POWER_LIMIT`.

## Wiring

| From | To |
|---|---|
| XIAO pin **D10** (P1.15) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | 330 ohm resistor, then the strip's DIN |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| XIAO GND | strip GND and the 5 V supply's GND |

The nRF52840's pins work at 3.3 V; the 74AHCT125 lifts the data signal to the 5 V a
strip expects. Power the strip from its own 5 V supply, not from the XIAO, and join the
grounds. The program limits the strip to 500 mA (`POWER_LIMIT`). The same wire also
serves the [`nrf52840-spi`](../nrf52840-spi) check.

Check D10 against Seeed's pinout card for your board. If the strip glitches, also try
D0 (P0.02): Nordic's datasheet recommends some pins near the radio for slow signals only. Avoid P0.09 and P0.10: they are
the NFC pins, on the pads under the board.

## One-time setup

```sh
rustup target add thumbv7em-none-eabihf
rustup component add llvm-tools
cargo install cargo-binutils
```

and download `uf2conv.py` (and `uf2families.json` next to it) from
<https://github.com/microsoft/uf2/tree/master/utils>. It needs Python 3.

## Flash it with the UF2 bootloader (no extra hardware)

1. Build and convert:

   ```sh
   cargo objcopy --release -- -O ihex nrf52840-pwm.hex
   python uf2conv.py nrf52840-pwm.hex -c -f 0xADA52840 -o nrf52840-pwm.uf2
   ```

2. Plug the XIAO in and **double-tap RESET** (the tiny button next to the USB socket).
   The onboard LED pulses and a USB drive appears.
3. Copy `nrf52840-pwm.uf2` onto the drive. The board restarts and runs the check.

**Check the address once.** `memory.x` puts the program at `0x27000`, just after the
bootloader's SoftDevice (Bluetooth stack) for **S140 7.3.0**, which current XIAO boards
ship with. Open `INFO_UF2.TXT` on the bootloader drive: if it names another SoftDevice
(for example S140 6.1.1, which ends at `0x26000`), change `FLASH`'s `ORIGIN` in
`memory.x` to match. A wrong address gives a board that stays in the bootloader or does
nothing.

## Or flash it with a debug probe

The XIAO has SWD pads on the back (SWDIO, SWCLK, GND, 3V3). With a probe on them (a
Raspberry Pi Pico running *debugprobe* works), install probe-rs and run:

```sh
cargo run --release
```

This writes only the program's part of the flash and keeps the bootloader. If the board
then stays in bootloader mode, copy the UF2 once as above and use the probe afterwards.

## Results

| Date | Round | Result |
|---|---|---|
| | | |
