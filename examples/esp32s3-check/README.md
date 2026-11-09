# ESP32-S3 hardware check: every way to drive a strip (esp-hal)

Drives one WS2812B ("NeoPixel") strip from an Adafruit ESP32-S3 Feather every way
esp-hal can make the pulses, one after another, on the same pin. It's for checking
ledkit on the S3, and for comparing the ways side by side.

> **Pending hardware test.** This example builds in CI, but it has not been run on a
> real ESP32-S3 yet. If you try it, please report what you see, round by round.

## What you'll see

Seven rounds, then a length sweep:

| Round | Peripheral | How the data gets there |
|---|---|---|
| 1 | RMT, blocking | the processor refills the RMT's memory while it sends |
| 2 | RMT, async | the same, refilled when the async task is polled |
| 3 | SPI, `ClocklessSpi::new` at 3.2 MHz | the processor feeds the SPI's small buffer |
| 4 | SPI with DMA, `new_packed` at 4 MHz, blocking | DMA streams the whole frame, no gaps |
| 5 | SPI with DMA, `new_packed` at 4 MHz, async | the same, awaited |
| 6 | UART at 4 Mbaud, blocking | the processor feeds the serial port |
| 7 | UART at 4 Mbaud, async | the same, awaited |

Each round, about 21 seconds:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**; if it stays
   red, the gap between frames wasn't kept.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.

Then the **sweep**: the RMT sends frames of every length from 1 to 150 LEDs, a dim blue
bar that grows along the strip. Then the RMT rainbow runs forever. Press **RESET** to
run the whole check again.

A round **passes** when every color is right through to the last LED, the back-to-back
frames end blue, the counting marks are on LEDs 0, 10, 20 … 90 and 99, and the rainbow
moves smoothly with no flashes or stray colors.

There's no RMT with DMA here: esp-hal 1.2 doesn't support it. The ESP-IDF check,
[`esp32s3-esp-idf`](../esp32s3-esp-idf), has it.

## Wiring

| From | To |
|---|---|
| Feather pin **5** (GPIO5) | 74AHCT125 pin 2 (1A, input) |
| 74AHCT125 pin 3 (1Y, output) | 330 ohm resistor, then the strip's DIN |
| 74AHCT125 pin 1 (1OE) | GND (this switches the output on) |
| 74AHCT125 pin 14 (VCC) | 5 V |
| 74AHCT125 pin 7 (GND) | GND |
| Feather GND | strip GND and the 5 V supply's GND |

The ESP32-S3's pins work at 3.3 V; the 74AHCT125 lifts the data signal to the 5 V a
strip expects. Power the strip from its own 5 V supply, not from the Feather, and join
the grounds. The program limits the strip to 500 mA (`POWER_LIMIT`).

## One-time setup

```sh
cargo install espup espflash --locked
espup install --targets esp32s3
```

## Run it

```sh
cargo run --release
```

If espflash can't connect, hold **BOOT**, tap **RESET**, let go of BOOT and run again;
afterwards press RESET to start the program.

## Expected serial output

```text
ledkit ESP32-S3 check: 100 LEDs on GPIO5
ROUND 1: RMT, blocking
  red
  green
  blue
  white
  back to back: red, then blue (should end blue)
  counting: LED 0 red, every 10th white, last blue
  rainbow
ROUND 2: RMT, async
...
ROUND 7: UART, async (4000000 baud)
...
SWEEP: RMT frames of 1 to 150 LEDs
  25 LEDs
...
  150 LEDs
DONE. RMT rainbow forever; press RESET to run the check again.
```

## Settings

At the top of `src/main.rs`: `LED_COUNT` (100), `SWEEP_MAX` (150), `LedType`
(`Ws2812b`), `BRIGHTNESS` (20%), `POWER_LIMIT` (500 mA), and the speeds `RMT_HZ`,
`SPI_HZ`, `SPI_PACKED_HZ` and `BAUD`.

## Troubleshooting

- **One round glitches and the others don't**: that's a finding. Note the round, what
  it looked like (flashes, wrong colors past some LED, stuck colors) and the LED count.
- **Every round flickers with no color pattern**: suspect the wiring first (a loose
  breadboard contact, a missing ground, no level shifter).
- **Round 4 or 5 panics at start-up**: the DMA buffer must hold the whole frame
  (`SPI_DMA_LEN`); with many more LEDs it may not fit in RAM.
