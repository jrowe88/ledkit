# ledkit-uart

A [ledkit](https://github.com/jrowe88/ledkit) backend that drives one-wire LEDs
(NeoPixels, WS2812B, SK6812 and friends) from a serial port (UART). Any board whose HAL
implements the `embedded-io` 0.7 `Write` trait works, blocking or async. `no_std`, no heap,
no `unsafe`.

**ledkit never sets up the port.** You configure the serial port, its baud, its pin and its
output inversion with your board's HAL, then hand over the port and the baud it really runs
at.

## How it works

A serial byte on the wire is 10 equal slots: a start bit, 8 data bits and a stop bit. With
the port's transmit line **inverted**, the line rests low, the start bit is high and the
stop bit is low. That is just what a NeoPixel wants: low between bits, and low for its latch.
Each serial byte carries two LED bits of 5 slots each, a few high slots and then low ones.
How many slots are high tells a `0` from a `1`.

ledkit works out that pattern from your LED type's datasheet timing and your port's real
baud, and checks all four pulse lengths against the datasheet. A baud that can't drive your
LEDs is refused with an error that names the nearest bauds that can, instead of a
flickering strip.

For a WS2812B at 4 Mbaud, a slot is 250 ns: a `0` is 1 slot high and 4 low (250 ns, then
1000 ns), and a `1` is 3 slots high and 2 low (750 ns, then 500 ns). Each LED takes 12 bytes
of buffer (16 for RGBW).

After each frame, ledkit waits the LED's reset time (280 µs for a WS2812B) with the delay you
give it, so the LEDs show the frame. A serial port can't send "all low" bytes the way SPI
can, so the latch has to be a pause.

## Which baud?

| LED type | Bauds that work | Use |
|---|---|---|
| WS2812B | 2.25 to 4.4 Mbaud | 4 Mbaud |
| WS2812 | 2.4 to 4.4 Mbaud | 4 Mbaud |
| SK6812, SK6812 RGBW | 2.7 to 4.4 Mbaud | 4 Mbaud |
| WS2811 (800 kHz) | 2.7 to 4.4 Mbaud | 4 Mbaud |
| WS2813, WS2815 | 2.25 to 2.65, or 3.0 to 3.3 Mbaud | 3 Mbaud |
| APA106 | 2.0 to 2.45, or 2.65 to 3.3 Mbaud | 3 Mbaud |
| WS2811 (400 kHz) | 1.55 to 1.9 Mbaud | 1.6 Mbaud |

(Worked out from ledkit's datasheet windows in 50 kHz steps.)

## Use it

```rust,ignore
use ledkit_core::{Rgb, Strip, Ws2812b};
use ledkit_uart::{buffer_len, ClocklessUart};

const LED_COUNT: usize = 30;
const BAUD: u32 = 4_000_000;

// `uart` is your HAL's serial port: 8N1, transmit inverted, at BAUD.
// `delay` is anything that implements `embedded_hal::delay::DelayNs`.
let output = ClocklessUart::<Ws2812b, _, _, _>::new(
    uart,
    BAUD,
    delay,
    [0u8; buffer_len::<Ws2812b>(LED_COUNT, BAUD)],
)
.unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Rgb::RED);
strip.show().unwrap();
```

For async (`strip.show_async().await`), pass a port that implements `embedded_io_async::Write`
and a delay that implements `embedded_hal_async::delay::DelayNs`. That is also how to use
DMA: if your HAL's async serial write uses DMA, so does ledkit.

## Wiring

- The serial port's **transmit** pin goes through a **74AHCT125** level shifter (a 3.3 V
  signal is too weak for 5 V LEDs to read reliably) to the strip's **DIN**.
- The port's receive pin isn't used.
- Power the LEDs from a 5 V supply, and join its ground to the board's.

**The output must be inverted.** Most serial ports can do that themselves (the Teensy 4's
LPUART, the ESP32's GPIO matrix). Without it, the line rests high and the LEDs never latch.

## Pauses between bytes

If the port pauses between bytes, the line sits low, which only lengthens the low part of an
LED bit. That is harmless while the low part stays inside the LED's limit: up to about
4.5 µs for a WS2812B. DMA and hardware FIFOs send without pauses; a very slow blocking
driver might not.

## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| ESP32-C6 (UART1, 4 Mbaud, inverted in the GPIO matrix) | WS2812-type strip, 100 LEDs | ✅ pass (2026-11-01): colors, counting and a fast rainbow clean through to LED 99, blocking and async |
| Teensy 4 (LPUART6, 4 Mbaud from a 24 MHz UART clock, blocking and DMA) | WS2812-type strip, 100 LEDs | ✅ pass (2026-11-01): full cycles of colors, counting and a rainbow in both rounds, through an SN74LVC245AN buffer |

Examples are in the `examples` folder of the repository: `esp32c6-uart` and `teensy4-uart`.
If you try one on your hardware, please report what you see.

## Features

- `smart-leds`: `smart-leds` write traits for the output.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
