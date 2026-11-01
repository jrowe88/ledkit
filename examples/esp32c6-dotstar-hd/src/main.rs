//! DotStar "HD" brightness, side by side with plain dimming, on the ESP32-C6.
//!
//! A DotStar (APA102) has two ways to dim: its 8-bit colors, and a 5-bit
//! "global" level on each LED. Plain dimming only lowers the colors, so dim
//! colors have few steps, and on some DotStars the shortest color pulses
//! barely light at all. HD brightness (ledkit's default for DotStars) lowers
//! the 5-bit level too, so the colors keep their steps.
//!
//! This program shows both at once on an 8x8 DotStar panel, so you can see
//! the difference:
//!
//! - bottom row: a still rainbow with plain dimming
//! - third row from the bottom: the same rainbow, same brightness, with HD
//! - top row: a dim white counter that tells you which step you're on
//!
//! It repeats forever:
//!
//! 1. Start: the top row blinks three times.
//! 2. Seven steps, 5 seconds each: 5%, 10%, 15%, 20%, 30%, 40% and 60%
//!    brightness. The counter shows the step number (1 LED for step 1, and
//!    so on).
//! 3. A sweep from 1% to 60% and back, about 7 seconds. The counter follows
//!    along, lighting one LED for each step's brightness the sweep has
//!    passed, so you can match what you see to the steps.
//!
//! How it works: ledkit's `Strip` sends one 5-bit level for a whole frame,
//! but the LEDs read it per LED. So this program runs two strips, one with
//! HD off and one with HD on, each into a small "capture" output, then sends
//! both rows in one frame, each LED with its own level.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use core::convert::Infallible;

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
use ledkit_core::{prelude::*, wire, Output};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// Which kind of DotStar it is. The Adafruit DotStar HD 8x8 is an APA102
/// (APA102-2020). An SK9822 works too.
type LedType = Apa102;

/// The SPI clock. The DotStar HD 8x8 ran clean from 1 to 40 MHz on the C6.
const SPI_HZ: u32 = 4_000_000;

/// The brightness steps, in percent. From about 55% up HD leaves the 5-bit
/// level at full, so the last step shows two identical rows. Two rows at 60%
/// stay well inside a USB port's 500 mA.
const STEPS: [u8; 7] = [5, 10, 15, 20, 30, 40, 60];

/// How long each step is held.
const STEP_MS: u32 = 5000;

// Data goes on GPIO4 and clock on GPIO6; change them in `main` below.

// ---------------------------------------------------------------------------
// The panel
// ---------------------------------------------------------------------------

/// The panel is 8 by 8. LED 0 is at the bottom left, and each row runs left
/// to right, starting from the bottom: LED `row * 8 + x`.
const WIDTH: usize = 8;
const LED_COUNT: usize = 64;

/// The rows used, counted from the bottom.
const PLAIN_ROW: usize = 0;
const HD_ROW: usize = 2;
const COUNTER_ROW: usize = 7;

/// The counter's color: a dim white, sent at full 5-bit level.
const COUNTER: Rgb = Rgb::new(24, 24, 24);

/// The whole frame: a 4-byte start, 4 bytes per LED, and the end frame.
const FRAME_LEN: usize = wire::clocked_frame_len::<LedType>(LED_COUNT);

// ---------------------------------------------------------------------------
// A capture output: keeps what a strip sends, including its 5-bit level
// ---------------------------------------------------------------------------

/// Records a row's processed colors and the 5-bit level the strip chose.
struct Capture {
    colors: [Rgb; WIDTH],
    global: u8,
}

impl Output<LedType> for Capture {
    type Error = Infallible;

    fn set_global_brightness(&mut self, level: u8) -> bool {
        self.global = level;
        true
    }

    fn write<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: Iterator<Item = Rgb> + ExactSizeIterator,
    {
        for (slot, color) in self.colors.iter_mut().zip(pixels) {
            *slot = color;
        }
        Ok(())
    }
}

