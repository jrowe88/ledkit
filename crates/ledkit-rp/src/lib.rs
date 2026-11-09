//! A ledkit backend that drives NeoPixels with the PIO of the Raspberry Pi RP2040 and
//! RP2350 (Pico, Pico 2, Feather RP2040 and friends), through `rp2040-hal` or
//! `rp235x-hal` (bare metal, `no_std`), blocking or async.
//!
//! These "clockless" LEDs (WS2812B and friends) are driven over one data wire with a
//! precisely timed pulse train: every color bit is a high pulse followed by a low pulse,
//! and the pulse widths tell `0` from `1`. A PIO state machine is a tiny processor
//! beside the main cores that makes these pulses with cycle-exact timing, so
//! interrupts don't disturb them.
//!
//! ## Which output do I use?
//!
//! | Output | How the data reaches the PIO | Use it for |
//! |---|---|---|
//! | [`PioOutput`] | the processor pushes each LED into the state machine's FIFO | short strips, simple programs; no buffer, no DMA channel |
//! | [`PioDmaOutput`] | a DMA channel streams a buffer into the FIFO | long strips, busy programs, async; needs a `'static` buffer (4 bytes per LED) and a DMA channel |
//!
//! Both implement [`Output`](ledkit_core::Output) (blocking, `strip.show()`) and
//! [`OutputAsync`](ledkit_core::OutputAsync) (`strip.show_async().await`).
//!
//! DotStars (APA102, SK9822) have a clock wire and don't use PIO here: drive them with
//! `ledkit-spi`'s `ClockedSpi` on the chip's SPI peripheral (the `rp2040-pio` and
//! `rp2350-pio` examples include a DotStar round that does exactly that).
//!
//! ## Quick tour
//!
//! This can only run on an RP chip, so the snippet is not run by `cargo test`. It is
//! written for the RP2350 (pick your chip with a Cargo feature, see "Features"):
//!
//! ```ignore
//! use hal::pio::PIOExt;
//! use ledkit_core::prelude::*;
//! use ledkit_rp::PioOutput;
//! use rp235x_hal as hal;
//!
//! let mut pac = hal::pac::Peripherals::take().unwrap();
//! // ... the usual clocks, `Sio` and `Pins` set-up ...
//! let timer = hal::Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);
//! let (mut pio, sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
//!
//! let output = PioOutput::<Ws2812b, _, _, _, _>::new(
//!     &mut pio,
//!     sm0,
//!     pins.gpio16.into_function(),            // the strip's data pin
//!     clocks.system_clock.freq().to_Hz(),     // ledkit times the pulses from this
//!     timer,                                  // and the reset gap from this
//! )
//! .unwrap();
//! let mut strip = Strip::<Ws2812b, 30, _>::new(output);
//! strip.fill(Rgb::BLUE);
//! strip.show().unwrap();
//! ```
//!
//! **ledkit never sets up the chip.** Your program sets up the clocks, splits the PIO
//! block and the DMA channels, and switches the pin to the PIO function; ledkit takes
//! the pieces in one constructor call.
//!
//! ## Timing
//!
//! The program is the Raspberry Pi `ws2812.pio` one: every bit lasts the same number
//! of PIO cycles, split into "high", "high for a 1 / low for a 0" and "low". ledkit
//! works these out from your LED's datasheet windows and the system clock you pass
//! (`sys_hz`), using `ledkit_core`'s equal-period search, and picks a **whole** clock
//! divider (a fractional one would make the pulses wobble). At 125 MHz (RP2040) and
//! 150 MHz (RP2350) every LED in the catalogue fits. If you change the system clock,
//! pass the new rate.
//!
//! ## The reset gap
//!
//! The LEDs latch the new colors once the data line has been low for the chip's reset
//! time (280 us for the WS2812B). The state machine holds the line low whenever it
//! has no data. Each `show()` returns when the frame's last bit is out, and the next
//! one waits until the reset time has passed since then, using the timer you passed.
//! So two `show()` calls back to back both take effect, and frames sent less often
//! than that never wait.
//!
//! ## Async
//!
//! `rp2040-hal` and `rp235x-hal` have no interrupt-driven async for the PIO or DMA, so
//! the async outputs wait by **yielding**: whenever the FIFO is full, the DMA busy or
//! the reset gap not over, the task lets the executor run other tasks and asks to be
//! polled again. Other tasks run while a frame goes out (with [`PioDmaOutput`] the
//! frame can't be disturbed by them), but the core doesn't sleep. With
//! [`PioOutput`], keep other tasks' busy stretches well under 240 us (the FIFO's 8
//! LEDs) or the frame splits.
//!
//! Don't cancel `show_async()` with [`PioOutput`] (for example by dropping it inside
//! `select`): a frame dropped half way leaves a partial frame on the LEDs. With
//! [`PioDmaOutput`] the DMA finishes the frame anyway and the next call waits for it.
//!
//! ## Wiring
//!
//! One data wire from a GPIO pin to the LED's data-in, a shared ground, and 5 V power
//! for the LEDs. The RP chips have 3.3 V outputs; most WS2812B strips need a level
//! shifter such as a 74AHCT125 (the Feather RP2040 SCORPIO has one built in on
//! GPIO16 to GPIO23). Any GPIO works with either PIO block.
//!
//! ## The Pico W and Pico 2 W radio (CYW43)
//!
//! The Wi-Fi chip on the Pico W boards is driven through a PIO state machine and a DMA
//! channel too: Embassy's `cyw43-pio` driver usually takes PIO0 state machine 0 and
//! DMA channel 0, and the radio uses GPIO23, 24, 25 and 29. ledkit only uses what you
//! hand it, so give it a different PIO block (PIO1 or PIO2) and a different DMA
//! channel, as the `rp2350-pio` example does, and use another GPIO for the strip.
//!
//! ## PIO program space
//!
//! Each output installs its own 4-instruction program; a PIO block holds 32. So up to
//! four outputs fit on one PIO block (one per state machine).
//! [`Error::NoProgramSpace`] says when a block is full. `release()` on either output
//! stops the state machine, removes the program, and hands the parts back.
//!
//! ## Building
//!
//! Pick the chip with one Cargo feature (see below); your own project provides the
//! rest of the start-up (the HAL's `rt` feature, the linker script, the boot block).
//! Copy it from `examples/rp2040-pio` or `examples/rp2350-pio` in the repository.
//! Targets: `thumbv6m-none-eabi` for the RP2040; `thumbv8m.main-none-eabihf` (Arm
//! Cortex-M33) or `riscv32imac-unknown-none-elf` (Hazard3 RISC-V) for the RP2350.
//! All build with stable Rust.
//!
//! ## Coming from `smart-leds`?
//!
//! With the `smart-leds` feature, both outputs implement `smart-leds`'
//! `SmartLedsWrite` and `SmartLedsWriteAsync`, so code written for `ws2812-pio` needs
//! only a new constructor. Colors are sent exactly as given: no brightness, gamma or
//! dithering is added.
//!
//! ## Tested on hardware
//!
//! | Board | Output | LEDs | Result |
//! |---|---|---|---|
//! | Adafruit Feather RP2040 SCORPIO | `PioOutput`, blocking (PIO0, GPIO4) | the onboard NeoPixel (`examples/rp2040-pio`, round 0) | pass (2026-11-08): colors in order, back-to-back frames end on the second color, smooth rainbow |
//! | Adafruit Feather RP2040 SCORPIO | `PioOutput` and `PioDmaOutput`, blocking and async (PIO0, GPIO16, DMA channel 0) | WS2812-type strip, 100 LEDs, on the board's 5 V NeoPixel output 0 | pass (2026-11-08), all four rounds: solid colors, back to back, counting, a rainbow at 50 frames a second; then every frame length from 1 to 150 LEDs |
//!
//! Still untested: the RP2350 (Pico 2 W, Arm and RISC-V cores), RGBW strips, and long
//! strips with a busy processor. The timing search and word packing are also covered by
//! host tests in `ledkit-core` (`encode::equal_period`, `encode::pio_timing`,
//! `encode::pio`). If you try it on other hardware, please report what you see.
//!
//! ## Features
//!
//! - `rp2040`: the RP2040, through `rp2040-hal`.
//! - `rp235x`: the RP2350, through `rp235x-hal` (Arm or RISC-V cores).
//! - `smart-leds`: the `smart-leds` write traits for both outputs.
//!
//! Pick exactly one of `rp2040` and `rp235x`.
#![no_std]

#[cfg(all(feature = "rp2040", feature = "rp235x"))]
compile_error!("ledkit-rp: pick one chip feature, `rp2040` or `rp235x`, not both");
#[cfg(not(any(feature = "rp2040", feature = "rp235x")))]
compile_error!("ledkit-rp: pick a chip feature, `rp2040` or `rp235x`");

#[cfg(feature = "rp2040")]
use rp2040_hal as hal;
#[cfg(all(feature = "rp235x", not(feature = "rp2040")))]
use rp235x_hal as hal;

mod clock;
mod cpu;
mod dma;
mod error;
mod program;
#[cfg(feature = "smart-leds")]
mod smart_leds;

pub use clock::MicrosecondClock;
pub use cpu::PioOutput;
pub use dma::{buffer_len, PioDmaOutput};
pub use error::Error;
