//! Every ledkit animation, one after another, on a Teensy 4.0 or 4.1.
//!
//! Built normally, it drives an 8x8 matrix; built with `--features strip`, a
//! 100-LED strip. Either way it starts with a quick check (red, green, blue,
//! white, then on the matrix the top row red and the left column green), then
//! plays all 16 built-in effects with their default settings and two combined
//! ones, 6 seconds each, using a different transition into each. Then it
//! starts again.
//!
//! The LED data leaves on pin 11 through the SPI hardware, wired exactly as in
//! the `teensy4-spi` example:
//!
//! ```text
//! Teensy pin 11 (LPSPI4 MOSI) -> 74AHCT125 input (1A)
//! 74AHCT125 output (1Y)       -> 330 ohm resistor -> LED DIN
//! 74AHCT125 power             -> 5 V, and its ground to the LEDs' ground
//! Teensy GND                  -> LED GND (all grounds joined)
//! ```

// No operating system here, and no ordinary `main`: the BSP provides start-up.
#![no_std]
#![no_main]

use core::time::Duration;

use cortex_m::asm::delay as wait_cycles;
use ledkit_anim::prelude::*;
use ledkit_core::prelude::*;
use ledkit_layout::{Canvas, Corner, MatrixLayout};
use ledkit_spi::{buffer_len, ClocklessSpi};
use teensy4_bsp as bsp;
use teensy4_panic as _; // If the program panics, stop safely.

use bsp::board;
use bsp::hal::{gpt, iomuxc};

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// `true` for the 8x8 matrix, `false` for a strip. Set by the `strip` feature.
const MATRIX: bool = !cfg!(feature = "strip");

/// How the matrix is wired. This is an Adafruit NeoPixel 8x8 held with its
/// data-in pin at the bottom left. If the check at the start shows the red row
/// anywhere but the top, or the green column anywhere but the left, change
/// this (the `esp32c6-matrix` example's wiring finder shows how).
const LAYOUT: MatrixLayout = MatrixLayout::new(8, 8)
    .columns_first()
    .starting_at(Corner::BottomLeft);

/// How many LEDs the strip has (used with the `strip` feature).
const STRIP_LEDS: usize = 100;

/// Which kind of LED it is. This decides the timing and the color order.
type LedType = Ws2812b;

/// The SPI speed we ask for. The Teensy can't hit it exactly; see
/// `actual_spi_hz`.
const SPI_HZ: u32 = 3_200_000;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, as ledkit estimates it. 500 mA is safe
/// on USB; raise it if the LEDs have their own 5 V supply.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How long each animation plays.
const STEP: Duration = Duration::from_secs(6);

/// How long each transition takes.
const CHANGE: Duration = Duration::from_secs(1);

// ---------------------------------------------------------------------------

/// How many LEDs ledkit sends to: the matrix or the strip.
const LED_COUNT: usize = if MATRIX {
    LAYOUT.width() as usize * LAYOUT.height() as usize
} else {
    STRIP_LEDS
};

/// The transitions, used in turn: each animation comes in with the next one.
const TRANSITIONS: [Transition; 7] = [
    Transition::fade(CHANGE),
    Transition::wipe(Edge::Left, CHANGE),
    Transition::dissolve(CHANGE),
    Transition::iris(CHANGE),
    Transition::fade_through_black(CHANGE),
    Transition::wipe(Edge::Top, CHANGE),
    Transition::cut(),
];

/// The Teensy 4's SPI divides a 132 MHz clock by an even number, so 3.2 MHz
/// becomes 132 MHz / 42 = 3,142,857 Hz. This repeats imxrt-hal 0.6's rounding
/// so ledkit knows the real speed (see the `teensy4-spi` example).
const fn actual_spi_hz(source_hz: u32, wanted_hz: u32) -> u32 {
    let mut half_divider = 1 + (source_hz - 1) / (wanted_hz * 2);
    if half_divider < 3 {
        half_divider = 3;
    }
    if half_divider > 128 {
        half_divider = 128;
    }
    source_hz / (half_divider * 2)
}

const ACTUAL_SPI_HZ: u32 = actual_spi_hz(board::LPSPI_FREQUENCY, SPI_HZ);
const BUFFER_LEN: usize = buffer_len::<LedType>(LED_COUNT, ACTUAL_SPI_HZ);

/// The Teensy's SPI driver sends at most 512 bytes per write.
const MAX_WRITE_BYTES: usize = 512;

/// Wait for a number of milliseconds by counting processor cycles.
fn delay_ms(milliseconds: u32) {
    wait_cycles(milliseconds * (board::ARM_FREQUENCY / 1_000));
}

