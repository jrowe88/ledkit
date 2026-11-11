//! A hardware check for the ESP32-S3 on ESP-IDF: drive a NeoPixel strip through the
//! RMT (with and without DMA), an SPI bus and a serial port, then the RMT again with
//! the Wi-Fi radio busy.
//!
//! | Round | Output | Wi-Fi |
//! |---|---|---|
//! | 1 | `RmtOutput::new` (ESP-IDF refills the RMT from an interrupt) | off |
//! | 2 | `RmtOutput::new_with_dma` (the RMT reads the frame through DMA) | off |
//! | 3 | `ClocklessSpi` on esp-idf-hal's `SpiBusDriver`, with DMA | off |
//! | 4 | `ClocklessUart` on esp-idf-hal's `UartDriver`, the pin inverted | off |
//! | 5 | `RmtOutput::new` | scanning nonstop |
//! | 6 | `RmtOutput::new_with_dma` | scanning nonstop |
//!
//! Every round shows the same pattern:
//!
//! 1. red, green, blue and white, the whole strip, one second each;
//! 2. two frames back to back, red then blue: the strip must end blue;
//! 3. counting, 5 seconds: LED 0 red, every 10th LED white, the last LED blue;
//! 4. a fast moving rainbow, 50 frames a second, for 10 seconds;
//! 5. frames of every length from 1 to `SWEEP_MAX` LEDs: a dim bar grows along the
//!    strip. (On the ESP32-C6, some lengths once crashed ESP-IDF's RMT driver.)
//!
//! Then the last round's rainbow runs forever. Press RESET to run the check again.
//!
//! The Wi-Fi rounds don't join a network: the radio scans for networks over and
//! over, which keeps it and its interrupts busy, with no password needed.

use std::thread::sleep;
use std::time::Duration;

use esp_idf_hal::delay::Ets;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal::spi::config::{Config as SpiConfig, DriverConfig};
use esp_idf_hal::spi::{Dma, SpiBusDriver, SpiDriver};
use esp_idf_hal::uart::config::Config as UartConfig;
use esp_idf_hal::uart::UartDriver;
use esp_idf_hal::units::Hertz;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi};
use ledkit_core::prelude::*;
use ledkit_core::Output;
use ledkit_esp_idf::RmtOutput;
use ledkit_spi::ClocklessSpi;
use ledkit_uart::ClocklessUart;

// ---------------------------------------------------------------------------
// Settings you may want to change
// ---------------------------------------------------------------------------

/// How many LEDs are connected.
const LED_COUNT: usize = 100;

/// The longest frame in the length sweep. Longer than the strip is fine: the
/// extra LEDs' data runs off the end.
const SWEEP_MAX: usize = 150;

/// Which kind of LED it is. This decides the timing and the color order.
/// If the colors come out wrong, this is the first thing to check.
type LedType = Ws2812b;

/// How bright the strip may get.
const BRIGHTNESS: Brightness = Brightness::from_percent(20);

/// The most current the LEDs may draw, so USB power is enough.
const POWER_LIMIT: Milliamps = Milliamps(500);

// The data wire is GPIO5 (marked "5" on the Feather), through a 74AHCT125 level
// shifter. To use another pin, change `gpio5` in `main` below. The SPI round also
// needs a clock pin, GPIO6 ("6"): leave it unconnected.

/// The SPI speed for round 3. 3.2 MHz is exact on the S3 (80 MHz / 25).
const SPI_HZ: u32 = 3_200_000;

/// The serial port's speed for round 4. 4 Mbaud suits a WS2812B, and is exact from
/// the 80 MHz clock ESP-IDF runs the port from (80 MHz / 20).
const BAUD: u32 = 4_000_000;

// Buffer sizes, worked out when the program is compiled, big enough for the sweep's
// longest frame. A speed that can't drive `LedType` stops the build.
const SPI_LEN: usize = ledkit_spi::buffer_len::<LedType>(SWEEP_MAX, SPI_HZ);
const UART_LEN: usize = ledkit_uart::buffer_len::<LedType>(SWEEP_MAX, BAUD);

