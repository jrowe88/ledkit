//! Drive a WS2812B ("NeoPixel") strip from a Teensy 4's serial port with ledkit.
//!
//! The strip's data wire is fed from serial port 1's transmit pin (pin 1),
//! with the output inverted. `ledkit-uart` works out serial bytes whose bits
//! look like the pulses a NeoPixel expects, so the port's hardware does the
//! timing.
//!
//! The program repeats two rounds forever, first writing to the port from the
//! processor (blocking), then by DMA, where the chip's DMA engine feeds the
//! port while the processor is free. The orange LED is on during the DMA round.
//! Each round shows:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 3. a fast moving rainbow, 50 frames a second, for 10 seconds.
//!
//! # Wiring
//!
//! ```text
//! Teensy pin 1 (serial 1 TX) -> 74AHCT125 input (1A)
//! 74AHCT125 output (1Y)      -> strip DIN
//! 74AHCT125 power            -> 5 V, and its ground to the strip's ground
//! Teensy GND                 -> strip GND (all grounds joined)
//! ```
//!
//! The Teensy 4's pins are NOT 5 V tolerant: never connect a 5 V signal to one.

// No operating system here, and no ordinary `main`: the BSP provides start-up.
#![no_std]
#![no_main]

use core::fmt;

use cortex_m::asm::delay as wait_cycles;
use ledkit_core::prelude::*;
use ledkit_uart::{buffer_len, ClocklessUart};
use teensy4_bsp as bsp;
use teensy4_panic as _; // If the program panics, stop safely.

use bsp::board;
use bsp::hal;
use hal::ccm::{clock_gate, uart_clk};
use hal::lpuart::{Baud, Direction, Lpuart};

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected.
const LED_COUNT: usize = 100;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// The serial port's speed. 4 Mbaud suits a WS2812B (see `ledkit-uart`'s
/// README for other LED types).
const BAUD: u32 = 4_000_000;

/// How bright the strip may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// ---------------------------------------------------------------------------
// The serial port's clock
// ---------------------------------------------------------------------------

/// The Teensy's serial ports run from a 24 MHz crystal. The board support
/// package divides it by 3 (8 MHz), which tops out at 2 Mbaud, too slow for
/// NeoPixels, so this program sets the divider to 1.
const UART_CLOCK_HZ: u32 = 24_000_000;

/// The port's speed settings for `BAUD`: 24 MHz / (6 x 1) = 4 Mbaud exactly.
const BAUD_SETTINGS: Baud = Baud::compute(UART_CLOCK_HZ, BAUD);

/// The speed the port really runs at, which ledkit needs to know.
const ACTUAL_BAUD: u32 = BAUD_SETTINGS.value(UART_CLOCK_HZ);

/// How many bytes ledkit needs for the encoded frame, worked out when the
/// program is compiled. A baud that can't drive `LedType` stops the build.
const BUFFER_LEN: usize = buffer_len::<LedType>(LED_COUNT, ACTUAL_BAUD);

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// Waits by counting processor cycles. ledkit uses it for the pause after
/// each frame, and the program for its own pauses.
struct CycleDelay;

impl embedded_hal::delay::DelayNs for CycleDelay {
    fn delay_ns(&mut self, ns: u32) {
        // 600 cycles per microsecond at the Teensy 4's 600 MHz.
        let cycles = u64::from(ns) * u64::from(board::ARM_FREQUENCY) / 1_000_000_000;
        wait_cycles(u32::try_from(cycles).unwrap_or(u32::MAX));
    }
}

impl embedded_hal_async::delay::DelayNs for CycleDelay {
    async fn delay_ns(&mut self, ns: u32) {
        embedded_hal::delay::DelayNs::delay_ns(self, ns);
    }
}

/// The serial port fed by DMA, as an async `embedded-io` writer that ledkit
/// can use. Each write hands the whole buffer to the DMA engine.
struct DmaUart<'a> {
    uart: &'a mut Lpuart,
    channel: &'a mut hal::dma::channel::Channel,
}

