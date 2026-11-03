//! Text and shapes on an LED matrix with ledkit, on the ESP32-S3, through the RMT.
//!
//! A matrix is just a long strip folded into a grid. `ledkit-layout` knows how
//! your panel is folded, so you can say "draw at column 3, row 5" (or draw text
//! and shapes with `embedded-graphics`) and it works out which LED that is.
//!
//! The program repeats three modes forever:
//!
//! 1. A wiring finder, for 3 seconds: it lights LEDs by their raw position on
//!    the strip, so you can see how your panel is wired.
//! 2. Shapes, for 5 seconds: a border, a diagonal line and a filled circle.
//! 3. Scrolling text, for about 8 seconds: "ledkit", a different color per letter.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_5X8, FONT_6X10},
        MonoFont, MonoTextStyle,
    },
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    rmt::{PulseCode, Rmt},
    time::Rate,
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_esp::{buffer_len, RmtOutput};
use ledkit_layout::{rgb888_from_rgb, Canvas, MatrixLayout, Order};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

// These are the defaults for a generic 8x8 panel. Set the real size and wiring
// for yours (see "Finding your wiring" in the README).

/// How many LEDs wide the matrix is.
const WIDTH: u16 = 8;

/// How many LEDs tall the matrix is.
const HEIGHT: u16 = 8;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// How the LEDs are wired into a grid. The defaults are progressive (every
/// line runs the same way), rows first, starting at the top-left. If your
/// panel differs, the wiring finder shows how: run the program and follow
/// "Finding your wiring" in the README. For a zig-zag panel starting at the
/// bottom-right, for example, write
/// `.serpentine().starting_at(Corner::BottomRight)` (and import `Corner`).
/// An Adafruit NeoPixel 8x8, held with its data-in pin at the bottom left, is
/// `.columns_first().starting_at(Corner::BottomLeft)`.
const LAYOUT: MatrixLayout = MatrixLayout::new(WIDTH, HEIGHT);

/// How many LEDs make one line of the chain: a row, or a column on a
/// columns-first panel. The wiring finder is built from it.
const LINE: usize = match LAYOUT.order() {
    Order::RowsFirst => WIDTH as usize,
    Order::ColumnsFirst => HEIGHT as usize,
};

/// The speed of the RMT's clock, in hertz. 80 MHz is the usual choice, and
/// ledkit needs a multiple of 10 MHz (it counts time in steps of 100 ns).
const RMT_HZ: u32 = 80_000_000;

/// How bright the matrix is allowed to get.
const BRIGHTNESS: Brightness = Brightness::from_percent(10);

/// The most current the LEDs may draw, as ledkit estimates it. This limits the
/// LEDs only, not the board, and every LED draws about 1 mA even when off, so
/// a big matrix can exceed USB's budget while showing nothing. Up to 8x8 is
/// usually fine on USB; for anything bigger, use a 5 V supply rated for the
/// panel, with its ground joined to the board's.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The matrix's data wire goes to GPIO5 ("5" on the Feather, through a 74AHCT125 level shifter). To
// use another pin, change `peripherals.GPIO5` in `main` below.

/// How many LEDs the matrix has.
const LED_COUNT: usize = WIDTH as usize * HEIGHT as usize;

/// The text that scrolls.
const MESSAGE: &str = "ledkit";

/// The font. Short panels get the 5x8 font, which fits an 8-row panel exactly;
/// taller ones get the bigger 6x10.
const FONT: &MonoFont = if HEIGHT < 10 { &FONT_5X8 } else { &FONT_6X10 };

/// How far the text moves right for each letter, in pixels (the letter plus its spacing).
const LETTER_WIDTH: i32 = (FONT.character_size.width + FONT.character_spacing) as i32;

/// How tall a letter is, in pixels.
const LETTER_HEIGHT: i32 = FONT.character_size.height as i32;

/// How long the scrolling text runs before the modes start over, in milliseconds.
const SCROLL_MS: u32 = 8000;

