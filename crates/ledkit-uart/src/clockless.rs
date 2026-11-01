//! The one-wire LED output over a serial port.

use crate::Error;
use core::marker::PhantomData;
use ledkit_core::encode::UartEncoding;
use ledkit_core::{wire, BufferTooSmall, Chip, ClocklessTiming, Output, OutputAsync, TimingError};

/// Drives one-wire LEDs (NeoPixels and friends) from a serial port.
///
/// Set up the port first with your board's HAL: 8 data bits, no parity, 1
/// stop bit, its transmit line **inverted**, and a baud that suits your LED
/// type (4 Mbaud for a WS2812B). Then hand over the port, the baud it really
/// runs at, a delay, and a buffer for the encoded frame
/// ([`buffer_len`] tells you how big).
///
/// Each frame is encoded into the buffer, written to the port in full,
/// flushed, and followed by a pause of the LED's reset time (280 µs for a
/// WS2812B) so the LEDs show it. That pause uses the delay you gave.
///
/// The very first frame, and a frame after a failed or cancelled send, get
/// the same pause before them too. That clears whatever was on the line
/// before (a port that rested high until its inversion was switched on, or
/// half a frame), so the LEDs never read it as part of the new frame.
///
/// # Example
///
/// ```ignore
/// use ledkit_core::{Strip, Ws2812b};
/// use ledkit_uart::{buffer_len, ClocklessUart};
///
/// const LED_COUNT: usize = 30;
/// const BAUD: u32 = 4_000_000;
/// // `uart` is your HAL's serial port, inverted, at 4 Mbaud;
/// // `delay` is anything that implements `embedded_hal::delay::DelayNs`.
/// let output = ClocklessUart::<Ws2812b, _, _, _>::new(
///     uart,
///     BAUD,
///     delay,
///     [0u8; buffer_len::<Ws2812b>(LED_COUNT, BAUD)],
/// )
/// .unwrap();
/// let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
/// ```
#[derive(Debug)]
pub struct ClocklessUart<C: Chip, U, D, B> {
    uart: U,
    delay: D,
    buffer: B,
    encoding: UartEncoding,
    reset_us: u32,
    /// Whether the line may not have rested low for the reset time yet:
    /// true at start-up and after a send that didn't finish.
    pause_first: bool,
    chip: PhantomData<C>,
}

impl<C: Chip, U, D, B: AsMut<[u8]>> ClocklessUart<C, U, D, B> {
    /// Creates the output. `baud` is the rate the port **really** runs at,
    /// which may differ a little from the one you asked your HAL for.
    ///
    /// # Errors
    ///
    /// Returns [`TimingError::UartBaudUnusable`] if no pattern at that baud
    /// fits the LED's timing. The error names the nearest bauds that work.
    pub fn new(uart: U, baud: u32, delay: D, buffer: B) -> Result<Self, TimingError> {
        let timing = const { ClocklessTiming::of::<C>() };
        let encoding = timing.uart_encoding(baud)?;
        Ok(Self {
            uart,
            delay,
            buffer,
            encoding,
            reset_us: timing.reset_us,
            pause_first: true,
            chip: PhantomData,
        })
    }

    /// The bit pattern in use.
    #[must_use]
    pub fn encoding(&self) -> UartEncoding {
        self.encoding
    }

    /// The serial port, for reading its state.
    #[must_use]
    pub fn port(&self) -> &U {
        &self.uart
    }

    /// The delay used for the pause after each frame.
    #[must_use]
    pub fn delay(&self) -> &D {
        &self.delay
    }

    /// Gives back the port, the delay and the buffer.
    #[must_use]
    pub fn release(self) -> (U, D, B) {
        (self.uart, self.delay, self.buffer)
    }

    /// Encodes LED wire bytes into the buffer, returning how many serial bytes to send.
    pub(crate) fn encode_wire(
        &mut self,
        wire: impl IntoIterator<Item = u8>,
    ) -> Result<usize, BufferTooSmall> {
        self.encoding.encode_from_iter(wire, self.buffer.as_mut())
    }
}

