# XIAO nRF52840 hardware check: DotStars from the SPI (`ledkit-spi`)

Drives DotStars (APA102 or SK9822) from a Seeed XIAO nRF52840's SPIM3 with
`ledkit-spi`'s `ClockedSpi`, at every clock the SPIM has, blocking and async, then steps
through ledkit's HD brightness. DotStars have a clock wire, so they don't use the PWM
path of [`nrf52840-pwm`](../nrf52840-pwm); SPI is their native path on every board.

> **Status: passes on hardware** (2026-11-08, XIAO nRF52840, DotStar HD 8x8), clean at
> every clock up to 32 MHz. If you try it with other DotStars, please report what you see.

## What you'll see

| Onboard LED | Part | What the LEDs do |
|---|---|---|
| blinks white 1 to 6 times, then off | 1: clock sweep at 1, 2, 4, 8, 16, 32 MHz (one blink = 1 MHz, six = 32 MHz), blocking | red, green, blue, white (1 s each), then counting for 3 s, at each clock |
| green | 2: async at 8 MHz | the same colors and counting, then a moving rainbow at 50 frames a second for 10 s |
| blue | 3: HD brightness at 5, 10, 15, 20, 30, 40, 60 %, 3 s each | a still rainbow getting brighter |
| off | a rainbow forever at 8 MHz | – |
| **red** | something failed; the program has stopped | – |

Counting: LED 0 red, every 10th LED white, the last LED blue. On the 8x8 panel that is
LEDs 0, 10 … 60 white and LED 63 blue.

A clock **passes** when every color is right through to the last LED and the counting
marks are in the right places. The 16 and 32 MHz steps go beyond what a breadboard and a
74AHCT125 always manage, so a glitch there is a finding about the wiring as much as the
code; note the fastest clean clock. Two chip details can also show up there:

- SPIM3, the only SPI that reaches 16 and 32 MHz, has a known fault (nRF52840
  erratum 198): its data can be corrupted when the processor uses the same RAM during
  a transfer. Odd colors that come and go, at any clock, point to this.
- Nordic's datasheet recommends some pins near the radio for slow signals only; check
  its pin table for P1.13 and P1.15 before reading too much into the 32 MHz result.

Part 3 passes when the dimmest step (5 %) still shows every color of the rainbow.

Settings at the top of `src/main.rs`: `LedType` (`Apa102`, or `Sk9822`), `LED_COUNT`
(64 for the 8x8; try 144 for an SK9822 strip or 300 for an APA102 strip), `BRIGHTNESS`
and `POWER_LIMIT`.

## Wiring

Two signals, each through its own channel of the 74AHCT125:

| From | To |
|---|---|
| XIAO **D10** (P1.15, MOSI) | 74AHCT125 pin 2 (1A); pin 3 (1Y) → the DotStars' DI (data in) |
| XIAO **D8** (P1.13, SCK) | 74AHCT125 pin 5 (2A); pin 6 (2Y) → the DotStars' CI (clock in) |
| 74AHCT125 pins 1 and 4 (1OE, 2OE) | GND |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| XIAO GND | the DotStars' GND and the 5 V supply's GND |

Power the DotStars from their own 5 V supply and join the grounds. Keep the clock wire
short for the fast clocks. Check D8 and D10 against Seeed's pinout card.

## Setup and flashing

As in [`nrf52840-pwm`](../nrf52840-pwm#one-time-setup), with this example's name:

```sh
cargo objcopy --release -- -O ihex nrf52840-dotstar.hex
python uf2conv.py nrf52840-dotstar.hex -c -f 0xADA52840 -o nrf52840-dotstar.uf2
```

Double-tap RESET and copy `nrf52840-dotstar.uf2` onto the drive that appears (check the
SoftDevice in `INFO_UF2.TXT` against `memory.x` once), or use `cargo run --release` with
an SWD probe.

## Results

| Date | LEDs | Part | Result |
|---|---|---|---|
| 2026-11-08 | DotStar HD 8x8 (APA102, 64), through a 74AHCT125 on a breadboard | 1: clock sweep, 1 to 32 MHz | pass: clean at every clock, 32 MHz included |
| 2026-11-08 | same | 2: async at 8 MHz | pass |
| 2026-11-08 | same | 3: HD brightness, 5 to 60 % | pass |
