//! A first ledkit program on an ESP32-S3 with ESP-IDF (`std`): a rainbow on a NeoPixel
//! strip, about 60 frames a second.
//!
//! It uses only the `ledkit` crate and esp-idf-hal. `ledkit::esp_idf::strip` sets up
//! the RMT peripheral, and `Frames` paces the loop.

use esp_idf_hal::delay::Delay;
use esp_idf_hal::peripherals::Peripherals;
use ledkit::prelude::*;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// Which kind of LED it is (printed on the strip or in the product listing).
type LedType = Ws2812b;

/// How many LEDs there are.
const LED_COUNT: usize = 100;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, as ledkit estimates it.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How many frames a second to draw.
const FRAMES_PER_SECOND: u16 = 60;

// The data wire goes to GPIO5 (through a 74AHCT125 level shifter). To use another
// pin, change `gpio5` below.

fn main() -> Result<(), ledkit::esp_idf::Error> {
    // Applies fixes ESP-IDF needs before Rust code runs. Always call it first.
    esp_idf_hal::sys::link_patches();

    let peripherals = Peripherals::take()?;
    // On the S3, `strip_with_dma` lets the RMT read the frame through DMA: better for
    // long strips while Wi-Fi is busy.
    let mut strip = ledkit::esp_idf::strip::<LedType, LED_COUNT>(peripherals.pins.gpio5)?;
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);

    let mut frames = Frames::new(ledkit::esp_idf::Clock::start(), Delay::new_default())
        .per_second(FRAMES_PER_SECOND);
    let rainbow = Rainbow::new();
    println!("Rainbow on {LED_COUNT} LEDs, ESP-IDF");

    loop {
        let now = frames.next_frame();
        rainbow.render(&mut strip, now);
        strip.show()?;
    }
}