/// How long one frame of the scrolling text lasts. The text moves one pixel per frame.
const FRAME_MS: u32 = 100;

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // Set up the RMT, the peripheral that makes timed pulse trains. ledkit
    // never does this for you, because every board is different.
    let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(RMT_HZ)).unwrap();
    // The clock esp-hal really set up. Read it before `rmt.channel0` is moved below.
    let rmt_hz = rmt.frequency().as_hz();

    // Wrap one RMT channel in a ledkit output: which pin the data wire is on,
    // the RMT's clock speed, and a scratch buffer to build the pulses in.
    // `buffer_len` works out how big that buffer must be.
    let output = RmtOutput::<LedType, _, _>::new(
        rmt.channel0,
        peripherals.GPIO5,
        rmt_hz,
        [PulseCode::default(); buffer_len::<LedType>(LED_COUNT)],
    )
    .unwrap();

    // The strip holds the color of every LED. Changes only reach the LEDs when
    // we call `show`.
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // Dithering smooths dim colors when you call show() about 100 times a
    // second or more. This demo updates more slowly at low brightness, where
    // dithering's tiny steps show up as flicker, so it's off here.
    strip.set_dithering(false);

    let delay = Delay::new();

    println!("ledkit matrix: {} x {} LEDs", WIDTH, HEIGHT);

    loop {
        // Mode 1: the wiring finder. It writes straight into the strip by
        // position, not through the layout, because it is about how the panel
        // is really wired. LED 0 is red, the rest of the first line is green,
        // and the start of the second line is blue. A line is a row, or a
        // column if the layout is columns first.
        println!("wiring finder");
        println!(
            "red = LED 0 (data in); green = the rest of the first line; blue = where the second line starts"
        );
        strip.fill(Rgb::BLACK);
        // `set` takes anything that converts into the chip's pixel type, so this
        // works for RGB and RGBW chips alike, and ignores indices past the end.
        strip.set(0, Rgb::RED);
        for index in 1..LINE {
            strip.set(index, Rgb::GREEN);
        }
        for index in LINE..LINE + 3 {
            strip.set(index, Rgb::BLUE);
        }
        strip.show().unwrap();
        delay.delay_millis(3000);

        // Mode 2: shapes. A `Canvas` lets `embedded-graphics` draw onto the
        // strip: it turns each (x, y) into the right LED using `LAYOUT`.
        println!("shapes");
        strip.fill(Rgb::BLACK);
        draw_shapes(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap());
        strip.show().unwrap();
        delay.delay_millis(5000);

        // Mode 3: scrolling text, one pixel per frame, from just off the right
        // edge until the last letter has left on the left, then again until
        // `SCROLL_MS` has passed. This works for any `WIDTH`, because it starts
        // at `WIDTH` and ends at minus the text's width.
        println!("scrolling text");
        let text_width = MESSAGE.len() as i32 * LETTER_WIDTH;
        let mut frames_left = SCROLL_MS / FRAME_MS;
        while frames_left > 0 {
            let mut x = WIDTH as i32;
            while x > -text_width && frames_left > 0 {
                strip.fill(Rgb::BLACK);
                draw_text(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), x);
                strip.show().unwrap();
                delay.delay_millis(FRAME_MS);
                x -= 1;
                frames_left -= 1;
            }
        }
    }
}

/// Draws a border, a diagonal line and a filled circle, in three colors.
/// Parts that do not fit on a small matrix are simply cut off.
fn draw_shapes<D: DrawTarget<Color = Rgb888>>(target: &mut D) {
    let width = WIDTH as u32;
    let height = HEIGHT as u32;

    // A one-pixel border around the whole matrix.
    let _ = Rectangle::new(Point::zero(), Size::new(width, height))
        .into_styled(PrimitiveStyle::with_stroke(Rgb888::RED, 1))
        .draw(target);

    // A line from corner to corner.
    let _ = Line::new(
        Point::zero(),
        Point::new(WIDTH as i32 - 1, HEIGHT as i32 - 1),
    )
    .into_styled(PrimitiveStyle::with_stroke(Rgb888::GREEN, 1))
    .draw(target);

    // A filled circle in the middle, half as wide as the shorter side.
    let diameter = (width.min(height) / 2).max(1);
    let corner = Point::new(
        (width as i32 - diameter as i32) / 2,
        (height as i32 - diameter as i32) / 2,
    );
    let _ = Circle::new(corner, diameter)
        .into_styled(PrimitiveStyle::with_fill(Rgb888::BLUE))
        .draw(target);
}

/// Draws `MESSAGE` with its left edge at column `x` (which may be off the
/// matrix), one letter at a time so the hue can shift along the text.
fn draw_text<D: DrawTarget<Color = Rgb888>>(target: &mut D, x: i32) {
    // Center the letters vertically. On a matrix shorter than the font is tall
    // this is negative, and the top and bottom of the letters are cut off.
    let y = (HEIGHT as i32 - LETTER_HEIGHT) / 2;
    for (index, letter) in MESSAGE.chars().enumerate() {
        // Each letter is a little further around the color circle.
        let hue = Hue::RED.shifted((index as u16).wrapping_mul(10_000));
        let style = MonoTextStyle::new(FONT, rgb888_from_rgb(Rgb::from_hue(hue)));
        let mut buffer = [0u8; 4];
        let _ = Text::with_baseline(
            letter.encode_utf8(&mut buffer),
            Point::new(x + index as i32 * LETTER_WIDTH, y),
            style,
            Baseline::Top,
        )
        .draw(target);
    }
}
