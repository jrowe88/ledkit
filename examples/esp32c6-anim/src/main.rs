//! Animations on an LED matrix or strip with ledkit, on the ESP32-C6, through the RMT.
//!
//! Set `MATRIX` below to pick what is plugged in:
//!
//! - `true`, an 8x8 matrix: swirl, rings, pinwheel, plasma, flames and rain,
//!   8 seconds each, with fades, an iris, a dissolve and a wipe between them.
//! - `false`, a strip: rainbow, a comet flying over a slow gradient, chase,
//!   sparkle, flames, and then the strip split in three (a comet, sparkle
//!   and a breathing glow), 8 seconds each, fading from one to the next.
//!
//! Both shows loop forever.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use core::time::Duration;

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    rmt::{PulseCode, Rmt},
    time::{Instant, Rate},
};
use esp_println::println;
use ledkit_anim::prelude::*;
use ledkit_core::prelude::*;
use ledkit_esp::{buffer_len, RmtOutput};
use ledkit_layout::{Canvas, Corner, MatrixLayout, Segment};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// `true` for an 8x8 matrix, `false` for a strip.
const MATRIX: bool = true;

/// How the matrix is wired. This is an Adafruit NeoPixel 8x8 held with its
/// data-in pin at the bottom left. For another panel, find its wiring with the
/// `esp32c6-matrix` example's wiring finder.
const LAYOUT: MatrixLayout = MatrixLayout::new(8, 8)
    .columns_first()
    .starting_at(Corner::BottomLeft);

/// How many LEDs the strip has (used when `MATRIX` is `false`).
const STRIP_LEDS: usize = 100;

/// Which kind of LED it is. This decides the timing and the color order.
type LedType = Ws2812b;

/// The speed of the RMT's clock, in hertz. 80 MHz is the usual choice.
const RMT_HZ: u32 = 80_000_000;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, as ledkit estimates it. Fine on USB for
/// an 8x8; a long strip needs its own 5 V supply, with its ground joined to the
/// board's.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How long each animation plays.
const STEP: Duration = Duration::from_secs(8);

/// How long each transition takes.
const CHANGE: Duration = Duration::from_secs(1);

// The data wire goes to GPIO4 (through a 74AHCT125 level shifter). To use
// another pin, change `peripherals.GPIO4` in `main` below.

/// How many LEDs ledkit sends to: the matrix or the strip.
const LED_COUNT: usize = if MATRIX {
    LAYOUT.width() as usize * LAYOUT.height() as usize
} else {
    STRIP_LEDS
};

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Set up the RMT, the peripheral that makes timed pulse trains, and wrap
    // one of its channels in a ledkit output on GPIO4.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(RMT_HZ)).unwrap();
    let rmt_hz = rmt.frequency().as_hz();
    let output = RmtOutput::<LedType, _, _>::new(
        rmt.channel0,
        peripherals.GPIO4,
        rmt_hz,
        [PulseCode::default(); buffer_len::<LedType>(LED_COUNT)],
    )
    .unwrap();

    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // This loop shows about 60 frames a second; dithering is smoother at 100
    // or more, and its tiny steps can flicker when slower, so it's off here.
    strip.set_dithering(false);

    // --- The matrix show ---------------------------------------------------
    let swirl = Swirl::new();
    let rings = Rings::new().palette(&palettes::OCEAN);
    let pinwheel = Pinwheel::new().color(Rgb::ORANGE);
    let plasma = Plasma::new().palette(&palettes::PARTY);
    let flames = Flames::new();
    let rain = Rain::new();
    let matrix_show = [
        Entry::new(&swirl, STEP),
        Entry::new(&rings, STEP).transition(Transition::fade(CHANGE)),
        Entry::new(&pinwheel, STEP).transition(Transition::iris(CHANGE)),
        Entry::new(&plasma, STEP).transition(Transition::dissolve(CHANGE)),
        Entry::new(&flames, STEP).transition(Transition::fade(CHANGE)),
        Entry::new(&rain, STEP).transition(Transition::wipe(Edge::Top, CHANGE)),
    ];

    // --- The strip show ----------------------------------------------------
    let rainbow = Rainbow::new();
    // A white comet over a slow, dim gradient: black on top lets the gradient
    // show through, and the comet's tail fades into it.
    let comet_over_gradient = GradientScroll::new()
        .palette(&palettes::SUNSET)
        .cycle_time(Duration::from_secs(20))
        .dimmed(90)
        .layer(Comet::new().color(Rgb::WHITE).tail_length(12), Blend::Over);
    let chase = Chase::new().color(Rgb::RED).size(2).spacing(3);
    let sparkle = Sparkle::new().background(Rgb::new(0, 0, 16)).density(20);
    // The strip split in three, each part its own little strip.
    let third = STRIP_LEDS / 3;
    let left = Comet::new().bounce(true).tail_length(6);
    let middle = Sparkle::new().colors_between(Rgb::GOLD, Rgb::WHITE);
    let right = Breathe::new().color(Rgb::PURPLE);
    let parts: [(Segment, &dyn Animation); 3] = [
        (Segment::new(0, third), &left),
        (Segment::new(third, third), &middle),
        (Segment::new(2 * third, STRIP_LEDS - 2 * third), &right),
    ];
    let split = Segments::new(&parts);
    let fade = Transition::fade(CHANGE);
    let strip_show = [
        Entry::new(&rainbow, STEP),
        Entry::new(&comet_over_gradient, STEP).transition(fade),
        Entry::new(&chase, STEP).transition(fade),
        Entry::new(&sparkle, STEP).transition(fade),
        Entry::new(&flames, STEP).transition(fade),
        Entry::new(&split, STEP).transition(fade),
    ];

    let show = Playlist::new(if MATRIX { &matrix_show } else { &strip_show });
    println!(
        "ledkit-anim: {} LEDs, {}",
        LED_COUNT,
        if MATRIX { "matrix" } else { "strip" }
    );

    let delay = Delay::new();
    let start = Instant::now();
    loop {
        // Animations are drawn from the time since the start, so they run at
        // the right speed however long a frame takes. esp-hal has its own
        // duration type, so pass the milliseconds.
        let now = Time::from_millis(start.elapsed().as_millis() as u32);
        if MATRIX {
            // A `Canvas` turns each (x, y) into the right LED using `LAYOUT`.
            show.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), now);
        } else {
            show.render(&mut strip, now);
        }
        strip.show().unwrap();
        delay.delay_millis(16);
    }
}
