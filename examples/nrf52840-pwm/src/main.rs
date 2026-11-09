//! A hardware check for the Seeed XIAO nRF52840: drive one NeoPixel strip with the
//! nRF52840's PWM and EasyDMA (`ledkit-nrf`), blocking and async.
//!
//! The PWM plays the whole frame from RAM by itself, so the processor is free while
//! it goes out. Each "round" is one way of using it:
//!
//! | Round | How | Onboard LED |
//! |---|---|---|
//! | 1 | blocking `show()` | blue |
//! | 2 | async `show_async()`, while another task keeps the processor busy | green |
//!
//! Every round shows the same pattern:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. two frames back to back, red then blue: the strip must end blue;
//! 3. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 4. a fast moving rainbow, 50 frames a second, for 10 seconds.
//!
//! Then frames of every length from 1 to `SWEEP_MAX` LEDs (a dim bar that grows along
//! the strip; onboard LED cyan), and a rainbow forever (onboard LED off). If anything
//! fails, the onboard LED turns red and the program stops. Press RESET to run the whole
//! check again.

// No operating system here, and no ordinary `main`: Embassy provides the start-up.
#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_executor::Spawner;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output as Pin, OutputDrive};
use embassy_time::{block_for, Duration, Timer};
use ledkit_core::prelude::*;
use ledkit_core::Output;
use ledkit_nrf::{buffer_len, PwmOutput};
use static_cell::StaticCell;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected.
const LED_COUNT: usize = 100;

/// The longest frame in the length sweep at the end. Longer than the strip is fine:
/// the extra LEDs' data runs off the end.
const SWEEP_MAX: usize = 150;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// How bright the strip may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The data wire is D10 (`P1_15`), through a 74AHCT125 level shifter. To use another
// pin, change `P1_15` in `main` below.

/// The longest frame the program sends: the whole strip, or the sweep if that's longer.
const FRAME_MAX: usize = if LED_COUNT > SWEEP_MAX {
    LED_COUNT
} else {
    SWEEP_MAX
};

/// One frame buffer, big enough for the longest frame, worked out when the program is
/// compiled. It lives in static memory: 150 WS2812B LEDs take 7.6 KB.
const BUFFER_LEN: usize = buffer_len::<LedType>(FRAME_MAX);
static BUFFER: StaticCell<[u16; BUFFER_LEN]> = StaticCell::new();

/// Whether the busy task should keep the processor busy (round 2 only).
static BUSY: AtomicBool = AtomicBool::new(false);

/// The XIAO's own RGB LED, which shows how far the check has got. Its three colors
/// are on P0.26 (red), P0.30 (green) and P0.06 (blue), and light when the pin is low.
struct StatusLed {
    red: Pin<'static>,
    green: Pin<'static>,
    blue: Pin<'static>,
}

impl StatusLed {
    fn show(&mut self, red: bool, green: bool, blue: bool) {
        // Low lights the LED.
        self.red.set_level(Level::from(!red));
        self.green.set_level(Level::from(!green));
        self.blue.set_level(Level::from(!blue));
    }
}

/// Shows one round of the test pattern. `$show` sends the strip.
macro_rules! run_round {
    ($strip:ident, $show:expr, $hue:ident) => {{
        $strip.set_brightness(BRIGHTNESS);
        $strip.set_power_limit(POWER_LIMIT);
        // Dithering smooths dim fades at 100+ frames a second; this check holds
        // still colors, where it would only add flicker.
        $strip.set_dithering(false);
        for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
            $strip.fill(color);
            $show;
            Timer::after_millis(1000).await;
        }
        // With no pause between them, the reset gap must still separate the frames,
        // or the blue frame joins onto the red one and the strip stays red.
        $strip.fill(Rgb::RED);
        $show;
        $strip.fill(Rgb::BLUE);
        $show;
        Timer::after_millis(1000).await;
        // Counting: LED 0 red, every 10th LED white, the last LED blue.
        $strip.fill(Rgb::BLACK);
        for i in (10..LED_COUNT).step_by(10) {
            $strip.set(i, Rgb::WHITE);
        }
        $strip.set(0, Rgb::RED);
        $strip.set(LED_COUNT - 1, Rgb::BLUE);
        $show;
        Timer::after_millis(5000).await;
        // A fast rainbow, 50 frames a second for 10 seconds.
        for _ in 0..500 {
            for i in 0..LED_COUNT {
                // A full color circle every 60 LEDs.
                let spread = (i * 65_536 / 60) as u16;
                $strip.set(i, Rgb::from_hue($hue.shifted(spread)));
            }
            $show;
            $hue = $hue.shifted(768);
            Timer::after_millis(20).await;
        }
    }};
}