impl<C, U, D, B> ClocklessUart<C, U, D, B>
where
    C: Chip,
    U: embedded_io::Write,
    D: embedded_hal::delay::DelayNs,
    B: AsMut<[u8]>,
{
    /// Sends the first `used` bytes of the buffer, then waits out the reset time.
    pub(crate) fn send(&mut self, used: usize) -> Result<(), Error<U::Error>> {
        if self.pause_first {
            self.delay.delay_us(self.reset_us);
        }
        // Until this send has finished, the next one must pause first.
        self.pause_first = true;
        let frame = &self.buffer.as_mut()[..used];
        self.uart.write_all(frame).map_err(Error::Uart)?;
        self.uart.flush().map_err(Error::Uart)?;
        self.delay.delay_us(self.reset_us);
        self.pause_first = false;
        Ok(())
    }
}

impl<C, U, D, B> ClocklessUart<C, U, D, B>
where
    C: Chip,
    U: embedded_io_async::Write,
    D: embedded_hal_async::delay::DelayNs,
    B: AsMut<[u8]>,
{
    /// Like [`send`](Self::send), but `async`.
    pub(crate) async fn send_async(&mut self, used: usize) -> Result<(), Error<U::Error>> {
        if self.pause_first {
            self.delay.delay_us(self.reset_us).await;
        }
        // Until this send has finished (it may also be cancelled), the next
        // one must pause first.
        self.pause_first = true;
        let frame = &self.buffer.as_mut()[..used];
        self.uart.write_all(frame).await.map_err(Error::Uart)?;
        self.uart.flush().await.map_err(Error::Uart)?;
        self.delay.delay_us(self.reset_us).await;
        self.pause_first = false;
        Ok(())
    }
}

impl<C, U, D, B> Output<C> for ClocklessUart<C, U, D, B>
where
    C: Chip,
    U: embedded_io::Write,
    D: embedded_hal::delay::DelayNs,
    B: AsMut<[u8]>,
{
    type Error = Error<U::Error>;

    fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_wire(wire::clockless_bytes::<C, _>(pixels))?;
        self.send(used)
    }
}

impl<C, U, D, B> OutputAsync<C> for ClocklessUart<C, U, D, B>
where
    C: Chip,
    U: embedded_io_async::Write,
    D: embedded_hal_async::delay::DelayNs,
    B: AsMut<[u8]>,
{
    type Error = Error<U::Error>;

    async fn write<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        let used = self.encode_wire(wire::clockless_bytes::<C, _>(pixels))?;
        self.send_async(used).await
    }
}

