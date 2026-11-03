//! A hardware check for the ESP32-S3: drive one NeoPixel strip every way esp-hal can.
//!
//! The strip's data wire stays on one pin while the program hands it from one
//! peripheral to the next. Each "round" is one way of making the pulses:
//!
//! | Round | Peripheral | How the data gets there |
//! |---|---|---|
//! | 1 | RMT, blocking | the processor refills the RMT's memory while it sends |
//! | 2 | RMT, async | the same, refilled when the async task is polled |
//! | 3 | SPI, `ClocklessSpi::new` | the processor feeds the SPI's small buffer |
//! | 4 | SPI with DMA, `new_packed`, blocking | DMA streams the whole frame, no gaps |
//! | 5 | SPI with DMA, `new_packed`, async | the same, awaited |
//! | 6 | UART, blocking | the processor feeds the serial port |
//! | 7 | UART, async | the same, awaited |
//!
//! Every round shows the same pattern:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. two frames back to back, red then blue: the strip must end blue;
//! 3. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 4. a fast moving rainbow, 50 frames a second, for 10 seconds.
//!
//! Then the RMT sends frames of every length from 1 to `SWEEP_MAX` LEDs (a dim bar
//! that grows along the strip), and the RMT rainbow runs forever. Press RESET to run
//! the whole check again.

// No operating system here, and no ordinary `main`: esp-hal provides the start-up.
#![no_std]
#![no_main]

use esp_backtrace as _; // If the program panics, print why over serial.
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    dma_rx_buffer, dma_tx_buffer,
    gpio::interconnect::OutputSignal,
    main,
    rmt::{PulseCode, Rmt},
    spi::{
        master::{Config as SpiConfig, Spi},
        Mode,
    },
    time::Rate,
    uart::{ClockSource, Config as UartConfig, UartTx},
};
use esp_println::println;
use ledkit_core::prelude::*;
use ledkit_core::Output;
use ledkit_esp::RmtOutput;
use ledkit_spi::ClocklessSpi;
use ledkit_uart::ClocklessUart;

// The bootloader on the chip wants a small "this is an app" label. This macro
// writes it for us.
esp_bootloader_esp_idf::esp_app_desc!();

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

// The data wire is GPIO5 (marked "5" on the Feather), through a 74AHCT125 level
// shifter. To use another pin, change every `GPIO5` in `main` below.

/// The RMT's clock. ledkit divides it down to 10 MHz.
const RMT_HZ: u32 = 80_000_000;

/// The SPI speed for round 3. 3.2 MHz is exact on the S3 (80 MHz / 25).
const SPI_HZ: u32 = 3_200_000;

/// The SPI speed for the DMA rounds. `new_packed` needs 4 MHz for a WS2812B
/// (5 SPI bits per LED bit), also exact (80 MHz / 20).
const SPI_PACKED_HZ: u32 = 4_000_000;

/// The serial port's speed. 4 Mbaud suits a WS2812B.
const BAUD: u32 = 4_000_000;

/// The clock the serial port runs from: the S3's 40 MHz crystal (set below).
const UART_CLOCK_HZ: u32 = 40_000_000;

/// The speed the port really runs at, which ledkit needs to know. The S3 divides
/// its clock in steps of 1/16, so most speeds come out a little off; this repeats
/// esp-hal 1.2's divider maths. 4 Mbaud is exact (40 MHz / 10).
const ACTUAL_BAUD: u32 = {
    let divider_16ths = UART_CLOCK_HZ as u64 * 16 / BAUD as u64;
    (UART_CLOCK_HZ as u64 * 16 / divider_16ths) as u32
};

// Buffer sizes, worked out when the program is compiled. A speed that can't drive
// `LedType` stops the build.
const RMT_LEN: usize = ledkit_esp::buffer_len::<LedType>(LED_COUNT);
const SWEEP_LEN: usize = ledkit_esp::buffer_len::<LedType>(SWEEP_MAX);
const SPI_LEN: usize = ledkit_spi::buffer_len::<LedType>(LED_COUNT, SPI_HZ);
const SPI_PACKED_LEN: usize = ledkit_spi::buffer_len_packed::<LedType>(LED_COUNT, SPI_PACKED_HZ);
const UART_LEN: usize = ledkit_uart::buffer_len::<LedType>(LED_COUNT, ACTUAL_BAUD);

