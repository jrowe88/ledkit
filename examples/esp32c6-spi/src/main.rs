//! First light: drive the ESP32-C6 DevKit's onboard RGB LED with ledkit.
//!
//! The board has one WS2812 "NeoPixel" soldered on. This program shows red,
//! green, blue and white for one second each, then loops a slow rainbow.
//!
//! How it works, in one sentence: a NeoPixel wants a very precisely timed
//! pulse train on one wire, and `ledkit-spi` makes that train out of ordinary
//! SPI bytes, so the chip's SPI hardware does the timing for us.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    spi::{
        master::{Config as SpiConfig, Spi},
        Mode,
    },
    time::Rate,
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_spi::{buffer_len, ClocklessSpi};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected. The DevKit's onboard LED is a strip of 1.
const LED_COUNT: usize = 1;

/// Which kind of LED it is. This decides the timing and the color order
/// (green, red, blue for the WS2812B). If the colors come out wrong, this is
/// the first thing to check.
type LedType = Ws2812b;

/// How fast the SPI bus runs, in bits per second. 3.2 MHz is exact on the
/// ESP32 because its 80 MHz clock divides evenly by 25, so every LED bit
/// gets the right pulse length.
const SPI_HZ: u32 = 3_200_000;

/// How bright the LED is allowed to get. The onboard LED is very bright up
/// close, so we run it at 20 percent.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

// The LED's data wire is connected to GPIO8 on the ESP32-C6-DevKitC-1. To use
// a different pin (for example, an external strip), change `peripherals.GPIO8`
// in `main` below to another pin, such as `peripherals.GPIO7`.

/// How much the rainbow turns each step. A hue is a number from 0 to 65535
/// that covers the whole color circle. 262 steps every 20 ms means one full
/// rainbow takes about 5 seconds (65536 / 262 * 20 ms).
const RAINBOW_STEP: u16 = 262;

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Set up the SPI bus. ledkit never does this for you, because every board
    // is different. Mode 0 is the usual idle-low setup. We only use the MOSI
    // pin (the data out wire). There is no clock pin and no chip-select pin,
    // because a NeoPixel has only one data wire.
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_hz(SPI_HZ))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO8);

    // Wrap the bus in a ledkit output. It is told the bus speed so it can work
    // out the right bytes, and it is given a scratch buffer to build them in.
    // `buffer_len` works out how big that buffer must be.
    let output = ClocklessSpi::<LedType, _, _>::new(
        spi,
        SPI_HZ,
        [0u8; buffer_len::<LedType>(LED_COUNT, SPI_HZ)],
    )
    .unwrap();

    // The strip holds the color of every LED. Changes only reach the LED when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);

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

    // Part two: a slow rainbow, forever.
    println!("rainbow");
    let mut hue = Hue::RED;
    loop {
        strip.fill(Rgb::from_hue(hue));
        strip.show().unwrap();
        hue = hue.shifted(RAINBOW_STEP);
        delay.delay_millis(20);
    }
}
