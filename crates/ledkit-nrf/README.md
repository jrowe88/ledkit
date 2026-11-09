# ledkit-nrf

A ledkit backend that drives NeoPixels with the nRF52840's PWM peripheral and
EasyDMA, through `embassy-nrf` (bare metal, `no_std`), blocking or async.

These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
and the length of the high pulse tells `0` from `1`. The nRF52's PWM can play a list
of "duty" values from RAM, one per PWM period, without the processor's help
(EasyDMA reads them). ledkit sets the PWM period to one LED bit and writes one duty
value per bit, so the whole frame goes out in hardware and interrupts or the radio
can't disturb it.

## Which nRF backend do I use?

| You want | Use |
|---|---|
| NeoPixels (WS2812B, SK6812, ...) on any pin, exact timing, no CPU | `ledkit-nrf` (this crate) |
| NeoPixels with less RAM (15 bytes per LED instead of 48) | `ledkit-spi`'s `ClocklessSpi::new_packed` on a SPIM at 4 MHz |
| DotStars (APA102, SK9822), which have a clock wire | `ledkit-spi`'s `ClockedSpi` on a SPIM |

The nRF's serial port (UARTE) can't drive NeoPixels: it tops out at 1 Mbaud and
can't invert its output, so `ledkit-uart` refuses it.

## Quick tour

This can only run on an nRF52840, so the snippet is not run by `cargo test`:

```rust
use ledkit_core::prelude::*;
use ledkit_nrf::{buffer_len, PwmOutput};

const LED_COUNT: usize = 8;

let p = embassy_nrf::init(Default::default());
let output = PwmOutput::<Ws2812b, _>::new(
    p.PWM0,
    p.P1_15, // pin D10 on the Seeed XIAO nRF52840
    [0u16; buffer_len::<Ws2812b>(LED_COUNT)],
)
.unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Rgb::BLUE);
strip.show().unwrap();
```

In an async (Embassy) program use `strip.show_async().await` instead (the
`OutputAsync` side of ledkit, feature `async`, on by default).

Don't cancel `show_async()`, for example by dropping it inside `select`. Dropping it
stops the PWM half way through the frame, and the next frame may then join onto it
without a reset gap, so the colors come out wrong. Let each `show_async().await`
finish.

## Which PWM, which pin?

The nRF52840 has four PWM peripherals, `PWM0` to `PWM3`, and each can drive any GPIO
pin. Use `PWM0` unless something else in your program already does. One `PwmOutput`
drives one strip.

On the Seeed XIAO nRF52840 the pins marked on the board are:

| Board pin | D0 | D1 | D2 | D3 | D4 | D5 | D6 | D7 | D8 | D9 | D10 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `embassy-nrf` name | `P0_02` | `P0_03` | `P0_28` | `P0_29` | `P0_04` | `P0_05` | `P1_11` | `P1_12` | `P1_13` | `P1_14` | `P1_15` |

Avoid `P0_09` and `P0_10`: they are the NFC antenna pins and don't work as GPIO
until NFC is turned off in the chip's settings (UICR). The XIAO's own RGB LED is on
`P0_26`, `P0_30` and `P0_06`. Check the pins against your board's pinout card.

The pin is driven with the nRF's "high drive" strength for sharp edges on a long
wire. The nRF52840's pins are 3.3 V; most WS2812B strips read that, but if the first
LED flickers, add a 5 V level shifter such as a 74AHCT125.

## Sizing the buffer

The driver builds the whole frame in RAM first: one 16-bit duty word per color bit,
plus the reset gap (below). `buffer_len` is a `const fn`, so it sizes an array:

```rust
use ledkit_core::Ws2812b;
use ledkit_nrf::buffer_len;

// 30 LEDs * 3 colors * 8 bits + 205 reset words = 925 words
const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
let buffer = [0u16; BUFFER_LEN];
```

That is 48 bytes per RGB LED and 64 per RGBW LED: 100 WS2812B LEDs take about
5.2 KB, 300 take about 15 KB, of the nRF52840's 256 KB. `B` can also be a
`&'static mut [u16; N]` (for example from the `static_cell` crate), which avoids
moving a large array through the stack. The buffer must be in RAM (EasyDMA can't read
flash), which an array or a `static mut` always is.

One frame can be at most 32 767 words (the PWM's limit): about 1350 RGB LEDs or 1020
RGBW LEDs. A longer frame is refused with `Error::FrameTooLong`.

## Timing

The PWM counts a 16 MHz clock (62.5 ns per tick). Every bit has the same length, so
ledkit looks for the bit length whose `0` and `1` both fit the chip's datasheet
(`ClocklessTiming::equal_period_ticks`):
22 ticks (1.375 us) for the WS2812B, 20 ticks (1.25 us) for the WS2812, WS2813 and
WS2811, 19 for the SK6812. This happens when the program is compiled.

The PWM's clock follows the chip's main clock. Start the 32 MHz crystal for exact
timing (`config.hfclk_source = HfclkSource::ExternalXtal` before
`embassy_nrf::init`); without it the internal oscillator is still close enough for
the LEDs.

## The reset gap

The LEDs latch the new colors once the data line has been low for the chip's reset
time (280 us for the WS2812B). ledkit ends every frame with that many all-low duty
words (205 for the WS2812B: one more than the gap needs, because the PWM reports the
end of a frame as its last word starts), and `show()` returns only after they have
played, so two `show()` calls back to back both take effect. Between frames the pin
is held low.

## Async

`show_async()` starts the frame, sleeps with an `embassy-time` timer for exactly as
long as the frame takes (other tasks run meanwhile), then checks that the PWM has
finished. So an async program needs an `embassy-time` driver, which every Embassy
program has (for `embassy-nrf`, its `time-driver-rtc1` feature). A blocking-only
program can turn off the default `async` feature and needs no time driver.

## Coming from `smart-leds`?

With the `smart-leds` feature, `PwmOutput` implements `smart-leds`'
`SmartLedsWrite` (and `SmartLedsWriteAsync` with `async`), so code written for
`nrf-smartled` needs only a new constructor. Colors are sent exactly as given: no
brightness, gamma or dithering is added, so keep using `smart-leds`' own
`brightness()` and `gamma()` adaptors.

## Building

Pick the chip with a Cargo feature (`nrf52840`). Your own project provides the rest of
the `embassy-nrf` setup: the `rt` feature, a time driver, `memory.x` and the target
`thumbv7em-none-eabihf`, which builds with plain stable Rust. Copy it from the
repository's `examples/nrf52840-pwm`, which uses this crate.

## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| Seeed XIAO nRF52840 (D10, through a 74AHCT125) | WS2812B strip, 100 LEDs | blocking, async (with a busy task) and the 1–150 LED length sweep pass, 2026-11-08 |

The duty words and timings are also covered by host tests in `ledkit-core`
(`equal_period_ticks`, `fill_pwm_words`). If you try another board or LED, please report
what you see.

## Features

- `nrf52840`: the chip. Required.
- `async` (default): `OutputAsync` for `PwmOutput`; needs an `embassy-time` driver.
- `smart-leds`: the `smart-leds` write traits for `PwmOutput`.