/// esp-hal's SPI DMA sends a frame in pieces the size of its DMA buffer, with a
/// pause between pieces. Make the buffer hold the whole frame, so it goes out in
/// one piece with no pauses (rounded up to a multiple of 4).
const SPI_DMA_LEN: usize = SPI_PACKED_LEN.next_multiple_of(4);

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
        // Dithering smooths dim fades at 100+ frames a second; this check holds
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
        // With no pause between them, the reset gap must still separate the frames,
        // or the blue frame joins onto the red one and the strip stays red.
        println!("  back to back: red, then blue (should end blue)");
        $strip.fill(Rgb::RED);
        $show;
        $strip.fill(Rgb::BLUE);
        $show;
        $delay.delay_millis(1000);
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

    println!("ledkit ESP32-S3 check: {} LEDs on GPIO5", LED_COUNT);

    println!("ROUND 1: RMT, blocking");
    {
        let rmt = Rmt::new(peripherals.RMT.reborrow(), Rate::from_hz(RMT_HZ)).unwrap();
        let rmt_hz = rmt.frequency().as_hz();
        let output = RmtOutput::<LedType, _, _>::new(
            rmt.channel0,
            peripherals.GPIO5.reborrow(),
            rmt_hz,
            [PulseCode::default(); RMT_LEN],
        )
        .unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), delay, hue);
    }

    println!("ROUND 2: RMT, async");
    {
        let rmt = Rmt::new(peripherals.RMT.reborrow(), Rate::from_hz(RMT_HZ))
            .unwrap()
            .into_async();
        let rmt_hz = rmt.frequency().as_hz();
        let output = RmtOutput::<LedType, _, _>::new(
            rmt.channel0,
            peripherals.GPIO5.reborrow(),
            rmt_hz,
            [PulseCode::default(); RMT_LEN],
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

    println!("ROUND 3: SPI, processor-fed (new, {} Hz)", SPI_HZ);
    {
        let spi = Spi::new(
            peripherals.SPI2.reborrow(),
            SpiConfig::default()
                .with_frequency(Rate::from_hz(SPI_HZ))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_mosi(peripherals.GPIO5.reborrow());
        let output = ClocklessSpi::<LedType, _, _>::new(spi, SPI_HZ, [0u8; SPI_LEN]).unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), delay, hue);
    }

    println!(
        "ROUND 4: SPI with DMA, blocking (new_packed, {} Hz)",
        SPI_PACKED_HZ
    );
    let spi_dma = {
        // DMA buffers live in static memory. The SPI only sends, but esp-hal wants a
        // receive buffer too; a tiny one does.
        let dma_rx = dma_rx_buffer!(4).unwrap();
        let dma_tx = dma_tx_buffer!(SPI_DMA_LEN).unwrap();
        let spi = Spi::new(
            peripherals.SPI2.reborrow(),
            SpiConfig::default()
                .with_frequency(Rate::from_hz(SPI_PACKED_HZ))
                .with_mode(Mode::_0),
        )
        .unwrap()
        .with_mosi(peripherals.GPIO5.reborrow())
        .with_dma(peripherals.DMA_CH0.reborrow())
        .with_buffers(dma_rx, dma_tx);
        let output =
            ClocklessSpi::<LedType, _, _>::new_packed(spi, SPI_PACKED_HZ, [0u8; SPI_PACKED_LEN])
                .unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), delay, hue);
        // Take the bus back to reuse it, with its DMA buffers, in round 5.
        strip.release().release().0
    };

    println!(
        "ROUND 5: SPI with DMA, async (new_packed, {} Hz)",
        SPI_PACKED_HZ
    );
    {
        let output = ClocklessSpi::<LedType, _, _>::new_packed(
            spi_dma.into_async(),
            SPI_PACKED_HZ,
            [0u8; SPI_PACKED_LEN],
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

    // The serial port: 8 data bits, no parity, 1 stop bit (the defaults), at BAUD,
    // clocked from the crystal (as ACTUAL_BAUD assumes). Only its transmit side is
    // used.
    let uart_config = UartConfig::default()
        .with_baudrate(BAUD)
        .with_clock_source(ClockSource::Xtal);

    println!("ROUND 6: UART, blocking ({} baud)", ACTUAL_BAUD);
    {
        // The pin, inverted: the line rests low, as NeoPixels need. The inversion
        // happens in the S3's GPIO matrix, on the way to the pin.
        let pin = OutputSignal::from(peripherals.GPIO5.reborrow()).with_output_inverter(true);
        let uart = UartTx::new(peripherals.UART1.reborrow(), uart_config)
            .unwrap()
            .with_tx(pin);
        let output =
            ClocklessUart::<LedType, _, _, _>::new(uart, ACTUAL_BAUD, delay, [0u8; UART_LEN])
                .unwrap();
        let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
        run_round!(strip, strip.show().unwrap(), delay, hue);
    }

    println!("ROUND 7: UART, async ({} baud)", ACTUAL_BAUD);
    {
        let pin = OutputSignal::from(peripherals.GPIO5.reborrow()).with_output_inverter(true);
        let uart = UartTx::new(peripherals.UART1.reborrow(), uart_config)
            .unwrap()
            .with_tx(pin)
            .into_async();
        let output = ClocklessUart::<LedType, _, _, _>::new(
            uart,
            ACTUAL_BAUD,
            BusyDelay(delay),
            [0u8; UART_LEN],
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

    println!("SWEEP: RMT frames of 1 to {} LEDs", SWEEP_MAX);
    let rmt = Rmt::new(peripherals.RMT, Rate::from_hz(RMT_HZ)).unwrap();
    let rmt_hz = rmt.frequency().as_hz();
    let mut output = RmtOutput::<LedType, _, _>::new(
        rmt.channel0,
        peripherals.GPIO5,
        rmt_hz,
        [PulseCode::default(); SWEEP_LEN],
    )
    .unwrap();
    // A shorter frame only changes the first LEDs and the rest keep their colors,
    // so clear the whole sweep range first. Then the bar grows from dark.
    output
        .write(core::iter::repeat_n(Rgb::BLACK, SWEEP_MAX))
        .unwrap();
    // Straight to the output, so no brightness is applied: send a dim blue.
    let dim_blue = Rgb::new(0, 0, 24);
    for length in 1..=SWEEP_MAX {
        output
            .write(core::iter::repeat_n(dim_blue, length))
            .unwrap();
        delay.delay_millis(30);
        if length % 25 == 0 {
            println!("  {} LEDs", length);
        }
    }
    delay.delay_millis(1000);

    println!("DONE. RMT rainbow forever; press RESET to run the check again.");
    let mut strip = Strip::<LedType, SWEEP_MAX, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    loop {
        for i in 0..SWEEP_MAX {
            let spread = (i * 65_536 / 60) as u16;
            strip.set(i, Rgb::from_hue(hue.shifted(spread)));
        }
        strip.show().unwrap();
        hue = hue.shifted(768);
        delay.delay_millis(20);
    }
}
