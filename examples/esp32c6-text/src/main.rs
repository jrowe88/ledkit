//! Scrolling text on an 8x8 NeoPixel matrix with ledkit, on the ESP32-C6.
//!
//! A `Ticker` scrolls a list of messages through the panel, one after another:
//! "ledkit", then how long the board has been running ("up 12s"), then
//! "Hello!", and round again, in rainbow colors.
//!
//! Before that, once at boot, a short speed test draws big text into memory
//! (not onto the LEDs) and prints how long each picture took. See `benchmark`.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use core::fmt::Write as _;

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    rmt::{PulseCode, Rmt},
    time::{Instant, Rate},
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_esp::{buffer_len, RmtOutput};
use ledkit_layout::{Canvas, Corner, MatrixLayout};
use ledkit_text::prelude::*;

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How the matrix is wired. This is an Adafruit NeoPixel 8x8 held with its
/// data-in pin at the bottom left (the same as the `esp32c6-anim` example).
/// For another panel, find its wiring with the `esp32c6-matrix` example's
/// wiring finder.
const LAYOUT: MatrixLayout = MatrixLayout::new(8, 8)
    .columns_first()
    .starting_at(Corner::BottomLeft);

/// Which kind of LED it is. This decides the timing and the color order.
type LedType = Ws2812b;

/// The speed of the RMT's clock, in hertz. 80 MHz is the usual choice.
const RMT_HZ: u32 = 80_000_000;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, as ledkit estimates it. Fine on USB for
/// an 8x8.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How fast the text scrolls, in LEDs per second.
const SCROLL_SPEED: u16 = 12;

/// Set to `false` to skip the speed test and go straight to the ticker.
const RUN_BENCHMARK: bool = true;

// The data wire goes to GPIO4 (through a 74AHCT125 level shifter). To use
// another pin, change `peripherals.GPIO4` in `main` below.

/// How many LEDs ledkit sends to.
const LED_COUNT: usize = LAYOUT.width() as usize * LAYOUT.height() as usize;

// ---------------------------------------------------------------------------
// A little text buffer
// ---------------------------------------------------------------------------

/// A small piece of memory we can write text into, like `sprintf` in Arduino.
/// This chip has no heap (so no `String`), so we keep the letters in an array
/// on the stack. Writing more than 16 bytes into it fails instead of
/// overflowing.
struct TextBuffer {
    bytes: [u8; 16],
    len: usize,
}

impl TextBuffer {
    const fn new() -> Self {
        TextBuffer {
            bytes: [0; 16],
            len: 0,
        }
    }

