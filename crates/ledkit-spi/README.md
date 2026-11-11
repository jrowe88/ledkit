# ledkit-spi

A ledkit backend that drives LEDs over any `embedded-hal` 1.0 SPI bus.

Many microcontrollers have an SPI peripheral, and a NeoPixel can be fed
from its data pin (MOSI) if the right bytes are sent. `ledkit-spi` works
out those bytes for your LED type and your SPI clock, so any board with a
HAL that implements the `embedded-hal` SPI traits works, blocking or async.

**ledkit never sets up the bus.** You configure the SPI peripheral, its
clock speed and its pin with your board's HAL, then hand over the bus and
the clock rate it really runs at.

## Which output do I use?

| Your LEDs | Wires | Use |
|---|---|---|
| NeoPixel type (WS2812B, SK6812 and friends) | one data wire, from MOSI | `ClocklessSpi` |
| DotStar type (APA102, SK9822) | data (MOSI) and clock (SCK) | `ClockedSpi` |

## Quick tour

On a real board it looks like this (the setup lines depend on your HAL,
here shaped like `esp-hal`):

```rust
use ledkit_core::prelude::*;
use ledkit_spi::{buffer_len, ClocklessSpi};

const LED_COUNT: usize = 30;
const SPI_HZ: u32 = 3_200_000;

// Your HAL: an SPI bus at SPI_HZ with the LED data wire on its MOSI pin.
let spi = Spi::new(peripherals.SPI2, Config::default().with_frequency(Rate::from_hz(SPI_HZ)))
    .unwrap()
    // The C6 DevKit's onboard LED; use your strip's data pin.
    .with_mosi(peripherals.GPIO8);

let output = ClocklessSpi::<Ws2812b, _, _>::new(
    spi,
    SPI_HZ,
    [0u8; buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)],
)
.unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Color::BLUE);
strip.show().unwrap();
```

The same code runs on your computer with a pretend bus. This one just
counts the bytes it is given:

```rust
use ledkit_core::prelude::*;
use ledkit_spi::{buffer_len, ClocklessSpi};

/// A pretend SPI bus that remembers how many bytes it was sent.
struct CountingBus(usize);

impl embedded_hal::spi::ErrorType for CountingBus {
    type Error = core::convert::Infallible;
}

impl embedded_hal::spi::SpiBus for CountingBus {
    fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        self.0 += words.len();
        Ok(())
    }
    fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

const LED_COUNT: usize = 30;
const SPI_HZ: u32 = 3_200_000;

let output = ClocklessSpi::<Ws2812b, _, _>::new(
    CountingBus(0),
    SPI_HZ,
    [0u8; buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)],
)
.unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Color::BLUE);
strip.show().unwrap();
let after_first = strip.output().bus().0;
strip.show().unwrap();

// The first frame starts with a reset gap; every frame after it is
// exactly the buffer.
assert_eq!(
    strip.output().bus().0 - after_first,
    buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)
);
```

## Coming from `smart-leds`?

With the `smart-leds` feature, `ClocklessSpi` and `ClockedSpi`
implement `smart-leds`' `SmartLedsWrite` (and `SmartLedsWriteAsync`), so
code written for `ws2812-spi` needs only a new constructor. Colors are
sent exactly as given: no brightness, gamma or dithering is added, so keep
using `smart-leds`' own `brightness()` and `gamma()` adaptors.

```rust
use ledkit_core::Ws2812b;
use ledkit_spi::{buffer_len, ClocklessSpi};
use smart_leds_trait::{SmartLedsWrite, RGB8};

const SPI_HZ: u32 = 3_200_000;

// Before: let mut ws = ws2812_spi::Ws2812::new(spi);
let mut ws = ClocklessSpi::<Ws2812b, _, _>::new(
    spi,
    SPI_HZ,
    [0u8; buffer_len::<Ws2812b>(8, SPI_HZ)],
)
.unwrap();

// The write call is unchanged.
ws.write([RGB8::new(255, 0, 0); 8]).unwrap();
```

## Which SPI clock?

A NeoPixel has no clock wire, so its bits are timed pulses. The SPI clock
sets how finely those pulses can be shaped, so only some speeds work.

| LEDs | SPI clock |
|---|---|
| WS2812/SK6812 family | 3.2 MHz is a good pick (2.3 to 6.8 MHz works for a WS2812B) |
| WS2811 at 400 kHz | make a guess with `ClocklessSpi::new`; the error message names the clocks that work |
| DotStars | any speed; 1 to 40 MHz all ran clean on an ESP32-C6 with a 64-LED DotStar HD 8×8 on a breadboard. Long strips and long wires may need slower |

