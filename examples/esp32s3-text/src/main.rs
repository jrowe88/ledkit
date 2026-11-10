//! A tour of `ledkit-text` on an 8x8 DotStar panel, on the Adafruit ESP32-S3
//! Feather.
//!
//! The program shows a series of scenes, one font or paint setting each, and
//! then starts again. Before each scene its number shows for a second, so you
//! can look it up in the table below. Each scene scrolls its text once or twice.
//!
//! | # | Font | What to look at |
//! |---|---|---|
//! | 1 | 4x6 | the smallest X11 font, exactly as drawn |
//! | 2 | 4x6 at 1.25x, medium edges | smoothed (anti-aliased) to fill all 8 rows |
//! | 3 | 4x6 at 1.25x, soft edges | blurrier smoothing |
//! | 4 | 4x6 at 1.25x, very crisp edges | smoothing that stays close to on/off |
//! | 5 | 5x7 | exactly as drawn |
//! | 6 | 5x8 | exactly as drawn, all 8 rows |
//! | 7 | Tiny 3x5 | capitals and digits, 1 LED between letters |
//! | 8 | Tiny 3x5 at 1.5x | smoothed up to 8 rows |
//! | 9 | 7-segment, 8 rows | a clock, smoothed |
//! | 10 | 7-segment, 8 rows, hard edges | each LED fully on or off |
//! | 11 | 7-segment slanted, 8 rows | leaning right |
//! | 12 | 14-segment, 8 rows | every capital letter |
//! | 13 | 5x7, blue outline | a one-LED outline round white letters |
//! | 14 | 4x6, dark blue behind | a plain background color |
//! | 15 | 5x7, black letters on Plasma | the background is an effect |
//! | 16 | 5x7, reversed | letters and background swapped (white on red becomes red on white) |
//! | 17 | 5x8, Rainbow letters | the letters are an effect |
//! | 18 | 4x6, Plasma letters, white outline | effect letters with an outline |

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
    time::{Instant, Rate},
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_layout::{Canvas, Corner, MatrixLayout};
use ledkit_spi::ClockedSpi;
use ledkit_text::prelude::*;

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How the panel is wired. The Adafruit DotStar HD 8x8 has LED 0 at the bottom
/// left, and every row runs left to right. If the text comes out mirrored or
/// upside down, change the corner.
const LAYOUT: MatrixLayout = MatrixLayout::new(8, 8).starting_at(Corner::BottomLeft);

/// Which kind of DotStar it is. The Adafruit DotStar HD 8x8 is an APA102.
type LedType = Apa102;

/// The SPI clock.
const DOTSTAR_HZ: u32 = 4_000_000;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(25);

/// The most current the LEDs may draw, as ledkit estimates it. Fine on USB for
/// an 8x8.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How fast the text scrolls, in LEDs per second.
const SCROLL_SPEED: u16 = 10;

/// How long each scene runs, in milliseconds.
const SCENE_MS: u32 = 9000;

/// How many scenes there are (see the table at the top).
const SCENES: u32 = 18;

// The DotStar data goes on GPIO5 and its clock on GPIO6 ("5" and "6" on the
// Feather), as in the `esp32s3-dotstar-hd` example. To use other pins, change
// them in `main` below.

/// How many LEDs ledkit sends to.
const LED_COUNT: usize = LAYOUT.width() as usize * LAYOUT.height() as usize;

/// A dim blue, for backgrounds and outlines.
const DARK_BLUE: Rgb = Rgb::new(0, 0, 60);