    /// What has been written so far.
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

// Implementing `core::fmt::Write` is what lets us use `write!` on it.
impl core::fmt::Write for TextBuffer {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let new = text.as_bytes();
        let end = self.len + new.len();
        if end > self.bytes.len() {
            return Err(core::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(new);
        self.len = end;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The speed test (easy to delete: remove `time_frames` and `benchmark`, the
// call to `benchmark` in `main`, and the `RUN_BENCHMARK` setting)
// ---------------------------------------------------------------------------

/// Draws 10 pictures with `draw` and prints the average time for one.
///
/// What the numbers mean: a picture that takes 33 milliseconds or less can be
/// drawn 30 times a second or more, which looks smooth. Much more than that
/// and a moving picture starts to look jerky. (This is only the drawing;
/// sending the pixels to the LEDs takes extra time, about 30 microseconds per
/// LED.)
fn time_frames(
    label: &str,
    pixels: &mut [Rgb],
    layout: MatrixLayout,
    mut draw: impl FnMut(&mut Canvas<'_, Rgb, MatrixLayout>, Time),
) {
    const FRAMES: u32 = 10;
    let mut canvas = Canvas::new(pixels, layout).unwrap();
    let start = Instant::now();
    for frame in 0..FRAMES {
        // Each picture is a little later than the last, as in a real animation.
        draw(&mut canvas, Time::from_millis(frame * 33));
    }
    let micros = start.elapsed().as_micros() as u32 / FRAMES;
    println!(
        "{}: {}.{} ms per frame",
        label,
        micros / 1000,
        (micros % 1000) / 100
    );
}

/// Draws big text into a 192x64 picture held in memory, as a big LED wall
/// would show, and prints how long each kind of text takes. Nothing is sent
/// to the LEDs.
fn benchmark() {
    const WIDTH: u16 = 192;
    const HEIGHT: u16 = 64;
    // 192 x 64 LEDs of 3 bytes each is about 37 KB, and it lives on the
    // stack. The ESP32-C6 has 512 KB of RAM and this program has no heap, so
    // the stack has room for it; if the board resets at start-up, set
    // `RUN_BENCHMARK` to false. It is only a picture in RAM, not real LEDs.
    let mut pixels = [Rgb::BLACK; WIDTH as usize * HEIGHT as usize];
    let layout = MatrixLayout::new(WIDTH, HEIGHT);
    println!(
        "ledkit-text speed test, {}x{} LEDs in memory:",
        WIDTH, HEIGHT
    );

    // A 40-LED-tall clock in a 14-segment font, with Plasma letters and a
    // black outline.
    let mut clock14 = TextArea::new(Font::fourteen_segment(40))
        .at(0, 0, WIDTH, HEIGHT)
        .letters(Plasma::new())
        .outline(Rgb::BLACK);
    clock14.set_text("12:34");
    time_frames("14-segment, 40 rows", &mut pixels, layout, |canvas, now| {
        clock14.render(canvas, now)
    });

    // The same, in a 7-segment font.
    let mut clock7 = TextArea::new(Font::seven_segment(40))
        .at(0, 0, WIDTH, HEIGHT)
        .letters(Plasma::new())
        .outline(Rgb::BLACK);
    clock7.set_text("12:34");
    time_frames("7-segment, 40 rows", &mut pixels, layout, |canvas, now| {
        clock7.render(canvas, now)
    });

    // An ordinary bitmap font, drawn twice as big.
    let mut words = TextArea::new(Font::X11_10X20.size(Size::X2)).at(0, 0, WIDTH, HEIGHT);
    words.set_text("Hello, LEDs!");
    time_frames("10x20 at 2x", &mut pixels, layout, |canvas, now| {
        words.render(canvas, now)
    });
}

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    if RUN_BENCHMARK {
        benchmark();
    }

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

    // The ticker: a 5x7 font, letters colored by a moving rainbow, scrolling
    // at `SCROLL_SPEED` LEDs a second. It covers the whole panel unless you
    // place it with `.at(left, top, width, height)`.
    let mut ticker = Ticker::new(Font::X11_5X7)
        .letters(Rainbow::new())
        .speed(SCROLL_SPEED);

    println!("ledkit-text: ticker on {} LEDs", LED_COUNT);

    let delay = Delay::new();
    let start = Instant::now();
    // The second we last wrote the "up" message for. It starts at a number
    // that can never match, so the first time round the loop sets the messages.
    let mut shown_seconds = u32::MAX;
    loop {
        let millis = start.elapsed().as_millis() as u32;
        let seconds = millis / 1000;

        // Once a second, build the "up 12s" message and give the ticker the
        // whole list again. The message showing right now is not interrupted:
        // the new list takes effect at the next gap, when a message has
        // finished and the next one would start. So the number you see can be
        // a few seconds behind.
        if seconds != shown_seconds {
            shown_seconds = seconds;
            let mut uptime = TextBuffer::new();
            // `write!` can only fail if the buffer is full, and 16 bytes is
            // plenty for "up " and a number, so ignoring the result is safe.
            let _ = write!(uptime, "up {}s", seconds);
            ticker.set_messages(&["ledkit", uptime.as_str(), "Hello!"]);
        }

        // Draw the ticker at this moment. A `Canvas` turns each (x, y) into
        // the right LED using `LAYOUT`.
        let now = Time::from_millis(millis);
        ticker.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), now);
        strip.show().unwrap();
        delay.delay_millis(16);
    }
}