**Pass the clock your bus really runs at.** Many HALs round the speed you
ask for to what the hardware can divide down to (a Raspberry Pi asking for
3.2 MHz gets about 3.1 MHz). ledkit builds its pulses from the number you
give it, so use the real speed if your HAL tells you, or work it out. A
few percent of difference is fine; a wrong guess by a lot is not.

## `new` or `new_packed`?

`ClocklessSpi::new` sends every LED bit as half a byte or a whole byte
(4 or 8 SPI bits per LED bit). It works on every SPI peripheral, even
when there are tiny pauses between bytes. Use it unless it refuses your
clock.

`ClocklessSpi::new_packed` squeezes the bits together, which makes the
buffer smaller (at 4 MHz a WS2812B needs 5 SPI bits per LED bit instead of
8). It does not add clock speeds for the WS2812B. Use it only when your SPI sends bytes
back to back, with no pauses (typically DMA). On a bus with pauses between
bytes, a pause can land inside a high pulse and corrupt a color. Use
`ClocklessSpi::new` otherwise.

## Sizing the buffer

The driver needs a buffer to build the SPI bytes in. Make it exactly big
enough with `buffer_len` (or `buffer_len_packed` for `new_packed`).
They are `const fn`s, so they size an array, and a clock that can't drive
your LED is a build error instead of a surprise later:

```rust
use ledkit_core::Ws2812b;
use ledkit_spi::buffer_len;

const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30, 3_200_000);
let buffer = [0u8; BUFFER_LEN];
assert_eq!(buffer.len(), 473);
```

## HALs that cap one transfer

Some SPI drivers can only send so many bytes at once. The Teensy 4's
LPSPI, for example, sends at most 512 bytes per write, which is only
about 33 LEDs. Add `with_max_write_len`
and ledkit cuts the frame into pieces of that size:

```rust
let output = ClocklessSpi::<Ws2812b, _, _>::new(spi, SPI_HZ, buffer)
    .unwrap()
    .with_max_write_len(512);
```

This is safe with `ClocklessSpi::new` as long as each pause between pieces stays
well below the LED's reset time (280 µs for the WS2812B): then a pause only
makes a low phase a little longer. A blocking HAL on a microcontroller, like
the Teensy's, starts the next piece right away. Under an operating system,
or with an async executor that may run other tasks between pieces, a pause
can last longer than that, and the LEDs latch half a frame; don't chunk
there. It is **not** safe with `ClocklessSpi::new_packed`, where a pause can land
inside a high pulse.

## Dim DotStars: HD brightness

DotStars have a 5-bit brightness level in every LED, on top of the 8-bit
color channels, and `ClockedSpi` sends it. So `Strip` uses "HD" brightness
by default: in the dim half of the range it lowers the 5-bit level and keeps
the channels wide (at 10%, a level of 2 out of 31 with channels up to about
47), so dim colors stay smooth and true. Above about half brightness it
sends the same bytes as plain dimming. Plain dimming would leave each
channel only 0 to 3 at 10%, and soft colors would round to off. HD
also lights LEDs that put their own curve on the color values: the
APA102-2020s of an Adafruit DotStar HD 8×8 draw about (color / 255)^2.6 of
full current (measured on an ESP32-S3) but follow the 5-bit level almost in
a straight line, so 10% is dark without HD and lit with it. At low 5-bit
levels their white turns a little bluish.
The `esp32c6-dotstar-hd` example shows the two side by side.

