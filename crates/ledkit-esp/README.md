# ledkit-esp

A ledkit backend that drives NeoPixels with the ESP32's RMT peripheral, through
`esp-hal` (bare metal, `no_std`), blocking or async.

These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
and the pulse widths tell `0` from `1`. The RMT (Remote Control Transceiver)
peripheral generates these pulses in hardware, so the timing is unaffected by
interrupts.

`ledkit-esp` is for bare-metal projects built on `esp-hal`. For projects built on
ESP-IDF, use `ledkit-esp-idf` instead.

## Which ESP crate do I use?

| Your project | Use |
|---|---|
| esp-hal: bare metal, `no_std`, stable Rust, works with Embassy | `ledkit-esp` |
| ESP-IDF: `std`, Wi-Fi through `esp-idf-svc`, nightly Rust | `ledkit-esp-idf` |
| Any board with an SPI bus (including ESP32 boards, no RMT needed) | `ledkit-spi` |

## Quick tour

This can only run on an ESP32, so the snippet is not run by `cargo test`. It is
written for the ESP32-C6 (pick your chip with a Cargo feature, see "Features"):

```rust
use esp_hal::rmt::{PulseCode, Rmt};
use esp_hal::time::Rate;
use ledkit_core::prelude::*;
use ledkit_esp::{buffer_len, RmtOutput};

const LED_COUNT: usize = 8;

let peripherals = esp_hal::init(esp_hal::Config::default());

let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();
// The RMT's clock. Pass it to `RmtOutput::new` below.
let rmt_hz = rmt.frequency().as_hz();

let output = RmtOutput::<Ws2812b, _, _>::new(
    rmt.channel0,
    // The C6 DevKit's onboard LED is on GPIO8; use your strip's data pin.
    peripherals.GPIO8,
    rmt_hz,
    [PulseCode::default(); buffer_len::<Ws2812b>(LED_COUNT)],
)
.unwrap();
let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
strip.fill(Rgb::BLUE);
strip.show().unwrap();
```

For async, create the driver with `Rmt::new(..).unwrap().into_async()` and use
`strip.show_async().await` (the `OutputAsync` side of ledkit).

## Pass the frequency you gave `Rmt::new`

**ledkit never sets up the RMT.** You create it with `Rmt::new` and a clock rate, and
the channel you hand over doesn't tell ledkit that rate. So pass it to
`RmtOutput::new` as `rmt_hz`; the simplest way is `rmt.frequency().as_hz()`, which
is the rate `esp-hal` really set up. ledkit divides it down to 10 MHz ticks (100 ns
each), which is why it must be 10 MHz times a whole number from 1 to 255, such as
80 MHz (divider 8). Any other value is refused with `Error::ClockNotUsable`
instead of sending slightly wrong pulses.

ESP32-H2 isn't supported yet. Its RMT runs from a 32 MHz clock, which can't make
ledkit's 10 MHz tick.

## Which channel?

Any transmit channel works, but the number of them differs by chip, and the
channel you hand over must be a *transmit* one (`esp-hal` checks this at compile
time):

| Chip | Transmit channels |
|---|---|
| ESP32 | 0 to 7 |
| ESP32-S2 | 0 to 3 |
| ESP32-S3 | 0 to 3 |
| ESP32-C3, C6 | 0 to 1 |

The rest are for receiving. See `esp-hal`'s `rmt` documentation for your chip.

## Wiring

One data wire from a GPIO pin to the LED's data-in, a shared ground, and 5 V power
for the LEDs. The board's 3.3 V signal is fine for most WS2812B strips; if the first
LED flickers, add a level shifter. On the ESP32-C6-DevKitC-1 the onboard LED is on
GPIO8.

## Sizing the buffer

The driver builds the whole frame as RMT pulse codes first: one per color bit, plus
two (the reset gap and an end marker). Make the buffer exactly big enough with
`buffer_len`, which is a `const fn`, so it sizes an array:

```rust
use ledkit_core::Ws2812b;
use ledkit_esp::buffer_len;

// 30 LEDs * 3 colors * 8 bits + 2 = 722 pulse codes
const BUFFER_LEN: usize = buffer_len::<Ws2812b>(30);
let buffer = [PulseCode::default(); BUFFER_LEN];
```

The pulse codes are 4 bytes each, so 30 WS2812B LEDs take about 2.9 kB of RAM.

## The reset gap

The LEDs latch the new colors once the data line has been low for the chip's reset
time (280 us for the WS2812B). ledkit appends that gap to every frame as a long low
pulse, and sending only finishes when the RMT has sent it. Two `show()` calls back
to back therefore both take effect. Between frames the output is held low.

## Coming from `smart-leds`?

With the `smart-leds` feature, `RmtOutput` implements `smart-leds`'
`SmartLedsWrite` (blocking) and `SmartLedsWriteAsync` (async), so code written for
`esp-hal-smartled` needs only a new constructor. Colors are sent exactly as given:
no brightness, gamma or dithering is added, so keep using `smart-leds`' own
`brightness()` and `gamma()` adaptors.

```rust
use ledkit_core::Ws2812b;
use ledkit_esp::{buffer_len, RmtOutput};
use smart_leds_trait::{SmartLedsWrite, RGB8};

let mut ws = RmtOutput::<Ws2812b, _, _>::new(
    rmt.channel0,
    peripherals.GPIO8,
    rmt_hz,
    [PulseCode::default(); buffer_len::<Ws2812b>(8)],
)
.unwrap();
ws.write([RGB8::new(255, 0, 0); 8]).unwrap();
```

## Building

This crate drives `esp-hal`'s RMT, which `esp-hal` still marks as unstable, so the
crate turns on `esp-hal`'s `unstable` feature for you. You pick the chip with one
Cargo feature (see below) and your own project provides the rest of the `esp-hal`
setup: target, linker script and the `rt` feature. Copy it from the
`examples/esp32c6-spi` project of the repository, which uses the same `esp-hal`.

The ESP32-C3 and C6 are RISC-V chips and build with plain stable Rust. The
ESP32, S2 and S3 use Xtensa cores and need the `esp` toolchain (installed by
`espup`).

## Tested on hardware

| Board | LEDs | Result |
|---|---|---|
| ESP32-C6 dev kit | the onboard LED (`examples/esp32c6-rmt`) | pending |

Everything else is covered by host tests of the shared pulse encoder in
`ledkit-core` (`fill_rmt_words`). If you try it on your hardware, please report what
you see.

## Features

- `esp32`, `esp32c3`, `esp32c6`, `esp32s2`, `esp32s3`: the chip. Pick
  exactly one; `esp-hal` refuses to build with none or two.
- `smart-leds`: the `smart-leds` write traits for `RmtOutput`.