/// How many buffer bytes [`ClocklessUart`] needs for `led_count` LEDs at `baud`.
///
/// A `const fn`, so it can size an array at compile time.
///
/// # Panics
///
/// Panics (stopping the build, when used in a constant) if the baud can't
/// drive this LED type. [`ClocklessUart::new`]'s error names bauds that work.
#[must_use]
pub const fn buffer_len<C: Chip>(led_count: usize, baud: u32) -> usize {
    match ClocklessTiming::of::<C>().uart_encoding(baud) {
        Ok(encoding) => encoding.buffer_len(led_count * C::ORDER.len()),
        Err(_) => panic!(
            "this baud can't drive this LED type; see ClocklessUart::new's error for working bauds"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{pair, Event, FakeDelay, FakeError, FakePort};
    use crate::Error;
    use ledkit_core::{
        wire, Brightness, ClocklessTiming, Rgb, Rgbw, Sk6812Rgbw, Strip, TimingError, Ws2812b,
    };

    const BAUD: u32 = 4_000_000;
    type Uart<const N: usize> = ClocklessUart<Ws2812b, FakePort, FakeDelay, [u8; N]>;

    fn uart<const N: usize>() -> Uart<N> {
        ClocklessUart::new(FakePort::default(), BAUD, FakeDelay::default(), [0u8; N]).unwrap()
    }

    /// What one frame of `pixels` should look like on the port.
    fn expected(pixels: &[Rgb]) -> Vec<u8> {
        let e = ClocklessTiming::of::<Ws2812b>()
            .uart_encoding(BAUD)
            .unwrap();
        let wire: Vec<u8> = wire::clockless_bytes::<Ws2812b, _>(pixels.iter().copied()).collect();
        let mut out = vec![0u8; e.buffer_len(wire.len())];
        e.encode(wire, &mut out).unwrap();
        out
    }

    fn red_strip<const B: usize>(output: Uart<B>) -> Strip<Ws2812b, 2, Uart<B>> {
        let mut strip = Strip::new(output);
        strip.set_dithering(false);
        strip.set_brightness(Brightness::FULL);
        strip.set(0, Rgb::RED);
        strip
    }

    #[test]
    fn sends_the_encoded_frame_then_flushes_and_waits() {
        let mut strip = red_strip(uart::<24>());
        strip.show().unwrap();
        let port = strip.output().port();
        assert_eq!(port.bytes, expected(&[Rgb::RED, Rgb::BLACK]));
        assert_eq!(port.flushes, 1);
        // The first frame also gets a reset pause before it (start-up).
        assert_eq!(strip.output().delay().waits_us, [280, 280]);
    }

    #[test]
    fn first_frame_pauses_writes_flushes_then_pauses() {
        let (port, delay, log) = pair();
        let mut strip = red_strip(ClocklessUart::new(port, BAUD, delay, [0u8; 24]).unwrap());
        strip.show().unwrap();
        strip.show().unwrap();
        assert_eq!(
            *log.borrow(),
            [
                Event::Wait(280),
                Event::Write(24),
                Event::Flush,
                Event::Wait(280),
                Event::Write(24),
                Event::Flush,
                Event::Wait(280),
            ]
        );
    }

    #[test]
    fn mid_frame_error_skips_flush_and_pause() {
        let (mut port, delay, log) = pair();
        port.max_per_write = 3;
        port.fail_on_write = Some(2);
        let mut strip = red_strip(ClocklessUart::new(port, BAUD, delay, [0u8; 24]).unwrap());
        assert_eq!(strip.show(), Err(Error::Uart(FakeError)));
        assert_eq!(
            *log.borrow(),
            [Event::Wait(280), Event::Write(3), Event::Write(3)]
        );
    }

    #[test]
    fn retry_after_an_error_pauses_first() {
        let (mut port, delay, log) = pair();
        port.fail_on_write = Some(1);
        let mut strip = red_strip(ClocklessUart::new(port, BAUD, delay, [0u8; 24]).unwrap());
        strip.show().unwrap();
        assert_eq!(strip.show(), Err(Error::Uart(FakeError)));
        log.borrow_mut().clear();
        strip.show().unwrap();
        assert_eq!(
            *log.borrow(),
            [
                Event::Wait(280),
                Event::Write(24),
                Event::Flush,
                Event::Wait(280),
            ]
        );
    }

    #[test]
    fn async_retry_after_an_error_pauses_first() {
        let (mut port, delay, log) = pair();
        port.fail_on_write = Some(1);
        let mut strip = red_strip(ClocklessUart::new(port, BAUD, delay, [0u8; 24]).unwrap());
        embassy_futures::block_on(strip.show_async()).unwrap();
        assert!(embassy_futures::block_on(strip.show_async()).is_err());
        log.borrow_mut().clear();
        embassy_futures::block_on(strip.show_async()).unwrap();
        assert_eq!(log.borrow()[0], Event::Wait(280));
    }

    #[test]
    fn every_frame_waits_the_reset_time() {
        let mut strip = red_strip(uart::<24>());
        strip.show().unwrap();
        strip.show().unwrap();
        let frame = expected(&[Rgb::RED, Rgb::BLACK]);
        assert_eq!(strip.output().port().bytes, [frame.clone(), frame].concat());
        assert_eq!(strip.output().delay().waits_us, [280, 280, 280]);
    }

    #[test]
    fn partial_writes_send_the_whole_frame() {
        let port = FakePort {
            max_per_write: 3,
            ..FakePort::default()
        };
        let output = ClocklessUart::new(port, BAUD, FakeDelay::default(), [0u8; 24]).unwrap();
        let mut strip = red_strip(output);
        strip.show().unwrap();
        assert_eq!(
            strip.output().port().bytes,
            expected(&[Rgb::RED, Rgb::BLACK])
        );
    }

    #[test]
    fn too_small_buffer_sends_nothing() {
        let mut strip = red_strip(uart::<23>());
        assert!(matches!(strip.show(), Err(Error::BufferTooSmall(_))));
        assert_eq!(strip.output().port().bytes, [0u8; 0]);
        assert_eq!(strip.output().port().flushes, 0);
        assert_eq!(strip.output().delay().waits_us, [0u32; 0]);
    }

    #[test]
    fn port_error_is_returned() {
        let port = FakePort {
            fail_on_write: Some(0),
            ..FakePort::default()
        };
        let output = ClocklessUart::new(port, BAUD, FakeDelay::default(), [0u8; 24]).unwrap();
        let mut strip = red_strip(output);
        assert_eq!(strip.show(), Err(Error::Uart(FakeError)));
        assert_eq!(strip.output().port().flushes, 0);
        assert_eq!(strip.output().delay().waits_us, [280]);
    }

    #[test]
    fn unusable_baud_fails_at_new() {
        let result: Result<Uart<24>, _> = ClocklessUart::new(
            FakePort::default(),
            1_000_000,
            FakeDelay::default(),
            [0u8; 24],
        );
        assert!(matches!(
            result,
            Err(TimingError::UartBaudUnusable {
                baud: 1_000_000,
                ..
            })
        ));
    }

    #[test]
    fn rgbw_frames() {
        let output: ClocklessUart<Sk6812Rgbw, _, _, _> =
            ClocklessUart::new(FakePort::default(), BAUD, FakeDelay::default(), [0u8; 32]).unwrap();
        let mut strip: Strip<Sk6812Rgbw, 2, _> = Strip::new(output);
        strip.set(0, Rgbw::new(0, 0, 0, 255));
        strip.show().unwrap();
        assert_eq!(strip.output().port().bytes.len(), 32);
    }

    #[test]
    fn empty_frame_flushes_and_waits() {
        let mut strip: Strip<Ws2812b, 0, _> = Strip::new(uart::<0>());
        strip.show().unwrap();
        assert_eq!(strip.output().port().bytes, [0u8; 0]);
        assert_eq!(strip.output().port().flushes, 1);
        assert_eq!(strip.output().delay().waits_us, [280, 280]);
    }

    #[test]
    fn async_matches_blocking() {
        let mut blocking = red_strip(uart::<24>());
        blocking.show().unwrap();
        let mut asynchronous = red_strip(uart::<24>());
        embassy_futures::block_on(asynchronous.show_async()).unwrap();
        assert_eq!(
            asynchronous.output().port().bytes,
            blocking.output().port().bytes
        );
        assert_eq!(asynchronous.output().port().flushes, 1);
        assert_eq!(asynchronous.output().delay().waits_us, [280, 280]);
    }

    #[test]
    fn async_port_error_is_returned() {
        let port = FakePort {
            fail_on_write: Some(0),
            ..FakePort::default()
        };
        let output = ClocklessUart::new(port, BAUD, FakeDelay::default(), [0u8; 24]).unwrap();
        let mut strip = red_strip(output);
        assert_eq!(
            embassy_futures::block_on(strip.show_async()),
            Err(Error::Uart(FakeError))
        );
        assert_eq!(strip.output().delay().waits_us, [280]);
    }

    #[test]
    fn buffer_len_is_const() {
        const N: usize = buffer_len::<Ws2812b>(30, 4_000_000);
        assert_eq!(N, 360);
    }

    #[test]
    fn release_gives_everything_back() {
        let (port, delay, buffer) = uart::<24>().release();
        assert_eq!(port.bytes, [0u8; 0]);
        assert_eq!(delay.waits_us, [0u32; 0]);
        assert_eq!(buffer.len(), 24);
    }
}
