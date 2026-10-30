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
    .with_mosi(peripherals.GPIO7);

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

assert_eq!(strip.output().bus().0, buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ));
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
| DotStars | anything up to 8 MHz for now |

**Pass the clock your bus really runs at.** Many HALs round the speed you
ask for to what the hardware can divide down to (a Raspberry Pi asking for
3.2 MHz gets about 3.1 MHz). ledkit builds its pulses from the number you
give it, so use the real speed if your HAL tells you, or work it out. A
few percent of difference is fine; a wrong guess by a lot is not.

## `new` or `new_packed`?

`ClocklessSpi::new` sends every LED bit as a whole number of bytes
(4 or 8 SPI bits per LED bit). It works on every SPI peripheral, even
when there are tiny pauses between bytes. Use it unless it refuses your
clock.

`ClocklessSpi::new_packed` squeezes the bits together, which allows more
clock speeds (for example 4 MHz). The cost: a pause between bytes can land
in the middle of a pulse and corrupt a color, so use it only when your
SPI sends bytes back to back (typically with DMA).

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
about 30 LEDs. Add `with_max_write_len`
and ledkit cuts the frame into pieces of that size:

```rust
let output = ClocklessSpi::<Ws2812b, _, _>::new(spi, SPI_HZ, buffer)
    .unwrap()
    .with_max_write_len(512);
```

This is safe with `ClocklessSpi::new`: a short pause between pieces only
makes a low phase a little longer, far shorter than the pause an LED
needs to latch. It is **not** safe with `ClocklessSpi::new_packed`,
where a pause can land inside a high pulse.

## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| ESP32-C6 `DevKit` (chip rev v0.2), 2026-10-30 | the onboard LED | passed: red, green, blue and white in order, a smooth rainbow, 20% brightness |
| Raspberry Pi (Linux spidev) | | untested on hardware; builds in CI |
| Teensy 4 (LPSPI) | | untested on hardware; builds in CI, hardware test pending |
| DotStars (`ClockedSpi`) | | untested on hardware |

Everything else is covered by host tests with a pretend SPI bus. Examples
are in the `examples` folder of the repository: `esp32c6-spi`,
`linux-spidev` and `teensy4-spi`. If you try one on your hardware, please
report what you see.

## Features

- `smart-leds`: `smart-leds` write traits for the outputs.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
