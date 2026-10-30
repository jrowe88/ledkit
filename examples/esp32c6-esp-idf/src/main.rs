//! First light: drive the ESP32-C6 DevKit's onboard RGB LED with ledkit, on ESP-IDF.
//!
//! The board has one WS2812 "NeoPixel" soldered on. This program shows red,
//! green, blue and white for one second each, then loops a slow rainbow.
//!
//! How it works, in one sentence: a NeoPixel wants a very precisely timed
//! pulse train on one wire, and `ledkit-esp-idf` has the chip's RMT hardware
//! generate that train, so the timing is right even while Wi-Fi is busy.

use std::thread::sleep;
use std::time::Duration;

use esp_idf_hal::peripherals::Peripherals;
use ledkit_core::prelude::*;
use ledkit_esp_idf::RmtOutput;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected. The DevKit's onboard LED is a strip of 1.
const LED_COUNT: usize = 1;

/// Which kind of LED it is. This decides the timing and the color order
/// (green, red, blue for the WS2812B). If the colors come out wrong, this is
/// the first thing to check.
type LedType = Ws2812b;

/// How bright the LED is allowed to get. The onboard LED is very bright up
/// close, so we run it at 20 percent.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

// The LED's data wire is connected to GPIO8 on the ESP32-C6-DevKitC-1. To use
// a different pin (for example, an external strip), change `peripherals.pins.gpio8`
// in `main` below to another pin, such as `peripherals.pins.gpio7`.

/// How much the rainbow turns each step. A hue is a number from 0 to 65535
/// that covers the whole color circle. 262 steps every 20 ms means one full
/// rainbow takes about 5 seconds (65536 / 262 * 20 ms).
const RAINBOW_STEP: u16 = 262;

fn main() {
    // Makes sure some ESP-IDF patches are linked in. Always the first line.
    esp_idf_hal::sys::link_patches();

    // Get access to the chip's pins and peripherals. This can only be done once.
    let peripherals = Peripherals::take().unwrap();

    // Claim an RMT channel on GPIO8 for our LED type. ledkit picks the RMT
    // settings from the chip's timing, so there is nothing else to configure.
    let output = RmtOutput::<LedType>::new(peripherals.pins.gpio8).unwrap();

    // The strip holds the color of every LED. Changes only reach the LED when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);

    println!("ledkit first light: {LED_COUNT} LED");

    // Part one: each basic color for one second, with its name printed so you
    // can compare what you see against what you should see.
    let test_colors = [
        ("red", Rgb::RED),
        ("green", Rgb::GREEN),
        ("blue", Rgb::BLUE),
        ("white", Rgb::WHITE),
    ];
    for (name, color) in test_colors {
        println!("showing {name}");
        strip.fill(color);
        strip.show().unwrap();
        sleep(Duration::from_secs(1));
    }

    // Part two: two frames back to back, with no delay between them. The LED
    // needs a short pause in the data to latch each frame, and ledkit-esp-idf
    // waits it out for us. If it did not, the second frame would be glued
    // onto the first and the LED would stay red instead of ending on blue.
    println!("two frames back to back: red, then blue");
    strip.fill(Rgb::RED);
    strip.show().unwrap();
    strip.fill(Rgb::BLUE);
    strip.show().unwrap();
    sleep(Duration::from_secs(1));

    // Part three: a slow rainbow, forever.
    println!("rainbow");
    let mut hue = Hue::RED;
    loop {
        strip.fill(Rgb::from_hue(hue));
        strip.show().unwrap();
        hue = hue.shifted(RAINBOW_STEP);
        sleep(Duration::from_millis(20));
    }
}