/// Keeps the processor busy while `BUSY` is set: 5 ms of work, then a 1 ms break,
/// over and over. The PWM sends from RAM on its own, so the LEDs must not mind.
#[embassy_executor::task]
async fn busy() {
    loop {
        if BUSY.load(Ordering::Relaxed) {
            block_for(Duration::from_millis(5));
        }
        Timer::after_millis(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Run from the 32 MHz crystal, so the PWM's 16 MHz clock is exact.
    let mut config = Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let mut p = embassy_nrf::init(config);

    let mut status = StatusLed {
        red: Pin::new(p.P0_26, Level::High, OutputDrive::Standard),
        green: Pin::new(p.P0_30, Level::High, OutputDrive::Standard),
        blue: Pin::new(p.P0_06, Level::High, OutputDrive::Standard),
    };
    let buffer: &'static mut [u16] = BUFFER.init([0; BUFFER_LEN]);
    let mut hue = Hue::RED;

    spawner.spawn(busy().unwrap());

    // ROUND 1: blocking. `show()` returns once the frame and its reset gap are out.
    status.show(false, false, true);
    {
        let output =
            PwmOutput::<LedType, _>::new(p.PWM0.reborrow(), p.P1_15.reborrow(), &mut *buffer)
                .unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), hue);
    }

    // ROUND 2: async, with the busy task hogging the processor.
    status.show(false, true, false);
    BUSY.store(true, Ordering::Relaxed);
    {
        let output =
            PwmOutput::<LedType, _>::new(p.PWM0.reborrow(), p.P1_15.reborrow(), &mut *buffer)
                .unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show_async().await.unwrap(), hue);
    }
    BUSY.store(false, Ordering::Relaxed);

    // SWEEP: frames of every length from 1 to SWEEP_MAX LEDs.
    status.show(false, true, true);
    let mut output = PwmOutput::<LedType, _>::new(p.PWM0, p.P1_15, buffer).unwrap();
    // A shorter frame only changes the first LEDs and the rest keep their colors,
    // so clear the whole strip (and the sweep range) first. Then the bar grows from dark.
    output
        .write(core::iter::repeat_n(Rgb::BLACK, FRAME_MAX))
        .unwrap();
    // Straight to the output, so no brightness is applied: send a dim blue.
    let dim_blue = Rgb::new(0, 0, 24);
    for length in 1..=SWEEP_MAX {
        output
            .write(core::iter::repeat_n(dim_blue, length))
            .unwrap();
        Timer::after_millis(30).await;
    }
    Timer::after_millis(1000).await;

    // DONE: a rainbow forever. Press RESET to run the check again.
    status.show(false, false, false);
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    loop {
        for i in 0..LED_COUNT {
            let spread = (i * 65_536 / 60) as u16;
            strip.set(i, Rgb::from_hue(hue.shifted(spread)));
        }
        strip.show_async().await.unwrap();
        hue = hue.shifted(768);
        Timer::after_millis(20).await;
    }
}

/// If anything goes wrong (an `unwrap()` above fails), light the onboard LED red and
/// stop.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use embassy_nrf::pac;
    // Red (P0.26) on, green (P0.30) and blue (P0.06) off; the LED lights when low.
    let (red, green, blue) = (1 << 26, 1 << 30, 1 << 6);
    pac::P0.outset().write(|w| w.0 = green | blue);
    pac::P0.outclr().write(|w| w.0 = red);
    pac::P0.dirset().write(|w| w.0 = red | green | blue);
    loop {
        cortex_m::asm::wfi();
    }
}
