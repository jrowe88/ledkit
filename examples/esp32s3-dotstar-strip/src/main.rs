//! A relaxing light show for a dense DotStar strip, on the Adafruit ESP32-S3 Feather.
//!
//! Three slow scenes, 45 seconds each, fading into one another, forever:
//!
//! 1. **Aurora**: curtains of green, teal and violet drifting along the strip,
//!    with stars twinkling in the dark gaps.
//! 2. **Fireflies**: warm glows wandering and fading in and out over a dusky sky.
//! 3. **Silk**: a band of rainbow colors folding back and forth, with a sheen of
//!    light sliding along it.
//!
//! The scenes themselves are in `scenes.rs`. This file sets up the board.
//!
//! DotStars (APA102) take a clock wire as well as data, so they can be fed fast
//! and they dim smoothly. This show uses both: it draws 100 frames a second, and
//! ledkit's "HD" brightness (on by default for DotStars) keeps dim colors smooth.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

mod scenes;

use core::time::Duration;

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
use ledkit::prelude::*;
use ledkit::spi::ClockedSpi;

use scenes::{Aurora, Fireflies, Silk};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs the strip has. The scenes stretch to fit; with a lower number
/// they cover only the first LEDs, and the rest stay dark.
const LED_COUNT: usize = 150;

/// Which kind of DotStar it is. Adafruit's DotStar strips are APA102s (some
/// newer ones are SK9822s, which also work as `Sk9822`).
type LedType = Apa102;

/// The SPI clock: how fast the bits go out. 4 MHz sends a 150-LED frame in about
/// 1.2 milliseconds.
const DOTSTAR_HZ: u32 = 4_000_000;

/// How bright the LEDs may get. A dense strip is bright up close.
const BRIGHTNESS: Brightness = Brightness::from_percent(25);

/// The most current the LEDs may draw, as ledkit estimates it. 500 mA is safe
/// even with the strip powered from USB; with its own 5 V supply, raise it.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How many frames a second to draw. DotStars take them easily, and the
/// slow movements look smoother for it.
const FRAMES_PER_SECOND: u16 = 100;

/// How long each scene plays, and how long the fade into it takes.
const SCENE_TIME: Duration = Duration::from_secs(45);
const FADE_TIME: Duration = Duration::from_secs(4);

// Data goes on GPIO5 and clock on GPIO6 ("5" and "6" on the Feather), through a
// 74AHCT125 level shifter. To use other pins, change them in `main` below.

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // SPI2 in mode 0: data (MOSI) on GPIO5, clock (SCK) on GPIO6. A DotStar
    // reads each bit on the clock's rising edge.
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_hz(DOTSTAR_HZ))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_mosi(peripherals.GPIO5)
    .with_sck(peripherals.GPIO6);

    // The strip holds the color of every LED; `show` sends them.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(ClockedSpi::new(spi));
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // HD brightness already keeps dim colors smooth on a DotStar, so dithering
    // adds nothing here.
    strip.set_dithering(false);

    // The show: each scene plays in turn and fades into the next. To watch just
    // one, delete the others from this list.
    let aurora = Aurora;
    let fireflies = Fireflies;
    let silk = Silk;
    let fade = Transition::fade(FADE_TIME);
    let scenes = [
        Entry::new(&aurora, SCENE_TIME).transition(fade),
        Entry::new(&fireflies, SCENE_TIME).transition(fade),
        Entry::new(&silk, SCENE_TIME).transition(fade),
    ];
    let show = Playlist::new(&scenes);

    // `Frames` keeps the loop at FRAMES_PER_SECOND and gives the time to draw.
    let mut frames =
        Frames::new(ledkit::esp::Clock::start(), Delay::new()).per_second(FRAMES_PER_SECOND);
    println!("DotStar show on {LED_COUNT} LEDs: aurora, fireflies, silk");

    loop {
        let now = frames.next_frame();
        show.render(&mut strip, now);
        strip.show().unwrap();
    }
}
