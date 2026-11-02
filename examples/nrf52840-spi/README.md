# XIAO nRF52840 hardware check: NeoPixels from the SPI (`ledkit-spi`)

Drives one WS2812B ("NeoPixel") strip from a Seeed XIAO nRF52840's SPI peripheral
(SPIM2, which always sends through EasyDMA), three ways, then checks that `ledkit-uart`
refuses the nRF's serial port, which can't drive NeoPixels. It's the portable path,
to compare with the PWM check in [`nrf52840-pwm`](../nrf52840-pwm) on the same wire.

> **Status: pending hardware test.** This example builds in CI, but it has not been run
> on a real XIAO nRF52840 yet. If you try it, please report what you see, round by round.

## What you'll see

The XIAO's own RGB LED shows which round is running:

| Onboard LED | Round | How |
|---|---|---|
| blue | 1 | `ClocklessSpi::new_packed` at 4 MHz (5 SPI bits per LED bit, 15 bytes per LED), blocking |
| green | 2 | the same, async |
| yellow | 3 | `ClocklessSpi::new` at 4 MHz (a whole SPI byte per LED bit), blocking |
| **magenta** | 4 | `ClocklessUart` at 1 Mbaud: refused, as expected (3 seconds, nothing is sent) |
| off | – | a rainbow forever, over SPI |
| **red** | – | something failed (or round 4 was *not* refused); the program has stopped |

Rounds 1 to 3, about 21 seconds each:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.

A round **passes** when every color is right through to the last LED, the back-to-back
frames end blue, the counting marks are on LEDs 0, 10, 20 … 90 and 99, and the rainbow
moves smoothly. Round 4 passes when the onboard LED shows magenta, not red.

Why only 4 MHz: the SPIM runs at 125 kHz to 8 MHz in powers of two (SPIM3 also at 16
and 32 MHz), and ledkit's WS2812B patterns need 2.4, 3.2 or 4 MHz. The serial port
(UARTE) tops out at 1 Mbaud and can't invert its output, so NeoPixels can't work there.
The check uses SPIM2 rather than SPIM3, which has a chip fault (nRF52840 erratum 198)
that can corrupt its data while the processor uses the same RAM.

## Wiring

The same as [`nrf52840-pwm`](../nrf52840-pwm#wiring): XIAO **D10** (P1.15, the SPI's
MOSI) → 74AHCT125 1A, 1Y → 330 ohm → the strip's DIN, 1OE to GND, VCC 5 V, the strip on
its own 5 V supply, grounds joined. No clock wire: NeoPixels don't use one.

## Setup and flashing

As in [`nrf52840-pwm`](../nrf52840-pwm#one-time-setup), with this example's name:

```sh
cargo objcopy --release -- -O ihex nrf52840-spi.hex
python uf2conv.py nrf52840-spi.hex -c -f 0xADA52840 -o nrf52840-spi.uf2
```

Double-tap RESET and copy `nrf52840-spi.uf2` onto the drive that appears (check the
SoftDevice in `INFO_UF2.TXT` against `memory.x` once), or use `cargo run --release` with
an SWD probe.

## Results

| Date | Round | Result |
|---|---|---|
| | | |