/// A rainbow row, dimmed by ledkit with HD on or off.
fn row_strip(hd: bool) -> Strip<LedType, WIDTH, Capture> {
    let mut strip = Strip::new(Capture {
        colors: [Rgb::BLACK; WIDTH],
        global: 31,
    });
    strip.set_hd_brightness(hd);
    // A still picture: dithering would only add flicker here.
    strip.set_dithering(false);
    for x in 0..WIDTH {
        let spread = (x * 65_536 / WIDTH) as u16;
        strip.set(x, Rgb::from_hue(Hue::RED.shifted(spread)));
    }
    strip
}

/// Writes one LED's 4 bytes (its 5-bit level, then the colors in the chip's
/// order) into the frame. ledkit's frame builder does the byte layout.
fn put(frame: &mut [u8; FRAME_LEN], index: usize, color: Rgb, global: u8) {
    let start = 4 + index * 4;
    let one = wire::clocked_frame::<LedType, _>([color]).with_global_brightness(global);
    for (dst, byte) in frame[start..start + 4].iter_mut().zip(one.skip(4)) {
        *dst = byte;
    }
}

/// Builds the frame: both rainbow rows (each with its own 5-bit level) and
/// `count` counter LEDs on the top row.
fn build(
    plain: &Strip<LedType, WIDTH, Capture>,
    hd: &Strip<LedType, WIDTH, Capture>,
    count: usize,
) -> [u8; FRAME_LEN] {
    // Start from an all-black frame, which also has the start and end bytes.
    let mut frame = [0u8; FRAME_LEN];
    for (dst, byte) in frame
        .iter_mut()
        .zip(wire::clocked_frame::<LedType, _>([Rgb::BLACK; LED_COUNT]))
    {
        *dst = byte;
    }
    for x in 0..WIDTH {
        let p = plain.output();
        put(&mut frame, PLAIN_ROW * WIDTH + x, p.colors[x], p.global);
        let h = hd.output();
        put(&mut frame, HD_ROW * WIDTH + x, h.colors[x], h.global);
    }
    for x in 0..count.min(WIDTH) {
        put(&mut frame, COUNTER_ROW * WIDTH + x, COUNTER, 31);
    }
    frame
}

#[main]
fn main() -> ! {
    // Start the chip at full speed.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // SPI2 in mode 0, data (MOSI) on GPIO4 and clock (SCK) on GPIO6. A
    // DotStar reads its data on the clock's rising edge.
    let mut spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_hz(SPI_HZ))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO4)
    .with_sck(peripherals.GPIO6);

    let mut plain = row_strip(false);
    let mut hd = row_strip(true);
    let delay = Delay::new();

    // Sets both rows to `percent`, runs them through ledkit and sends the frame.
    let mut show = |plain: &mut Strip<LedType, WIDTH, Capture>,
                    hd: &mut Strip<LedType, WIDTH, Capture>,
                    percent: u8,
                    count: usize| {
        for strip in [&mut *plain, &mut *hd] {
            strip.set_brightness(Brightness::from_percent(percent));
            strip.show().unwrap();
        }
        spi.write(&build(plain, hd, count)).unwrap();
    };

    println!("ledkit DotStar HD side by side: bottom row plain, third row HD");
    loop {
        println!("start");
        for _ in 0..3 {
            show(&mut plain, &mut hd, 0, WIDTH);
            delay.delay_millis(250);
            show(&mut plain, &mut hd, 0, 0);
            delay.delay_millis(250);
        }

        for (i, &percent) in STEPS.iter().enumerate() {
            show(&mut plain, &mut hd, percent, i + 1);
            // LED 0 is pure red, so its red byte shows how each row dims.
            println!(
                "step {}: {}%  plain: level 31, red {}  HD: level {}, red {}",
                i + 1,
                percent,
                plain.output().colors[0].r,
                hd.output().global,
                hd.output().colors[0].r,
            );
            delay.delay_millis(STEP_MS);
        }

        println!("sweep 1% -> 60% -> 1%");
        let top = STEPS[STEPS.len() - 1];
        for percent in (1..=top).chain((1..=top).rev()) {
            let passed = STEPS.iter().filter(|&&s| s <= percent).count();
            show(&mut plain, &mut hd, percent, passed);
            delay.delay_millis(60);
        }
    }
}