/// Builds scene `number`'s ticker (numbers as in the table at the top).
fn scene(number: u32) -> Ticker {
    let words = "Hello 42";
    let (ticker, message) = match number {
        1 => (Ticker::new(Font::X11_4X6), words),
        2 => (Ticker::new(Font::X11_4X6.size(Size::X1_25)), words),
        3 => (
            Ticker::new(Font::X11_4X6.size(Size::X1_25).edges(Edges::Soft)),
            words,
        ),
        4 => (
            Ticker::new(Font::X11_4X6.size(Size::X1_25).edges(Edges::VeryCrisp)),
            words,
        ),
        5 => (Ticker::new(Font::X11_5X7), words),
        6 => (Ticker::new(Font::X11_5X8), words),
        7 => (Ticker::new(Font::TINY_3X5.spacing(1)), "HELLO 42"),
        8 => (
            Ticker::new(Font::TINY_3X5.size(Size::X1_5).spacing(1)),
            "HELLO 42",
        ),
        9 => (Ticker::new(Font::seven_segment(8)), "12:34"),
        10 => (Ticker::new(Font::seven_segment(8).hard_edges()), "12:34"),
        11 => (Ticker::new(Font::seven_segment_slanted(8)), "12:34"),
        12 => (Ticker::new(Font::fourteen_segment(8)), "HELLO"),
        13 => (Ticker::new(Font::X11_5X7).outline(DARK_BLUE), words),
        14 => (
            Ticker::new(Font::X11_4X6)
                .letters(Rgb::YELLOW)
                .behind(DARK_BLUE),
            words,
        ),
        15 => (
            Ticker::new(Font::X11_5X7)
                .letters(Rgb::BLACK)
                .behind(Plasma::new()),
            words,
        ),
        16 => (
            Ticker::new(Font::X11_5X7)
                .letters(Rgb::WHITE)
                .behind(Rgb::RED)
                .reverse(true),
            words,
        ),
        17 => (Ticker::new(Font::X11_5X8).letters(Rainbow::new()), words),
        _ => (
            Ticker::new(Font::X11_4X6)
                .letters(Plasma::new())
                .outline(Rgb::new(90, 90, 90)),
            words,
        ),
    };
    let mut ticker = ticker.speed(SCROLL_SPEED);
    ticker.set_messages(&[message]);
    ticker
}

#[main]
fn main() -> ! {
    // Start the chip at full speed.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // SPI2 in mode 0, data (MOSI) on GPIO5 and clock (SCK) on GPIO6. A
    // DotStar reads its data on the clock's rising edge.
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_hz(DOTSTAR_HZ))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO5)
    .with_sck(peripherals.GPIO6);

    let delay = Delay::new();
    let boot = Instant::now();
    let millis = || boot.elapsed().as_millis() as u32;

    let mut strip = Strip::<LedType, LED_COUNT, _>::new(ClockedSpi::new(spi));
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // This loop shows about 60 frames a second; dithering is smoother at 100
    // or more, and its tiny steps can flicker when slower, so it's off here.
    strip.set_dithering(false);

    // The scene number, in dim white, in the middle of the panel.
    let mut label = TextArea::new(Font::TINY_3X5.spacing(1)).letters(Rgb::new(120, 120, 120));
    let mut number_text = [0u8; 2];

    loop {
        for number in 1..=SCENES {
            // The scene number, for one second.
            println!("scene {}", number);
            let digits = if number < 10 {
                number_text[0] = b'0' + number as u8;
                &number_text[..1]
            } else {
                number_text[0] = b'0' + (number / 10) as u8;
                number_text[1] = b'0' + (number % 10) as u8;
                &number_text[..2]
            };
            label.set_text(core::str::from_utf8(digits).unwrap_or("?"));
            strip.fill(Rgb::BLACK);
            label.render(
                &mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(),
                Time::from_millis(0),
            );
            strip.show().unwrap();
            delay.delay_millis(1000);

            // The scene itself. The ticker's clock starts at 0 for each scene.
            let mut ticker = scene(number);
            let start = millis();
            while millis().wrapping_sub(start) < SCENE_MS {
                let now = Time::from_millis(millis().wrapping_sub(start));
                ticker.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), now);
                strip.show().unwrap();
                delay.delay_millis(16);
            }
        }
    }
}
