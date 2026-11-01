//! Drive a NeoPixel strip from the ESP32-C6's serial port (UART) with ledkit.
//!
//! The strip's data wire is fed from a serial port's transmit pin, with the
//! output inverted. `ledkit-uart` works out serial bytes whose bits look like
//! the pulses a NeoPixel expects, so the port's hardware does the timing.
//!
//! The program repeats two rounds forever, first with the blocking serial
//! port, then with the async one:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 3. a fast moving rainbow, 50 frames a second, for 10 seconds.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    gpio::interconnect::OutputSignal,
    main,
    uart::{Config as UartConfig, UartTx},
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_uart::{buffer_len, ClocklessUart};

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected.
const LED_COUNT: usize = 100;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// The serial port's speed. 4 Mbaud suits a WS2812B (see `ledkit-uart`'s
/// README for other LED types). The C6 makes it exactly: 80 MHz / 20.
const BAUD: u32 = 4_000_000;

/// How bright the strip may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The data wire is GPIO4. To use another pin, change `GPIO4` in `main` below.

/// How many bytes ledkit needs for the encoded frame, worked out when the
/// program is compiled. A baud that can't drive `LedType` stops the build.
const BUFFER_LEN: usize = buffer_len::<LedType>(LED_COUNT, BAUD);

/// An async delay that simply waits, for `ClocklessUart`'s pause after each
/// frame. (A program with an async runtime would use its timer instead.)
struct BusyDelay(Delay);

impl embedded_hal_async::delay::DelayNs for BusyDelay {
    async fn delay_ns(&mut self, ns: u32) {
        embedded_hal::delay::DelayNs::delay_ns(&mut self.0, ns);
    }
}

/// Shows one round of the test pattern. `$show` sends the strip.
macro_rules! run_round {
    ($strip:ident, $show:expr, $delay:ident, $hue:ident) => {{
        $strip.set_brightness(BRIGHTNESS);
        $strip.set_power_limit(POWER_LIMIT);
        // Dithering smooths dim fades at 100+ frames a second; this demo holds
        // still colors, where it would only add flicker.
        $strip.set_dithering(false);
        for (name, color) in [
            ("red", Rgb::RED),
            ("green", Rgb::GREEN),
            ("blue", Rgb::BLUE),
            ("white", Rgb::WHITE),
        ] {
            println!("  {}", name);
            $strip.fill(color);
            $show;
            $delay.delay_millis(1000);
        }
        println!("  counting: LED 0 red, every 10th white, last blue");
        $strip.fill(Rgb::BLACK);
        for i in (10..LED_COUNT).step_by(10) {
            $strip.set(i, Rgb::WHITE);
        }
        $strip.set(0, Rgb::RED);
        $strip.set(LED_COUNT - 1, Rgb::BLUE);
        $show;
        $delay.delay_millis(5000);
        println!("  rainbow");
        for _ in 0..500 {
            for i in 0..LED_COUNT {
                // A full color circle every 60 LEDs.
                let spread = (i * 65_536 / 60) as u16;
                $strip.set(i, Rgb::from_hue($hue.shifted(spread)));
            }
            $show;
            $hue = $hue.shifted(768);
            $delay.delay_millis(20);
        }
    }};
}

#[main]
fn main() -> ! {
    // Start the chip at full speed.
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let mut peripherals = esp_hal::init(config);
    let delay = Delay::new();
    let mut hue = Hue::RED;

    // The serial port: 8 data bits, no parity, 1 stop bit (the defaults), at
    // BAUD. Only its transmit side is used.
    let uart_config = UartConfig::default().with_baudrate(BAUD);

    println!(
        "ledkit on a serial port: {} LEDs at {} baud",
        LED_COUNT, BAUD
    );
    loop {
        println!("BLOCKING");
        {
            // The pin, inverted: the line rests low, as NeoPixels need. The
            // inversion happens in the C6's GPIO matrix, on the way to the pin.
            let pin = OutputSignal::from(peripherals.GPIO4.reborrow()).with_output_inverter(true);
            let uart = UartTx::new(peripherals.UART1.reborrow(), uart_config)
                .unwrap()
                .with_tx(pin);
            let output =
                ClocklessUart::<LedType, _, _, _>::new(uart, BAUD, delay, [0u8; BUFFER_LEN])
                    .unwrap();
            let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
            run_round!(strip, strip.show().unwrap(), delay, hue);
        }
        println!("ASYNC");
        {
            let pin = OutputSignal::from(peripherals.GPIO4.reborrow()).with_output_inverter(true);
            let uart = UartTx::new(peripherals.UART1.reborrow(), uart_config)
                .unwrap()
                .with_tx(pin)
                .into_async();
            let output = ClocklessUart::<LedType, _, _, _>::new(
                uart,
                BAUD,
                BusyDelay(delay),
                [0u8; BUFFER_LEN],
            )
            .unwrap();
            let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
            run_round!(
                strip,
                embassy_futures::block_on(strip.show_async()).unwrap(),
                delay,
                hue
            );
        }
    }
}
