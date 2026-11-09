//! A hardware check for the Seeed XIAO nRF52840: drive one NeoPixel strip from the
//! SPI peripheral (SPIM, `ledkit-spi`), and show that the serial port can't.
//!
//! The SPIM always sends through EasyDMA, so a whole frame goes out without the
//! processor's help, but it only runs at 125 kHz to 8 MHz in powers of two (32 MHz on
//! SPIM3). WS2812-type LEDs need 4 MHz there. This uses SPIM2: SPIM3 has a known chip
//! fault (nRF52840 erratum 198) that can corrupt its data when the processor uses the
//! same RAM during a transfer, and 4 MHz doesn't need SPIM3.
//!
//! | Round | How | Onboard LED |
//! |---|---|---|
//! | 1 | `ClocklessSpi::new_packed` at 4 MHz (5 SPI bits per LED bit), blocking | blue |
//! | 2 | the same, async | green |
//! | 3 | `ClocklessSpi::new` at 4 MHz (8 SPI bits per LED bit), blocking | yellow |
//! | 4 | `ClocklessUart` at 1 Mbaud: **must be refused** | magenta = refused, as expected |
//!
//! Rounds 1 to 3 show the same pattern:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. two frames back to back, red then blue: the strip must end blue;
//! 3. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 4. a fast moving rainbow, 50 frames a second, for 10 seconds.
//!
//! Round 4 sends nothing: the nRF's serial port (UARTE) tops out at 1 Mbaud and can't
//! invert its output, so `ledkit-uart` must refuse it with a clear error. The onboard LED
//! stays magenta for 3 seconds, then a rainbow runs forever over SPI (onboard LED off).
//! If anything fails, the onboard LED turns red and the program stops.

// No operating system here, and no ordinary `main`: Embassy provides the start-up.
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::config::{Config, HfclkSource};
use embassy_nrf::gpio::{Level, Output as Pin, OutputDrive};
use embassy_nrf::{bind_interrupts, peripherals, spim, uarte, Peri};
use embassy_time::{Delay, Timer};
use ledkit_core::prelude::*;
use ledkit_core::TimingError;
use ledkit_spi::{buffer_len, buffer_len_packed, ClocklessSpi};
use ledkit_uart::ClocklessUart;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected.
const LED_COUNT: usize = 100;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// How bright the strip may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The data wire is D10 (`P1_15`, the SPI's MOSI pin), through a 74AHCT125 level
// shifter: the same wire as the `nrf52840-pwm` check. D8 (`P1_13`) carries the SPI's
// clock, which NeoPixels don't use: leave it unconnected (see `led_spi`).

/// The SPI speed: 4 MHz, the one SPIM speed that suits WS2812-type LEDs.
const SPI_HZ: u32 = 4_000_000;

/// The serial port's fastest speed on the nRF52840.
const UART_MAX_BAUD: u32 = 1_000_000;

// Buffer sizes, worked out when the program is compiled.
const PACKED_LEN: usize = buffer_len_packed::<LedType>(LED_COUNT, SPI_HZ);
const PLAIN_LEN: usize = buffer_len::<LedType>(LED_COUNT, SPI_HZ);

// The SPIM and the serial port finish their transfers with an interrupt.
bind_interrupts!(struct Irqs {
    SPI2 => spim::InterruptHandler<peripherals::SPI2>;
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;
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
}

/// SPIM2 at 4 MHz, sending on D10 (MOSI).
///
/// NeoPixels have no clock wire, but the SPIM still needs a clock pin: given none
/// (`Spim::new_txonly_nosck`), it never starts, and the first `show()` waits forever.
/// So D8 (`P1_13`) is the clock pin here, with nothing connected to it.
fn led_spi<'d>(
    spi: Peri<'d, peripherals::SPI2>,
    sck: Peri<'d, peripherals::P1_13>,
    mosi: Peri<'d, peripherals::P1_15>,
) -> spim::Spim<'d> {
    let mut config = spim::Config::default();
    config.frequency = spim::Frequency::M4;
    spim::Spim::new_txonly(spi, Irqs, sck, mosi, config)
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

    // ROUND 1: new_packed at 4 MHz, blocking.
    status.show(false, false, true);
    {
        let spi = led_spi(p.SPI2.reborrow(), p.P1_13.reborrow(), p.P1_15.reborrow());
        let output =
            ClocklessSpi::<LedType, _, _>::new_packed(spi, SPI_HZ, [0u8; PACKED_LEN]).unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), hue);
    }

    // ROUND 2: the same, async.
    status.show(false, true, false);
    {
        let spi = led_spi(p.SPI2.reborrow(), p.P1_13.reborrow(), p.P1_15.reborrow());
        let output =
            ClocklessSpi::<LedType, _, _>::new_packed(spi, SPI_HZ, [0u8; PACKED_LEN]).unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show_async().await.unwrap(), hue);
    }

    // ROUND 3: new at 4 MHz (a whole SPI byte per LED bit), blocking.
    status.show(true, true, false);
    {
        let spi = led_spi(p.SPI2.reborrow(), p.P1_13.reborrow(), p.P1_15.reborrow());
        let output = ClocklessSpi::<LedType, _, _>::new(spi, SPI_HZ, [0u8; PLAIN_LEN]).unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), hue);
    }

    // ROUND 4: the serial port at its fastest, 1 Mbaud. ledkit-uart must refuse it.
    {
        let mut uart_config = uarte::Config::default();
        uart_config.baudrate = uarte::Baudrate::Baud1m;
        // D6 (P1.11), the XIAO's TX pin. Nothing is sent.
        let uart = uarte::UarteTx::new(p.UARTE0, Irqs, p.P1_11, uart_config);
        match ClocklessUart::<LedType, _, _, _>::new(uart, UART_MAX_BAUD, Delay, [0u8; 1]) {
            // Refused, with the speeds that would work: as expected.
            Err(TimingError::UartBaudUnusable { .. }) => status.show(true, false, true),
            // Accepted (or refused for another reason): that's a bug.
            _ => panic!("ledkit-uart should refuse 1 Mbaud for a WS2812B"),
        }
        Timer::after_millis(3000).await;
    }

    // DONE: a rainbow forever over SPI. Press RESET to run the check again.
    status.show(false, false, false);
    let spi = led_spi(p.SPI2, p.P1_13, p.P1_15);
    let output = ClocklessSpi::<LedType, _, _>::new_packed(spi, SPI_HZ, [0u8; PACKED_LEN]).unwrap();
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
