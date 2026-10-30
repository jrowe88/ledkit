# ledkit-core

Core of ledkit: the LED chip catalogue, signal encoders and the pixel buffer.

`ledkit-core` knows what NeoPixels, DotStars and friends expect on the
wire, keeps your colors in a `Strip`, and hands finished frames to an
`Output` (a driver for your board). It works without `std` or an
allocator. It is part of the ledkit family of crates.

## Quick tour

This runs on your computer using `testing::MemoryOutput`, which just
remembers the last frame. Real projects use a backend crate for their
board instead (coming in milestone 3); the rest of the code stays the same.

Start every program with `use ledkit_core::prelude::*;`. It brings in
the strip, the chip types, colors (including `Hue` and `Hsv`),
brightness, the gamma tables and `Dim`, so you don't have to hunt for
where each one lives.

```rust
use ledkit_core::prelude::*;
use ledkit_core::testing::MemoryOutput;

const LED_COUNT: usize = 8;
type LedType = Ws2812b; // what is printed on your LEDs

let mut strip = Strip::<LedType, LED_COUNT, _>::new(MemoryOutput::<_, LED_COUNT>::new());

// Keep it comfortable to look at, and keep the power supply safe.
strip.set_brightness(0.3);
strip.set_power_limit(Milliamps(1500));

strip.fill(Color::BLUE);
strip.show().unwrap();

// A rainbow: spread the color wheel (65536 steps) over the strip.
for index in 0..LED_COUNT {
    let hue = Hue((index * 65536 / LED_COUNT) as u16);
    strip.set(index, Rgb::from_hue(hue));
}
strip.show().unwrap();

assert_eq!(strip.output().frames_written, 2);
```

## Brightness, gamma and dithering, in plain words

Your eyes do not see light in a straight line: an LED at half power looks
much brighter than "half". ledkit takes care of that for you.

- **Brightness is perceptual.** `strip.set_brightness(0.5)` looks about
  half as bright. If you want the raw hardware level instead (0 to 255,
  no curve), use `Brightness::exact`. Changing it every frame is
  fine and intended: that is how you make a breathing or fading strip.
  Dithering and the power limit are worked out afresh on every `show()`,
  so they follow along.
- **Fading single pixels** (twinkles, comet tails) uses the same curve:
  `color.dimmed(0.5)` looks half as bright (see `Dim`).
- **Colors are sent as you typed them.** `Rgb::new(255, 128, 0)` goes out
  unchanged, and rainbows and fades made from computed colors look right
  without extra work.
- **Color gamma is opt-in.** Colors that come from images, color pickers
  or web colors are made for screens, and dim shades look too bright on
  LEDs. For those, call `strip.set_color_gamma(&GAMMA_2_6)` (the table is
  in the `prelude`). Leave it off for colors you pick or compute
  yourself; a gamma curve would darken them and lose detail in the dark
  shades.
- **Dithering is on by default.** At low brightness there are few
  brightness steps, so ledkit varies the rounding a little from frame to
  frame, which makes dim colors look smooth. It works best when `show()`
  is called quickly, about 100 times a second or more, as animations do.
  If you call `show()` slowly or irregularly at low brightness, the
  rounding changes slowly enough to see as a gentle twinkle. In that case,
  or if you see shimmer in photos or video, turn it off with
  `strip.set_dithering(false)`.

## Which chip type do I use?

Find the name printed on your product (or its datasheet).

| Printed on the product | Type |
|---|---|
| NeoPixel strip, ring, matrix, "WS2812B" | `Ws2812b` |
| "WS2812" (older, 5 V) | `Ws2812` |
| "WS2813" (backup data line) | `Ws2813` |
| "WS2815", 12 V strip | `Ws2815` |
| "WS2811", 12 V pixel strings or bullet lights | `Ws2811` |
| "WS2811" at 400 kHz (very old strings) | `Ws2811Slow` |
| "SK6812", "SK6812 Mini", "SK6805" | `Sk6812` (aliases `Sk6812Mini`, `Sk6805`) |
| "SK6812 RGBW", RGBW NeoPixel | `Sk6812Rgbw` |
| "APA106", 8 mm through-hole pixels | `Apa106` |
| "DotStar", "APA102", "APA102C" | `Apa102` |
| "SK9822" (APA102 clone) | `Sk9822` |
| SparkFun LuMini | `Apa102_2020` (alias of `Apa102`) |

## How ledkit talks to your LEDs

NeoPixel-style chips have a single data wire, and the bits are carried by
very precise pulse lengths. DotStar-style chips have a data wire and a
clock wire, and are much more relaxed. A backend crate sends your frame
using one of these techniques, and `ledkit-core` provides the pieces:

- `wire` turns pixels into the bytes a chip expects (including the
  chip's color order, such as green-red-blue for the WS2812B).
- `encode` turns a chip's timing into what a peripheral needs: SPI bit
  patterns that imitate the pulses (`encode::SpiEncoding`), or pulse
  lengths in clock ticks for the ESP32 RMT or RP2040 PIO
  (`encode::PulseTicks`).
- DotStars are sent as plain SPI frames with `wire::clocked_frame`.

You only meet these when writing a backend; see `Output`.

## Writing a custom chip

If your LED is not in the catalogue, describe it by implementing `Chip`.
The timings below come from the chip's datasheet: each `Phase` is
`(typical, minimum, maximum)` in nanoseconds.

```rust
use ledkit_core::{ChannelOrder, Chip, ClocklessTiming, Phase, Protocol, Rgb};

/// A made-up one-wire LED.
struct MyLed;

impl Chip for MyLed {
    type Pixel = Rgb;
    const NAME: &'static str = "MyLed";
    const ORDER: ChannelOrder = ChannelOrder::GRB;
    const PROTOCOL: Protocol = Protocol::Clockless(ClocklessTiming {
        zero_high: Phase::new(400, 220, 450),
        zero_low: Phase::new(900, 580, 5000),
        one_high: Phase::new(800, 580, 1000),
        one_low: Phase::new(600, 450, 5000),
        reset_us: 280,
    });
}

assert_eq!(MyLed::NAME, "MyLed");
```

## Features

- `smart-leds`: interoperability with the `smart-leds-trait` crate.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option. See NOTICE for attributions.
