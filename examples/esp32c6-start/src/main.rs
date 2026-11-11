//! A first ledkit program on an ESP32-C6 with esp-hal: a rainbow on a NeoPixel strip
//! or matrix, about 60 frames a second.
//!
//! It uses only the `ledkit` crate and esp-hal. `ledkit::esp::strip!` sets up the RMT
//! peripheral, and `Frames` paces the loop. Build with `--features async` for the
//! same program written with `async`/`.await`.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::peripherals::Peripherals;
use esp_println::println;
use ledkit::prelude::*;

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// Which kind of LED it is (printed on the strip or in the product listing).
type LedType = Ws2812b;

/// How many LEDs there are. 64 is an 8x8 matrix; a strip works the same way.
const LED_COUNT: usize = 64;

/// How bright the LEDs may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, as ledkit estimates it. Fine on USB for 64
/// LEDs at this brightness.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// How many frames a second to draw.
const FRAMES_PER_SECOND: u16 = 60;

// The data wire goes to GPIO4 (through a 74AHCT125 level shifter). To use another
// pin, change `peripherals.GPIO4` below.

#[main]
fn main() -> ! {
    // Start the chip at full speed and get access to its hardware.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    #[cfg(not(feature = "async"))]
    let result = run(peripherals);
    #[cfg(feature = "async")]
    let result = embassy_futures::block_on(run_async(peripherals));

    // `run` only comes back if something went wrong.
    let error = result.unwrap_err();
    panic!("the LEDs stopped: {error}");
}

/// The program, blocking: each `show()` returns once the frame has gone out.
#[cfg(not(feature = "async"))]
fn run(peripherals: Peripherals) -> Result<(), ledkit::esp::Error> {
    use esp_hal::delay::Delay;

    let mut strip = ledkit::esp::strip!(LedType, LED_COUNT, peripherals.RMT, peripherals.GPIO4)?;
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);

    let mut frames =
        Frames::new(ledkit::esp::Clock::start(), Delay::new()).per_second(FRAMES_PER_SECOND);
    let rainbow = Rainbow::new();
    println!("Rainbow on {LED_COUNT} LEDs, blocking");

    loop {
        let now = frames.next_frame();
        rainbow.render(&mut strip, now);
        strip.show()?;
    }
}

/// The same program, async: the waits are `.await`s, so in a program with more
/// tasks the others run meanwhile.
#[cfg(feature = "async")]
async fn run_async(peripherals: Peripherals) -> Result<(), ledkit::esp::Error> {
    use esp_hal::timer::timg::TimerGroup;
    use esp_hal::timer::OneShotTimer;

    let mut strip = ledkit::esp::strip!(
        LedType,
        LED_COUNT,
        peripherals.RMT,
        peripherals.GPIO4,
        async
    )?;
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);

    // An async delay from one of the chip's hardware timers.
    let timers = TimerGroup::new(peripherals.TIMG0);
    let delay = OneShotTimer::new(timers.timer0).into_async();
    let mut frames =
        FramesAsync::new(ledkit::esp::Clock::start(), delay).per_second(FRAMES_PER_SECOND);
    let rainbow = Rainbow::new();
    println!("Rainbow on {LED_COUNT} LEDs, async");

    loop {
        let now = frames.next_frame().await;
        rainbow.render(&mut strip, now);
        strip.show_async().await?;
    }
}