#[bsp::rt::entry]
fn main() -> ! {
    let board::Resources {
        pins,
        lpspi4,
        mut gpt1,
        ..
    } = board::t40(board::instances());
    // The Teensy 4.1 and the MicroMod use `board::t41` / `board::tmm` instead.

    // A millisecond clock for the animations. GPT1 counts the 1 MHz peripheral
    // clock divided by 1000, so its count is the milliseconds since it started.
    // It wraps after about 49 days, just like ledkit's `Time`.
    const _: () = assert!(board::PERCLK_FREQUENCY == 1_000_000);
    gpt1.set_clock_source(gpt::ClockSource::PeripheralClock);
    gpt1.set_divider(1000);
    gpt1.set_mode(gpt::Mode::FreeRunning);
    gpt1.set_reset_on_enable(true);

    // The SPI bus on pin 11, set up as in the `teensy4-spi` example: a
    // pull-down on the data pin, the pin tristated between transfers, and the
    // clock started straight away, so the first LED reads its data correctly.
    let mut data_pin = pins.p11;
    iomuxc::configure(
        &mut data_pin,
        iomuxc::Config::modify().set_pull_keeper(Some(iomuxc::PullKeeper::Pulldown100k)),
    );
    let mut spi: board::Lpspi = board::lpspi(
        lpspi4,
        board::LpspiPins {
            sdo: data_pin,
            sdi: pins.p12,
            sck: pins.p13,
        },
        SPI_HZ,
    );
    spi.disabled(|_| {
        // Safety: `spi` owns LPSPI4 and the peripheral is switched off while
        // this runs, so nothing else is using these registers.
        let regs = unsafe { bsp::ral::lpspi::LPSPI4::instance() };
        bsp::ral::modify_reg!(bsp::ral::lpspi, regs, CFGR1, OUTCFG: OUTCFG_1);
    });
    let mut timing = spi.clock_configs();
    timing.pcssck = 0;
    timing.sckpcs = 0;
    timing.dbt = 0;
    spi.disabled(|disabled| disabled.set_clock_configs(timing));

    let output = ClocklessSpi::<LedType, _, _>::new(spi, ACTUAL_SPI_HZ, [0u8; BUFFER_LEN])
        .unwrap()
        .with_max_write_len(MAX_WRITE_BYTES);
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // Frames come about 60 times a second; dithering's tiny steps can flicker
    // that slowly, so it's off.
    strip.set_dithering(false);

    // --- The check ---------------------------------------------------------
    for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
        strip.fill(color);
        strip.show().unwrap();
        delay_ms(1000);
    }
    if MATRIX {
        // The top row red and the left column green (yellow where they
        // meet): if they show anywhere else, `LAYOUT` is wrong.
        strip.fill(Rgb::BLACK);
        let mut canvas = Canvas::new(strip.pixels_mut(), LAYOUT).unwrap();
        for i in 0..8 {
            canvas.set(i, 0, Rgb::RED);
            canvas.set(0, i, Rgb::GREEN);
        }
        canvas.set(0, 0, Rgb::YELLOW);
        strip.show().unwrap();
        delay_ms(3000);
    }

    // --- The show ----------------------------------------------------------
    // Every built-in effect with its default settings, in `Effect::ALL` order.
    let effects = Effect::ALL;
    // Two combined ones: a white comet over a dim rainbow, and a rainbow
    // running backwards at half speed, mirrored from the middle.
    let comet_over_rainbow = Rainbow::new()
        .dimmed(60)
        .layer(Comet::new().color(Rgb::WHITE), Blend::Over);
    let mirrored = Rainbow::new().reversed().at_speed(50).mirrored();
    let combined: [&dyn Animation; 2] = [&comet_over_rainbow, &mirrored];

    let entries: [Entry; 18] = core::array::from_fn(|i| {
        let animation: &dyn Animation = match effects.get(i) {
            Some(effect) => effect,
            None => combined[i - effects.len()],
        };
        Entry::new(animation, STEP).transition(TRANSITIONS[i % TRANSITIONS.len()])
    });
    let show = Playlist::new(&entries);

    gpt1.enable();
    loop {
        let now = Time::from_millis(gpt1.count());
        if MATRIX {
            // A `Canvas` turns each (x, y) into the right LED using `LAYOUT`.
            show.render(&mut Canvas::new(strip.pixels_mut(), LAYOUT).unwrap(), now);
        } else {
            show.render(&mut strip, now);
        }
        strip.show().unwrap();
        delay_ms(16);
    }
}
