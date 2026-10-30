//! A ledkit backend that drives LEDs over any `embedded-hal` 1.0 SPI bus.
//!
//! Many microcontrollers have an SPI peripheral, and a NeoPixel can be fed
//! from its data pin (MOSI) if the right bytes are sent. `ledkit-spi` works
//! out those bytes for your LED type and your SPI clock, so any board with a
//! HAL that implements the `embedded-hal` SPI traits works, blocking or async.
//!
//! **ledkit never sets up the bus.** You configure the SPI peripheral, its
//! clock speed and its pin with your board's HAL, then hand over the bus and
//! the clock rate it really runs at.
//!
//! ## Quick tour
//!
//! On a real board it looks like this (the setup lines depend on your HAL,
//! here shaped like `esp-hal`):
//!
//! ```ignore
//! use ledkit_core::prelude::*;
//! use ledkit_spi::{buffer_len, ClocklessSpi};
//!
//! const LED_COUNT: usize = 30;
//! const SPI_HZ: u32 = 3_200_000;
//!
//! // Your HAL: an SPI bus at SPI_HZ with the LED data wire on its MOSI pin.
//! let spi = Spi::new(peripherals.SPI2, Config::default().with_frequency(Rate::from_hz(SPI_HZ)))
//!     .unwrap()
//!     .with_mosi(peripherals.GPIO7);
//!
//! let output = ClocklessSpi::<Ws2812b, _, _>::new(
//!     spi,
//!     SPI_HZ,
//!     [0u8; buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)],
//! )
//! .unwrap();
//! let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
//! strip.fill(Color::BLUE);
//! strip.show().unwrap();
//! ```
//!
//! The same code runs on your computer with a pretend bus. This one just
//! counts the bytes it is given:
//!
//! ```
//! use ledkit_core::prelude::*;
//! use ledkit_spi::{buffer_len, ClocklessSpi};
//!
//! /// A pretend SPI bus that remembers how many bytes it was sent.
//! struct CountingBus(usize);
//!
//! impl embedded_hal::spi::ErrorType for CountingBus {
//!     type Error = core::convert::Infallible;
//! }
//!
//! impl embedded_hal::spi::SpiBus for CountingBus {
//!     fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
//!         Ok(())
//!     }
//!     fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
//!         self.0 += words.len();
//!         Ok(())
//!     }
//!     fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> {
//!         Ok(())
//!     }
//!     fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> {
//!         Ok(())
//!     }
//!     fn flush(&mut self) -> Result<(), Self::Error> {
//!         Ok(())
//!     }
//! }
//!
//! const LED_COUNT: usize = 30;
//! const SPI_HZ: u32 = 3_200_000;
//!
//! let output = ClocklessSpi::<Ws2812b, _, _>::new(
//!     CountingBus(0),
//!     SPI_HZ,
//!     [0u8; buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ)],
//! )
//! .unwrap();
//! let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
//! strip.fill(Color::BLUE);
//! strip.show().unwrap();
//!
//! assert_eq!(strip.output().bus().0, buffer_len::<Ws2812b>(LED_COUNT, SPI_HZ));
//! ```
//!
//! ## Which SPI clock?
//!
//! The clock sets how finely the LED pulses can be shaped, so only some
//! speeds work. For a WS2812B (NeoPixel), any speed from 2.3 to 6.8 MHz
//! does; 3.2 MHz is a good pick. If your speed won't work,
//! [`ClocklessSpi::new`] returns an error naming the nearest ones that do.
#![cfg_attr(not(test), no_std)]

mod clocked;
mod clockless;
mod error;
#[cfg(test)]
mod testing;

pub use clocked::ClockedSpi;
pub use clockless::{buffer_len, buffer_len_packed, ClocklessSpi};
pub use error::Error;