fn main() {
    // Makes sure some ESP-IDF patches are linked in. Always the first line.
    esp_idf_hal::sys::link_patches();

    // Get access to the chip's pins and peripherals. This can only be done once.
    let mut peripherals = Peripherals::take().unwrap();
    let mut hue = Hue::RED;

    println!("ledkit ESP32-S3 ESP-IDF check: {LED_COUNT} LEDs on GPIO5");

    // Every round borrows GPIO5 for its output and gives it back when the output is
    // dropped at the end of the round. esp-idf-hal marks this `reborrow` unsafe only
    // because a driver that is `mem::forget`-ed would keep using the pin; this
    // program never forgets one, so each `unsafe` block below is sound.

    println!("ROUND 1: RMT, Wi-Fi off");
    {
        let output =
            RmtOutput::<LedType>::new(unsafe { peripherals.pins.gpio5.reborrow() }).unwrap();
        run_round(output, &mut hue);
    }

    println!("ROUND 2: RMT with DMA, Wi-Fi off");
    {
        let output =
            RmtOutput::<LedType>::new_with_dma(unsafe { peripherals.pins.gpio5.reborrow() })
                .unwrap();
        run_round(output, &mut hue);
    }

    println!("ROUND 3: SPI with DMA ({SPI_HZ} Hz)");
    {
        // With DMA, ESP-IDF sends the whole frame as one transfer. Without it, the
        // SPI sends 64 bytes at a time, with pauses in between.
        let driver = SpiDriver::new(
            unsafe { peripherals.spi2.reborrow() },
            unsafe { peripherals.pins.gpio6.reborrow() }, // clock, unused
            unsafe { peripherals.pins.gpio5.reborrow() }, // data
            None::<esp_idf_hal::gpio::AnyIOPin>,
            &DriverConfig::new().dma(Dma::Auto(4096)),
        )
        .unwrap();
        let spi = SpiBusDriver::new(driver, &SpiConfig::new().baudrate(Hertz(SPI_HZ))).unwrap();
        let output = ClocklessSpi::<LedType, _, _>::new(spi, SPI_HZ, [0u8; SPI_LEN]).unwrap();
        run_round(output, &mut hue);
    }

    println!("ROUND 4: serial port ({BAUD} baud, pin inverted)");
    {
        // The transmit-only `UartTxDriver` would do, but esp-idf-hal 0.47 asks ESP-IDF
        // for no receive buffer, which ESP-IDF 5.5 refuses. So use the full driver,
        // with its receive side on GPIO6 (unconnected, also the SPI round's clock).
        let uart = UartDriver::new(
            unsafe { peripherals.uart1.reborrow() },
            unsafe { peripherals.pins.gpio5.reborrow() }, // transmit: the data wire
            unsafe { peripherals.pins.gpio6.reborrow() }, // receive: unused
            None::<esp_idf_hal::gpio::AnyIOPin>,
            None::<esp_idf_hal::gpio::AnyIOPin>,
            &UartConfig::new().baudrate(Hertz(BAUD)),
        )
        .unwrap();
        println!("  the port runs at {} baud", uart.baudrate().unwrap());
        // Invert the transmit line, so it rests low as NeoPixels need. esp-idf-hal
        // has no setting for this, so call ESP-IDF directly.
        let inverted = unsafe {
            esp_idf_hal::sys::uart_set_line_inverse(
                uart.port(),
                esp_idf_hal::sys::uart_signal_inv_t_UART_SIGNAL_TXD_INV,
            )
        };
        esp_idf_hal::sys::esp!(inverted).unwrap();
        let output =
            ClocklessUart::<LedType, _, _, _>::new(uart, BAUD, Ets, [0u8; UART_LEN]).unwrap();
        run_round(output, &mut hue);
    }

    println!("Starting Wi-Fi: scanning nonstop from now on");
    let sysloop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();
    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs)).unwrap(),
        sysloop,
    )
    .unwrap();
    wifi.set_configuration(&Configuration::Client(ClientConfiguration::default()))
        .unwrap();
    wifi.start().unwrap();
    std::thread::Builder::new()
        .stack_size(8192)
        .spawn(move || loop {
            match wifi.scan() {
                Ok(found) => println!("  (Wi-Fi scan: {} networks)", found.len()),
                Err(e) => println!("  (Wi-Fi scan failed: {e})"),
            }
        })
        .unwrap();

    println!("ROUND 5: RMT, Wi-Fi scanning");
    {
        let output =
            RmtOutput::<LedType>::new(unsafe { peripherals.pins.gpio5.reborrow() }).unwrap();
        run_round(output, &mut hue);
    }

    println!("ROUND 6: RMT with DMA, Wi-Fi scanning");
    let output = RmtOutput::<LedType>::new_with_dma(peripherals.pins.gpio5).unwrap();
    let mut strip = run_round(output, &mut hue);

    println!("DONE. Rainbow forever; press RESET to run the check again.");
    loop {
        rainbow_frame(&mut strip, &mut hue);
        sleep(Duration::from_millis(20));
    }
}