The APA102 makes its 5-bit level with a slow flicker, which some cameras
show as banding at low brightness (the SK9822 doesn't have it). If you film
your LEDs, you can turn HD off:

```rust,ignore
strip.set_hd_brightness(false);
```

## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| ESP32-C6 dev kit (chip rev v0.2), 2026-10-30 | the onboard LED | passed: red, green, blue and white in order, a smooth rainbow, 20% brightness |
| ESP32-C6 (SPI2, 3.2 MHz, esp-hal blocking) | WS2812-type strip, 100 LEDs, through a 74AHCT125 | ✅ pass (2026-10-31) with `new` and with `new_packed`: solid colors, a counting pattern with the last LED marked, and a rainbow at 50 frames a second, no glitches |
| Raspberry Pi Zero W (Linux spidev, 32-bit Pi OS) | WS2812-type strip, 100 LEDs | ✅ pass (2026-11-01): colors and a clean rainbow at 3.2 MHz, with `core_freq=250` pinning the SPI clock |
| Teensy 4 (LPSPI) | NeoPixel 8×8 matrix (64 LEDs) | ✅ pass (2026-10-31), with the two SPI settings in `examples/teensy4-spi` |
| Teensy 4 (LPSPI) | WS2812-type strip, 100 LEDs | ✅ pass (2026-11-01): colors, counting and a fast rainbow clean through to LED 99, with each frame sent in three 512-byte writes (`with_max_write_len`), so the pauses between writes don't break the strip |
| Teensy 4 (LPSPI) | DotStar HD 8×8 (APA102-2020, 64 LEDs) | ✅ pass (2026-10-31); the `0xFF` end frame was the fix |
| ESP32-C6 (SPI2, 1 to 40 MHz requested) | DotStar HD 8×8 (APA102-2020, 64 LEDs) | ✅ pass (2026-10-31), clean at every speed tried; dark at 10% brightness without HD, lit with it (see "Dim DotStars" above) |
| ESP32-C6 (SPI2, 4 MHz) | SK9822 strip, 144 LEDs | ✅ pass (2026-10-31): each frame latched at once through to LED 143 with the zero end frame, and HD brightness worked |
| ESP32-C6 (SPI2, 4 MHz) | DotStar strip (APA102-type 5050), 300 LEDs | ✅ pass (2026-11-01): sized to exactly 300, the last LED lit, every frame latched at once through to LED 299 with the `0xFF` end frame (19 bytes), and an HD rainbow ran clean |
| Seeed XIAO nRF52840 (SPIM2, 4 MHz, embassy-nrf) | WS2812B strip, 100 LEDs, through a 74AHCT125 | ✅ pass (2026-11-08) with `new_packed` (blocking and async) and `new`: colors, back-to-back frames, counting and a rainbow clean through to LED 99. The nRF's SPIM needs a clock pin even for NeoPixels: without one (`new_txonly_nosck`) it never starts and `show()` never returns; `examples/nrf52840-spi` puts it on an unconnected pin |
| Seeed XIAO nRF52840 (SPIM3, 1 to 32 MHz, embassy-nrf) | DotStar HD 8×8 (APA102-2020, 64 LEDs), through a 74AHCT125 | ✅ pass (2026-11-08): colors and counting clean at every clock up to 32 MHz, async at 8 MHz, and HD brightness down to 5 % |
| Adafruit ESP32-S3 Feather (SPI2 with DMA, 3.2 MHz, esp-idf-hal `SpiBusDriver`, ESP-IDF 5.5) | WS2812B strip, 100 LEDs, through a 74AHCT125 | ✅ pass (2026-11-11), `examples/esp32s3-esp-idf` round 3 with `new`: colors, back-to-back frames, counting, a fast rainbow and frames of 1 to 150 LEDs, clean through to LED 99 |
| Feather RP2040 SCORPIO (rp2040-hal SPI0, data GPIO19 / clock GPIO18 through the board's 5 V outputs) | DotStar HD 8×8 (APA102-2020, 64 LEDs) | ✅ pass (2026-11-08), `examples/rp2040-pio` round 5: colors, back to back, counting and a rainbow at 4 MHz, then the counting pattern clean at every step of a 1 to 32 MHz sweep (the RP2040 makes 31.25 MHz for 32) |
| Raspberry Pi Pico 2 W (RP2350, Arm and RISC-V cores; rp235x-hal SPI0, data GPIO19 / clock GPIO18 through a 74AHCT125) | DotStar HD 8×8 (APA102-2020, 64 LEDs) | ✅ pass (2026-11-09), `examples/rp2350-pio` round 5: the pattern at 4 MHz, then the counting pattern clean at every step of the 1 to 32 MHz sweep, on both core types |
| Adafruit ESP32-S3 Feather (SPI2, esp-hal) | WS2812B strip, 100 LEDs, data straight from GPIO5 on a 4.3 V supply | ✅ pass (2026-11-09), `examples/esp32s3-check` rounds 3 to 5: `new` at 3.2 MHz, and `new_packed` at 4 MHz with DMA, blocking and async. The first run showed LED 0 green on the first frame after setting up the bus: the S3's SPI holds its data pin high until its first transfer, and the strip read that as an extra bit. Fixed by starting the first frame with a reset gap |
| Adafruit ESP32-S3 Feather (SPI2, 4 MHz) | DotStar HD 8×8 (APA102-2020, 64 LEDs) | ✅ pass (2026-11-09), `examples/esp32s3-dotstar-hd`: HD and plain side by side, as on the ESP32-C6 |
| other boards (`ClockedSpi`) | | untested on hardware |

Everything else is covered by host tests with a pretend SPI bus. Examples
are in the `examples` folder of the repository: `esp32c6-spi`,
`esp32c6-dotstar-hd`, `linux-spidev` and `teensy4-spi`. If you try one on your hardware, please
report what you see.

## Features

- `smart-leds`: `smart-leds` write traits for the outputs.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
