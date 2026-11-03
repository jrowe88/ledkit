//! A hardware check for the Seeed XIAO nRF52840: drive DotStars (APA102, SK9822) from
//! the SPI peripheral (SPIM3) with `ledkit-spi`'s `ClockedSpi`, at every clock the SPIM
//! has.
//!
//! DotStars have a clock wire as well as a data wire, so their timing comes from the
//! clock and any speed the LEDs keep up with works. SPIM3 is the nRF52840's only SPI
//! that runs above 8 MHz, and it sends through EasyDMA.
//!
//! | Part | What | Onboard LED |
//! |---|---|---|
//! | 1 | a clock sweep: 1, 2, 4, 8, 16 and 32 MHz, blocking. At each: red, green, blue and white (1 s each), then counting (3 s) | blinks white once per step before it (1 blink = 1 MHz ... 6 blinks = 32 MHz) |
//! | 2 | async at 8 MHz: the same colors and counting, then a rainbow at 50 frames a second for 10 s | green |
//! | 3 | brightness steps with ledkit's HD dimming: a still rainbow at 5, 10, 15, 20, 30, 40 and 60 %, 3 s each | blue |
//! | then | a rainbow forever at 8 MHz | off |
//!
//! Counting: LED 0 red, every 10th LED white, the last LED blue. If anything fails, the
//! onboard LED turns red and the program stops. Press RESET to run the check again.

// No operating system here, and no ordinary `main`: Embassy provides the start-up.
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output as Pin, OutputDrive};
use embassy_nrf::{bind_interrupts, peripherals, spim};
use embassy_time::Timer;
use ledkit_core::prelude::*;
use ledkit_spi::ClockedSpi;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// Which kind of DotStar it is. The Adafruit DotStar HD 8x8 is an APA102
/// (APA102-2020). For an SK9822 strip, use `Sk9822`.
type LedType = Apa102;

/// How many LEDs are connected: 64 for the 8x8 panel. Try 144 (SK9822) or 300
/// (APA102) for long strips.
const LED_COUNT: usize = 64;

/// How bright the LEDs may get in parts 1 and 2.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

/// The clocks of the sweep: 1, 2, 4, 8, 16 and 32 MHz.
const CLOCKS: [spim::Frequency; 6] = [
    spim::Frequency::M1,
    spim::Frequency::M2,
    spim::Frequency::M4,
    spim::Frequency::M8,
    spim::Frequency::M16,
    spim::Frequency::M32,
];

/// The brightness steps of part 3, in percent.
const STEPS: [u8; 7] = [5, 10, 15, 20, 30, 40, 60];

// Data goes on D10 (`P1_15`, MOSI) and clock on D8 (`P1_13`, SCK), each through its
// own 74AHCT125 channel. Change them in `spi` below.

// The SPIM finishes its transfers with an interrupt.
bind_interrupts!(struct Irqs {
    SPIM3 => spim::InterruptHandler<peripherals::SPI3>;
});

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

    /// Blinks white `times` times, then stays off.
    async fn blink(&mut self, times: usize) {
        for _ in 0..times {
            self.show(true, true, true);
            Timer::after_millis(200).await;
            self.show(false, false, false);
            Timer::after_millis(300).await;
        }
        Timer::after_millis(500).await;
    }
}

/// SPIM3 on the XIAO's SPI pins at the clock `frequency`.
macro_rules! spi {
    ($p:ident, $frequency:expr) => {{
        let mut config = spim::Config::default();
        config.frequency = $frequency;
        spim::Spim::new_txonly(
            $p.SPI3.reborrow(),
            Irqs,
            $p.P1_13.reborrow(),
            $p.P1_15.reborrow(),
            config,
        )
    }};
}

/// Red, green, blue and white, one second each, then counting for 3 seconds. `$show`
/// sends the strip.
macro_rules! colors_and_counting {
    ($strip:ident, $show:expr) => {{
        $strip.set_brightness(BRIGHTNESS);
        $strip.set_power_limit(POWER_LIMIT);
        $strip.set_dithering(false);
        for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
            $strip.fill(color);
            $show;
            Timer::after_millis(1000).await;
        }
        // Counting: LED 0 red, every 10th LED white, the last LED blue.
        $strip.fill(Rgb::BLACK);
        for i in (10..LED_COUNT).step_by(10) {
            $strip.set(i, Rgb::WHITE);
        }
        $strip.set(0, Rgb::RED);
        $strip.set(LED_COUNT - 1, Rgb::BLUE);
        $show;
        Timer::after_millis(3000).await;
    }};
}

/// Paints a rainbow along the strip, a full color circle every 60 LEDs.
fn rainbow(strip: &mut Strip<LedType, LED_COUNT, impl Sized>, hue: Hue) {
    for i in 0..LED_COUNT {
        let spread = (i * 65_536 / 60) as u16;
        strip.set(i, Rgb::from_hue(hue.shifted(spread)));
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Run from the 32 MHz crystal, so the SPI clock is exact.
    let mut config = Config::default();
    config.hfclk_source = HfclkSource::ExternalXtal;
    let mut p = embassy_nrf::init(config);

    let mut status = StatusLed {
        red: Pin::new(p.P0_26, Level::High, OutputDrive::Standard),
        green: Pin::new(p.P0_30, Level::High, OutputDrive::Standard),
        blue: Pin::new(p.P0_06, Level::High, OutputDrive::Standard),
    };
    let mut hue = Hue::RED;

    // PART 1: the clock sweep, blocking.
    for (step, frequency) in CLOCKS.into_iter().enumerate() {
        status.blink(step + 1).await;
        let output = ClockedSpi::<LedType, _>::new(spi!(p, frequency));
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        colors_and_counting!(strip, strip.show().unwrap());
    }

    // PART 2: async at 8 MHz.
    status.show(false, true, false);
    {
        let output = ClockedSpi::<LedType, _>::new(spi!(p, spim::Frequency::M8));
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        colors_and_counting!(strip, strip.show_async().await.unwrap());
        for _ in 0..500 {
            rainbow(&mut strip, hue);
            strip.show_async().await.unwrap();
            hue = hue.shifted(768);
            Timer::after_millis(20).await;
        }
    }

    // PART 3: brightness steps. HD dimming (on by default for DotStars) lowers each
    // LED's 5-bit level too, so dim steps keep their colors.
    status.show(false, false, true);
    {
        let output = ClockedSpi::<LedType, _>::new(spi!(p, spim::Frequency::M8));
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        strip.set_power_limit(POWER_LIMIT);
        strip.set_dithering(false);
        rainbow(&mut strip, Hue::RED);
        for percent in STEPS {
            strip.set_brightness(Brightness::from_percent(percent));
            strip.show().unwrap();
            Timer::after_millis(3000).await;
        }
    }

    // DONE: a rainbow forever at 8 MHz. Press RESET to run the check again.
    status.show(false, false, false);
    let output = ClockedSpi::<LedType, _>::new(spi!(p, spim::Frequency::M8));
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    loop {
        rainbow(&mut strip, hue);
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