/// A DMA error, in the shape `embedded-io` wants.
#[derive(Debug)]
struct DmaError(hal::dma::Error);

impl fmt::Display for DmaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DMA error: {}", self.0)
    }
}

impl core::error::Error for DmaError {}

impl embedded_io::Error for DmaError {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl embedded_io::ErrorType for DmaUart<'_> {
    type Error = DmaError;
}

impl embedded_io_async::Write for DmaUart<'_> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, DmaError> {
        self.uart
            .dma_write(self.channel, buf)
            .await
            .map_err(DmaError)?;
        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), DmaError> {
        // Waits until the last bit has left the port. This never fails.
        let _ = embedded_io::Write::flush(self.uart);
        Ok(())
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
        for color in [Rgb::RED, Rgb::GREEN, Rgb::BLUE, Rgb::WHITE] {
            $strip.fill(color);
            $show;
            $delay.delay_ms(1000);
        }
        // Counting: LED 0 red, every 10th LED white, the last LED blue.
        $strip.fill(Rgb::BLACK);
        for i in (10..LED_COUNT).step_by(10) {
            $strip.set(i, Rgb::WHITE);
        }
        $strip.set(0, Rgb::RED);
        $strip.set(LED_COUNT - 1, Rgb::BLUE);
        $show;
        $delay.delay_ms(5000);
        // A fast rainbow, a full color circle every 60 LEDs.
        for _ in 0..500 {
            for i in 0..LED_COUNT {
                let spread = (i * 65_536 / 60) as u16;
                $strip.set(i, Rgb::from_hue($hue.shifted(spread)));
            }
            $show;
            $hue = $hue.shifted(768);
            $delay.delay_ms(20);
        }
    }};
}

#[bsp::rt::entry]
fn main() -> ! {
    use embedded_hal::delay::DelayNs;

    let board::Resources {
        pins,
        lpuart6,
        mut gpio2,
        mut dma,
        mut ccm,
        ..
    } = board::t40(board::instances());
    // The Teensy 4.1 and the MicroMod use `board::t41` / `board::tmm` instead.

    // Run the serial ports from the full 24 MHz crystal. The clock may only
    // change while the ports' clock gates are off.
    for gate in clock_gate::UART_CLOCK_GATES {
        gate.set(&mut ccm, clock_gate::OFF);
    }
    uart_clk::set_divider(&mut ccm, 1);
    for gate in clock_gate::UART_CLOCK_GATES {
        gate.set(&mut ccm, clock_gate::ON);
    }

    // Serial port 1: transmit on pin 1. Its receive pin (0) is claimed but not
    // used. 8 data bits, no parity, 1 stop bit. Then set the real speed and
    // invert the output, so the line rests low as NeoPixels need.
    let mut uart: Lpuart = board::lpuart(lpuart6, pins.p1, pins.p0, 115_200);
    uart.disable(|port| {
        port.set_baud(&BAUD_SETTINGS);
        port.set_inversion(Direction::Tx, true);
    });

    let led = board::led(&mut gpio2, pins.p13);
    let mut channel = dma[0].take().unwrap();
    let mut delay = CycleDelay;
    let mut hue = Hue::RED;

    loop {
        // Round one: the processor writes to the port.
        led.clear();
        {
            let output = ClocklessUart::<LedType, _, _, _>::new(
                &mut uart,
                ACTUAL_BAUD,
                CycleDelay,
                [0u8; BUFFER_LEN],
            )
            .unwrap();
            let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
            run_round!(strip, strip.show().unwrap(), delay, hue);
        }

        // Round two: the DMA engine feeds the port.
        led.set();
        {
            let port = DmaUart {
                uart: &mut uart,
                channel: &mut channel,
            };
            let output = ClocklessUart::<LedType, _, _, _>::new(
                port,
                ACTUAL_BAUD,
                CycleDelay,
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
