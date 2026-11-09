//! A hardware check for the Raspberry Pi Pico 2 W (RP2350): drive one NeoPixel strip
//! every way `ledkit-rp` can, then a DotStar strip over SPI.
//!
//! The strip's data wire stays on GPIO16 while the program hands the PIO state
//! machine from one output to the next. Each "round" is one way of feeding it:
//!
//! | Round | Output | How the data gets to the PIO |
//! |---|---|---|
//! | 1 | `PioOutput`, blocking | the processor pushes each LED into the FIFO |
//! | 2 | `PioDmaOutput`, blocking | a DMA channel streams the frame |
//! | 3 | `PioDmaOutput`, async | the same, awaited |
//! | 4 | `PioOutput`, async | the processor, yielding while the FIFO is full |
//!
//! Every round starts with its number: the first 1 to 4 LEDs blink magenta three
//! times. Then the same pattern:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. two frames back to back, red then blue: the strip must end blue;
//! 3. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 4. a fast moving rainbow, 50 frames a second, for 10 seconds.
//!
//! Then DMA frames of every length from 1 to `SWEEP_MAX` LEDs (a dim blue bar that
//! grows along the strip), then the DotStar round (5 blinks): the same pattern on a
//! DotStar (APA102) strip through SPI0 at 4 MHz, then a clock sweep (the counting
//! pattern at each of `DOTSTAR_CLOCKS`, 2 seconds each). Last, the NeoPixel rainbow
//! runs forever on DMA. Press RESET (or unplug and replug) to run it all again.
//!
//! The PIO block (PIO1), DMA channel (1) and pins are chosen to leave PIO0, DMA
//! channel 0 and the radio's pins free for the Wi-Fi chip's driver.

// No operating system here, and no ordinary `main`: the HAL provides the start-up.
#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use hal::dma::DMAExt;
use hal::fugit::RateExtU32;
use hal::gpio::FunctionSpi;
use hal::pio::PIOExt;
use hal::Clock;
use ledkit_core::prelude::*;
use ledkit_core::Output;
use ledkit_rp::{PioDmaOutput, PioOutput};
use ledkit_spi::ClockedSpi;
use panic_halt as _; // If the program panics, stop.
use rp235x_hal as hal;
use static_cell::ConstStaticCell;

/// Tells the RP2350's boot ROM that this is a program to run (and whether it is
/// for the Arm or the RISC-V cores, from the build target).
#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many NeoPixels are connected.
const LED_COUNT: usize = 100;

/// The longest frame in the length sweep. Longer than the strip is fine: the extra
/// LEDs' data runs off the end.
const SWEEP_MAX: usize = 150;

/// Which kind of NeoPixel it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// Which kind of DotStar is on SPI0 (use `Sk9822` for SK9822 strips).
type DotStarType = Apa102;

/// How many DotStars are connected. The round still runs with none connected.
const DOTSTAR_COUNT: usize = 64;

/// The DotStar clock for its pattern round.
const DOTSTAR_HZ: u32 = 4_000_000;

/// The DotStar clocks of the sweep. The RP2350 divides its 150 MHz peripheral clock,
/// so the real rate is the nearest it can make at or below each.
const DOTSTAR_CLOCKS: [u32; 9] = [
    1_000_000, 2_000_000, 4_000_000, 8_000_000, 12_000_000, 16_000_000, 20_000_000, 24_000_000,
    32_000_000,
];

/// How bright the strips may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The NeoPixel data wire is GPIO16 (pin 21), through a 74AHCT125 level shifter.
// The DotStar data is GPIO19 (pin 25, SPI0 TX) and its clock GPIO18 (pin 24, SPI0
// SCK), also through the 74AHCT125. To use other pins, change them in `main`.

/// The Pico 2 W's crystal.
const XTAL_HZ: u32 = 12_000_000;

/// The DMA buffer: one 32-bit word per LED, as long as the longest frame. It must
/// outlive the program, because the DMA engine reads it in the background.
static BUFFER: ConstStaticCell<[u32; ledkit_rp::buffer_len::<LedType>(SWEEP_MAX)]> =
    ConstStaticCell::new([0; ledkit_rp::buffer_len::<LedType>(SWEEP_MAX)]);

