//! First light: drive the Adafruit ESP32-S3 Feather's onboard NeoPixel with ledkit,
//! through the RMT.
//!
//! The board has one WS2812 "NeoPixel" soldered on. This program shows red,
//! green, blue and white for one second each, then two frames back to back
//! (red, then blue), then loops a slow rainbow.
//!
//! How it works, in one sentence: a NeoPixel wants a very precisely timed
//! pulse train on one wire, and `ledkit-esp` has the chip's RMT peripheral
//! (built for exactly this) generate that train, so the timing is right
//! even when the processor is busy.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_esp::{buffer_len, RmtOutput};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected. The Feather's onboard NeoPixel is a strip of 1.
const LED_COUNT: usize = 1;

/// Which kind of LED it is. This decides the timing and the color order
/// (green, red, blue for the WS2812B). If the colors come out wrong, this is
/// the first thing to check.
type LedType = Ws2812b;

/// The speed of the RMT's clock, in hertz. 80 MHz is the usual choice, and
/// ledkit needs a multiple of 10 MHz (it counts time in steps of 100 ns).
const RMT_HZ: u32 = 80_000_000;

/// How bright the LED is allowed to get. The onboard LED is very bright up
/// close, so we run it at 20 percent.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

// The onboard NeoPixel's data wire is GPIO33 on the ESP32-S3 Feather, and GPIO21
// switches its power on. To drive an external strip instead, change
// `peripherals.GPIO33` in `main` below to another pin, such as `peripherals.GPIO5`
// (marked "5" on the board).

/// How much the rainbow turns each step. A hue is a number from 0 to 65535
/// that covers the whole color circle. 262 steps every 20 ms means one full
/// rainbow takes about 5 seconds (65536 / 262 * 20 ms).
const RAINBOW_STEP: u16 = 262;

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The Feather can switch its NeoPixel's power off to save battery. Turn it
    // on (GPIO21 high) and keep it on: `_neopixel_power` must live as long as
    // the program, because dropping it would let the pin go.
    let _neopixel_power = Output::new(peripherals.GPIO21, Level::High, OutputConfig::default());

    // Set up the RMT, the peripheral that makes timed pulse trains. ledkit
    // never does this for you, because every board is different. We tell it
    // to run at `RMT_HZ`.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(RMT_HZ)).unwrap();
    // The clock esp-hal really set up. Read it before `rmt.channel0` is moved below.
    let rmt_hz = rmt.frequency().as_hz();

    // Wrap one RMT channel in a ledkit output. It is told which pin the LED's
    // data wire is on, the RMT's clock speed (read back from `rmt`, so it is the
    // speed esp-hal really set up), and a scratch buffer to build the pulses in.
    // `buffer_len` works out how big that buffer must be.
    let output = RmtOutput::<LedType, _, _>::new(
        rmt.channel0,
        peripherals.GPIO33,
        rmt_hz,
        [PulseCode::default(); buffer_len::<LedType>(LED_COUNT)],
    )
    .unwrap();

    // The strip holds the color of every LED. Changes only reach the LED when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    // Dithering smooths dim colors when you call show() about 100 times a
    // second or more. This demo updates more slowly at low brightness, where
    // dithering's tiny steps show up as flicker, so it's off here.
    strip.set_dithering(false);

    let delay = Delay::new();

    println!("ledkit first light: {} LED", LED_COUNT);

    // Part one: each basic color for one second, with its name printed so you
    // can compare what you see against what you should see.
    let test_colors = [
        ("red", Rgb::RED),
        ("green", Rgb::GREEN),
        ("blue", Rgb::BLUE),
        ("white", Rgb::WHITE),
    ];
    for (name, color) in test_colors {
        println!("showing {}", name);
        strip.fill(color);
        strip.show().unwrap();
        delay.delay_millis(1000);
    }

    // Part two: two frames back to back, with no delay between them. The LED
    // needs a short pause in the data to latch each frame, and ledkit-esp
    // waits it out for us. If it did not, the second frame would be glued
    // onto the first and the LED would stay red instead of ending on blue.
    println!("two frames back to back: red, then blue");
    strip.fill(Rgb::RED);
    strip.show().unwrap();
    strip.fill(Rgb::BLUE);
    strip.show().unwrap();
    delay.delay_millis(1000);

    // Part three: a slow rainbow, forever.
    println!("rainbow");
    let mut hue = Hue::RED;
    loop {
        strip.fill(Rgb::from_hue(hue));
        strip.show().unwrap();
        hue = hue.shifted(RAINBOW_STEP);
        delay.delay_millis(20);
    }
}
