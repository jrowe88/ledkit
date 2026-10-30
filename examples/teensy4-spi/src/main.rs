//! Drive a WS2812B ("NeoPixel") LED strip from a Teensy 4.0 or 4.1 with ledkit.
//!
//! This program shows red, green, blue and white for one second each, then
//! plays a moving rainbow forever.
//!
//! How it works, in one sentence: a NeoPixel wants a very precisely timed
//! pulse train on one wire, and `ledkit-spi` makes that train out of ordinary
//! SPI bytes, so the chip's SPI hardware (LPSPI4) does the timing for us.
//!
//! # Wiring
//!
//! ```text
//! Teensy pin 11 (LPSPI4 MOSI) -> 74AHCT125 input (1A)
//! 74AHCT125 output (1Y)       -> 330 ohm resistor -> strip DIN
//! 74AHCT125 power             -> 5 V, and its ground to the strip's ground
//! Teensy GND                  -> strip GND (all grounds joined)
//! ```
//!
//! * The Teensy 4 works at 3.3 V and its pins are NOT 5 V tolerant. The
//!   74AHCT125 chip lifts the data signal to the 5 V a strip expects. Never
//!   connect a 5 V signal to a Teensy pin.
//! * Power the strip from 5 V, never from the Teensy's 3.3 V pin.
//! * Pin 13 is the SPI clock. We don't need it, but it is also wired to the
//!   Teensy's orange LED, which will glow while we send data. That's harmless.

// No operating system here, and no ordinary `main`: the BSP provides start-up.
#![no_std]
#![no_main]

use cortex_m::asm::delay as wait_cycles;
use ledkit_core::prelude::*;
use ledkit_spi::{buffer_len, ClocklessSpi};
use teensy4_bsp as bsp;
use teensy4_panic as _; // If the program panics, stop safely.

use bsp::board;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected. Change this to your strip's length.
const LED_COUNT: usize = 30;

/// Which kind of LED it is. This decides the timing and the color order
/// (green, red, blue for the WS2812B). If the colors come out wrong, this is
/// the first thing to check.
type LedType = Ws2812b;

/// The SPI speed we ask for, in bits per second. 3.2 MHz suits the WS2812B.
/// The Teensy can't hit this exactly, see `ACTUAL_SPI_HZ` below.
const SPI_HZ: u32 = 3_200_000;

/// How bright the LEDs are allowed to get. A full-bright strip is very bright
/// and draws a lot of current, so we run it at 20 percent.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the strip may draw, in milliamps. ledkit turns the
/// brightness down if the colors on screen would need more. 500 mA is safe for
/// USB power; raise it if you add a separate 5 V supply for the strip.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How much the rainbow turns each frame. A hue is a number from 0 to 65535
/// that covers the whole color circle, so 512 steps every 20 ms means one
/// full rainbow takes about 2.5 seconds (65536 / 512 * 20 ms).
const RAINBOW_STEP: u16 = 512;

// ---------------------------------------------------------------------------
// Working out the real SPI speed
// ---------------------------------------------------------------------------

/// The Teensy 4's SPI hardware divides a fixed 132 MHz clock
/// (`board::LPSPI_FREQUENCY`) by an even number to make the bus speed, so it
/// can only make certain speeds. 132 MHz divided by 3.2 MHz is 41.25, which
/// isn't a whole number, so the hardware picks the next slower speed:
/// 132 MHz / 42 = 3,142,857 Hz.
///
/// NOTE: this mirrors imxrt-hal 0.6's divider rounding. Re-check it if the
/// HAL version changes.
///
/// This function repeats the rounding the Teensy HAL does inside
/// `Disabled::set_clock_hz` (round the divider up, and never go below 6), so
/// we can tell ledkit the speed the bus REALLY runs at. The 1.8 percent
/// difference in pulse length is well inside what a WS2812B tolerates, but
/// ledkit should know the true number.
const fn actual_spi_hz(source_hz: u32, wanted_hz: u32) -> u32 {
    // The HAL works with half of the divider, rounded up.
    let mut half_divider = 1 + (source_hz - 1) / (wanted_hz * 2);
    // The HAL never goes below 3 (divide by 6) or above 128 (divide by 256).
    if half_divider < 3 {
        half_divider = 3;
    }
    if half_divider > 128 {
        half_divider = 128;
    }
    source_hz / (half_divider * 2)
}

/// The speed the bus really runs at: 3,142,857 Hz with the settings above.
const ACTUAL_SPI_HZ: u32 = actual_spi_hz(board::LPSPI_FREQUENCY, SPI_HZ);

/// How many bytes ledkit needs to describe the whole strip.
const BUFFER_LEN: usize = buffer_len::<LedType>(LED_COUNT, ACTUAL_SPI_HZ);

/// The Teensy's SPI driver can send at most 4096 bits (512 bytes) in a single
/// write. ledkit cuts longer frames into pieces of this size.
const MAX_WRITE_BYTES: usize = 512;

/// Wait for a number of milliseconds by counting processor cycles.
fn delay_ms(milliseconds: u32) {
    wait_cycles(milliseconds * (board::ARM_FREQUENCY / 1_000));
}

#[bsp::rt::entry]
fn main() -> ! {
    // Get access to the chip's pins and peripherals. The BSP also sets the
    // chip to run at full speed (600 MHz) here.
    let board::Resources { pins, lpspi4, .. } = board::t40(board::instances());
    // The Teensy 4.1 and the MicroMod use `board::t41` / `board::tmm` instead.
    // Pins 11, 12 and 13 are the same on all of them.

    // Set up the SPI bus. ledkit never does this for you, because every board
    // is different. We want the MOSI pin (pin 11, the data out wire). The
    // hardware needs a pin for each of data out, data in and clock, so pins 12
    // and 13 are claimed too, but we leave pin 12 unconnected. There is no
    // chip-select pin, because a NeoPixel has only one data wire. The bus
    // starts in mode 0, the usual idle-low setup, most significant bit first.
    let spi: board::Lpspi = board::lpspi(
        lpspi4,
        board::LpspiPins {
            sdo: pins.p11,
            sdi: pins.p12,
            sck: pins.p13,
        },
        SPI_HZ,
    );

    // Wrap the bus in a ledkit output. It is told the real bus speed so it can
    // work out the right bytes, and it is given a scratch buffer to build them
    // in. `BUFFER_LEN` is how big that buffer must be.
    //
    // `with_max_write_len` is needed because the Teensy's SPI driver refuses to
    // send more than 512 bytes at once. Longer strips are sent in several
    // writes. The tiny pauses between them are harmless for a NeoPixel.
    let output = ClocklessSpi::<LedType, _, _>::new(spi, ACTUAL_SPI_HZ, [0u8; BUFFER_LEN])
        .unwrap()
        .with_max_write_len(MAX_WRITE_BYTES);

    // The strip holds the color of every LED. Changes only reach the LEDs when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);

    // Part one: each basic color for one second.
    let test_colors = [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE];
    for color in test_colors {
        strip.fill(color);
        strip.show().unwrap();
        delay_ms(1000);
    }

    // Part two: a rainbow that moves along the strip, forever.
    let mut offset = Hue::RED;
    loop {
        for i in 0..LED_COUNT {
            // Spread the full color circle evenly over the strip, then slide
            // the whole pattern a little each frame.
            let spread = (i * 65_536 / LED_COUNT) as u16;
            strip.set(i, Rgb::from_hue(offset.shifted(spread)));
        }
        strip.show().unwrap();
        offset = offset.shifted(RAINBOW_STEP);
        delay_ms(20);
    }
}