/// Blinks the first `$n` LEDs magenta three times: "round $n starts".
macro_rules! announce {
    ($strip:ident, $show:expr, $delay:ident, $n:expr) => {{
        $strip.set_brightness(BRIGHTNESS);
        $strip.set_power_limit(POWER_LIMIT);
        $strip.set_dithering(false);
        for _ in 0..3 {
            $strip.fill(Rgb::BLACK);
            $strip.fill_range(0..$n, Rgb::MAGENTA);
            $show;
            $delay.delay_ms(300);
            $strip.fill(Rgb::BLACK);
            $show;
            $delay.delay_ms(300);
        }
        $delay.delay_ms(700);
    }};
}

/// Shows one round of the test pattern on a strip of `$count` LEDs. `$show` sends
/// the strip.
macro_rules! run_round {
    ($strip:ident, $show:expr, $delay:ident, $hue:ident, $count:expr) => {{
        // Dithering smooths dim fades at 100+ frames a second; this check holds
        // still colors, where it would only add flicker.
        $strip.set_dithering(false);
        for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
            $strip.fill(color);
            $show;
            $delay.delay_ms(1000);
        }
        // With no pause between them, the reset gap must still separate the frames,
        // or the blue frame joins onto the red one and the strip stays red.
        $strip.fill(Rgb::RED);
        $show;
        $strip.fill(Rgb::BLUE);
        $show;
        $delay.delay_ms(1000);
        // Counting: LED 0 red, every 10th white, the last blue.
        $strip.fill(Rgb::BLACK);
        for i in (10..$count).step_by(10) {
            $strip.set(i, Rgb::WHITE);
        }
        $strip.set(0, Rgb::RED);
        $strip.set($count - 1, Rgb::BLUE);
        $show;
        $delay.delay_ms(5000);
        // A fast rainbow, 50 frames a second for 10 seconds.
        for _ in 0..500 {
            for i in 0..$count {
                // A full color circle every 60 LEDs.
                let spread = (i * 65_536 / 60) as u16;
                $strip.set(i, Rgb::from_hue($hue.shifted(spread)));
            }
            $show;
            $hue = $hue.shifted(768);
            $delay.delay_ms(20);
        }
    }};
}

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    // 150 MHz system clock from the 12 MHz crystal.
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    // The timer keeps frames apart (ledkit) and does the waiting (this program).
    let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
    let mut delay = timer;
    let sys_hz = clocks.system_clock.freq().to_Hz();

    let (mut pio, sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
    let dma = pac.DMA.split(&mut pac.RESETS);
    let buffer: &'static mut [u32] = BUFFER.take();
    let mut hue = Hue::RED;

    // ROUND 1: the processor feeds the FIFO, blocking.
    let output = PioOutput::<LedType, _, _, _, _>::new(
        &mut pio,
        sm0,
        pins.gpio16.into_function(),
        sys_hz,
        timer,
    )
    .unwrap();
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    announce!(strip, strip.show().unwrap(), delay, 1);
    run_round!(strip, strip.show().unwrap(), delay, hue, LED_COUNT);
    let (sm0, pin) = strip.release().release(&mut pio);

    // ROUND 2: DMA feeds the FIFO, blocking.
    let output = PioDmaOutput::<LedType, _, _, _, _, _>::new(
        &mut pio, sm0, pin, dma.ch1, buffer, sys_hz, timer,
    )
    .unwrap();
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    announce!(strip, strip.show().unwrap(), delay, 2);
    run_round!(strip, strip.show().unwrap(), delay, hue, LED_COUNT);
    let (sm0, pin, ch1, buffer) = strip.release().release(&mut pio).unwrap();

    // ROUND 3: DMA, async. `block_on` polls the future until it is done; a program
    // with an async executor would `.await` it among its other tasks.
    let output =
        PioDmaOutput::<LedType, _, _, _, _, _>::new(&mut pio, sm0, pin, ch1, buffer, sys_hz, timer)
            .unwrap();
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    let show = |strip: &mut Strip<LedType, LED_COUNT, _>| {
        embassy_futures::block_on(strip.show_async()).unwrap();
    };
    announce!(strip, show(&mut strip), delay, 3);
    run_round!(strip, show(&mut strip), delay, hue, LED_COUNT);
    let (sm0, pin, ch1, buffer) = strip.release().release(&mut pio).unwrap();

    // ROUND 4: the processor feeds the FIFO, async.
    let output = PioOutput::<LedType, _, _, _, _>::new(&mut pio, sm0, pin, sys_hz, timer).unwrap();
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    let show = |strip: &mut Strip<LedType, LED_COUNT, _>| {
        embassy_futures::block_on(strip.show_async()).unwrap();
    };
    announce!(strip, show(&mut strip), delay, 4);
    run_round!(strip, show(&mut strip), delay, hue, LED_COUNT);
    let (sm0, pin) = strip.release().release(&mut pio);

    // SWEEP: DMA frames of 1 to SWEEP_MAX LEDs, straight to the output (so no
    // brightness is applied: send a dim blue).
    let mut output =
        PioDmaOutput::<LedType, _, _, _, _, _>::new(&mut pio, sm0, pin, ch1, buffer, sys_hz, timer)
            .unwrap();
    // A shorter frame only changes the first LEDs and the rest keep their colors,
    // so clear the whole sweep range first. Then the bar grows from dark.
    output
        .write(core::iter::repeat_n(Rgb::BLACK, SWEEP_MAX))
        .unwrap();
    let dim_blue = Rgb::new(0, 0, 24);
    for length in 1..=SWEEP_MAX {
        output
            .write(core::iter::repeat_n(dim_blue, length))
            .unwrap();
        delay.delay_ms(30);
    }
    delay.delay_ms(1000);
    // Blank the NeoPixels while the DotStars run.
    output
        .write(core::iter::repeat_n(Rgb::BLACK, SWEEP_MAX))
        .unwrap();

    // DOTSTAR: ledkit-spi drives DotStars on any embedded-hal SPI bus, the RP2350's
    // included. The bus is borrowed, so each strip below can reuse it.
    let mut spi = hal::spi::Spi::<_, _, _, 8>::new(
        pac.SPI0,
        (
            pins.gpio19.into_function::<FunctionSpi>(),
            pins.gpio18.into_function::<FunctionSpi>(),
        ),
    )
    .init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        DOTSTAR_HZ.Hz(),
        embedded_hal::spi::MODE_0,
    );
    {
        let mut dotstars = Strip::<DotStarType, DOTSTAR_COUNT, _>::new(ClockedSpi::new(&mut spi));
        announce!(dotstars, dotstars.show().unwrap(), delay, 5);
        run_round!(
            dotstars,
            dotstars.show().unwrap(),
            delay,
            hue,
            DOTSTAR_COUNT
        );
    }
    for hz in DOTSTAR_CLOCKS {
        spi.set_baudrate(clocks.peripheral_clock.freq(), hz.Hz());
        let mut dotstars = Strip::<DotStarType, DOTSTAR_COUNT, _>::new(ClockedSpi::new(&mut spi));
        dotstars.set_brightness(BRIGHTNESS);
        dotstars.fill(Rgb::BLACK);
        for i in (10..DOTSTAR_COUNT).step_by(10) {
            dotstars.set(i, Rgb::WHITE);
        }
        dotstars.set(0, Rgb::RED);
        dotstars.set(DOTSTAR_COUNT - 1, Rgb::BLUE);
        dotstars.show().unwrap();
        delay.delay_ms(2000);
    }

    // DONE: the NeoPixel rainbow, on DMA, forever.
    let mut strip = Strip::<LedType, SWEEP_MAX, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    strip.set_dithering(false);
    loop {
        for i in 0..SWEEP_MAX {
            let spread = (i * 65_536 / 60) as u16;
            strip.set(i, Rgb::from_hue(hue.shifted(spread)));
        }
        strip.show().unwrap();
        hue = hue.shifted(768);
        delay.delay_ms(20);
    }
}