/// Shows one round of the test pattern on `output`, then hands the strip back.
fn run_round<O>(output: O, hue: &mut Hue) -> Strip<LedType, LED_COUNT, O>
where
    O: Output<LedType>,
    O::Error: core::fmt::Debug,
{
    let mut strip = Strip::<LedType, LED_COUNT, _>::new(output);
    strip.set_brightness(BRIGHTNESS);
    strip.set_power_limit(POWER_LIMIT);
    // Dithering smooths dim fades at 100+ frames a second; this check holds still
    // colors, where it would only add flicker.
    strip.set_dithering(false);

    for (name, color) in [
        ("red", Rgb::RED),
        ("green", Rgb::GREEN),
        ("blue", Rgb::BLUE),
        ("white", Rgb::WHITE),
    ] {
        println!("  {name}");
        strip.fill(color);
        strip.show().unwrap();
        sleep(Duration::from_secs(1));
    }

    // With no pause between them, the reset gap must still separate the frames, or
    // the blue frame joins onto the red one and the strip stays red.
    println!("  back to back: red, then blue (should end blue)");
    strip.fill(Rgb::RED);
    strip.show().unwrap();
    strip.fill(Rgb::BLUE);
    strip.show().unwrap();
    sleep(Duration::from_secs(1));

    println!("  counting: LED 0 red, every 10th white, last blue");
    strip.fill(Rgb::BLACK);
    for i in (10..LED_COUNT).step_by(10) {
        strip.set(i, Rgb::WHITE);
    }
    strip.set(0, Rgb::RED);
    strip.set(LED_COUNT - 1, Rgb::BLUE);
    strip.show().unwrap();
    sleep(Duration::from_secs(5));

    println!("  rainbow");
    for _ in 0..500 {
        rainbow_frame(&mut strip, hue);
        sleep(Duration::from_millis(20));
    }

    println!("  sweep: frames of 1 to {SWEEP_MAX} LEDs");
    // A shorter frame only changes the first LEDs and the rest keep their colors,
    // so clear the whole sweep range first. Then the bar grows from dark.
    strip
        .output_mut()
        .write(std::iter::repeat_n(Rgb::BLACK, SWEEP_MAX))
        .unwrap();
    // Straight to the output, so no brightness is applied: send a dim blue.
    let dim_blue = Rgb::new(0, 0, 24);
    for length in 1..=SWEEP_MAX {
        strip
            .output_mut()
            .write(std::iter::repeat_n(dim_blue, length))
            .unwrap();
        sleep(Duration::from_millis(30));
        if length % 25 == 0 {
            println!("    {length} LEDs");
        }
    }
    sleep(Duration::from_secs(1));
    strip
}

/// Fills the strip with a moving rainbow (a full color circle every 60 LEDs) and
/// shows it.
fn rainbow_frame<O>(strip: &mut Strip<LedType, LED_COUNT, O>, hue: &mut Hue)
where
    O: Output<LedType>,
    O::Error: core::fmt::Debug,
{
    for i in 0..LED_COUNT {
        let spread = (i * 65_536 / 60) as u16;
        strip.set(i, Rgb::from_hue(hue.shifted(spread)));
    }
    strip.show().unwrap();
    *hue = hue.shifted(768);
}
