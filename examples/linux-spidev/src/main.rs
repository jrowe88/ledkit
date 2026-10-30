//! Drive an addressable LED strip from a Raspberry Pi (or any Linux board)
//! through its SPI device file, using ledkit.
//!
//! This program fills the strip with red, green and blue for one second each,
//! then plays a moving rainbow until you press Ctrl-C.
//!
//! How it works, in one sentence: a NeoPixel wants a very precisely timed
//! pulse train on one wire, and `ledkit-spi` makes that train out of ordinary
//! SPI bytes, so the Pi's SPI hardware does the timing for us.

use std::{thread::sleep, time::Duration};

use ledkit_core::prelude::*;
use ledkit_spi::{buffer_len, ClocklessSpi};
use linux_embedded_hal::{
    spidev::{SpiModeFlags, SpidevOptions},
    SpidevBus,
};

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected. Change this to your strip's length.
const LED_COUNT: usize = 30;

/// Which kind of LED it is. This decides the timing and the color order
/// (green, red, blue for the WS2812B). If the colors come out wrong, this is
/// the first thing to check.
type LedType = Ws2812b;

/// How fast the SPI bus runs, in bits per second. ledkit turns each LED bit
/// into a few SPI bits, so this speed fixes the pulse lengths. 3.2 MHz suits
/// the WS2812B.
///
/// The Pi's SPI divider rounds, so the real clock is a little below 3.2 MHz
/// (about 3.1 MHz), which is within tolerance.
const SPI_HZ: u32 = 3_200_000;

/// The SPI device file. `/dev/spidev0.0` is SPI bus 0, chip-select 0, which
/// uses the MOSI pin on GPIO10 (physical pin 19) of the Pi's header.
const SPI_DEVICE: &str = "/dev/spidev0.0";

/// How much the rainbow turns each frame. A hue is a number from 0 to 65535
/// that covers the whole color circle, so 512 steps every 20 ms means one
/// full rainbow takes about 2.5 seconds (65536 / 512 * 20 ms).
const RAINBOW_STEP: u16 = 512;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open the SPI device file. If this fails, SPI is probably not switched on
    // (see the README) or you lack permission to use the device.
    let mut spi = SpidevBus::open(SPI_DEVICE)?;

    // Configure it: 8 bits per word, SPI mode 0 (the usual idle-low setup),
    // and our clock speed. Linux picks the closest speed the hardware offers.
    let options = SpidevOptions::new()
        .bits_per_word(8)
        .max_speed_hz(SPI_HZ)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options)?;

    // Wrap the bus in a ledkit output. It is told the bus speed so it can work
    // out the right bytes, and it is given a scratch buffer to build them in.
    // `buffer_len` works out how big that buffer must be. On Linux we can
    // simply use a `Vec` for the buffer.
    let output = ClocklessSpi::<LedType, _, _>::new(
        spi,
        SPI_HZ,
        vec![0u8; buffer_len::<LedType>(LED_COUNT, SPI_HZ)],
    )?;

    // The strip holds the color of every LED. Changes only reach the LEDs when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    // Keep it dim: a full-bright strip is very bright, and draws a lot of
    // current from your power supply.
    strip.set_brightness(Brightness::from_percent(20));

    println!("ledkit on {SPI_DEVICE}: {LED_COUNT} LEDs. Press Ctrl-C to stop.");

    // Part one: each basic color for one second.
    for (name, color) in [
        ("red", Rgb::RED),
        ("green", Rgb::GREEN),
        ("blue", Rgb::BLUE),
    ] {
        println!("showing {name}");
        strip.fill(color);
        strip.show()?;
        sleep(Duration::from_secs(1));
    }

    // Part two: a rainbow that moves along the strip, until Ctrl-C.
    println!("rainbow");
    let mut offset = Hue::RED;
    loop {
        for i in 0..LED_COUNT {
            // Spread the full color circle evenly over the strip, then slide
            // the whole pattern a little each frame.
            let spread = (i * 65_536 / LED_COUNT) as u16;
            strip.set(i, Rgb::from_hue(offset.shifted(spread)));
        }
        strip.show()?;
        offset = offset.shifted(RAINBOW_STEP);
        sleep(Duration::from_millis(20));
    }
}
