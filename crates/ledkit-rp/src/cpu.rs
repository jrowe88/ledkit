//! [`PioOutput`]: the processor feeds the state machine's FIFO.

use core::marker::PhantomData;

use ledkit_core::encode::pio_words;
use ledkit_core::{wire, Chip, Output, OutputAsync};

use crate::clock::{yield_now, FrameClock, MicrosecondClock};
use crate::hal::gpio::AnyPin;
use crate::hal::pio::{PIOExt, StateMachineIndex, Tx, UninitStateMachine, PIO};
use crate::program::{self, bytes_per_word, Machine};
use crate::Error;

/// A chain of clockless LEDs of type `C` (for example [`ledkit_core::Ws2812b`]) on
/// one pin, driven by a PIO state machine that the **processor** feeds.
///
/// The processor packs each LED into a 32-bit word and pushes it into the state
/// machine's FIFO (8 words deep), which sends it with exact timing. It needs no
/// buffer and no DMA channel. The processor is busy for the whole frame (about
/// 30 us per LED), and if an interrupt holds it up for more than the FIFO's
/// 8 LEDs' worth (about 240 us) mid-frame, the line rests low long enough for the
/// LEDs to latch and the frame splits in two. For long strips or busy programs
/// use [`PioDmaOutput`](crate::PioDmaOutput).
///
/// - `C` is your LED type.
/// - `P` and `SM` are the PIO block and state machine (from `PIO::split`).
/// - `I` is the data pin, already switched to that PIO block's function.
/// - `T` is the timer (see [`MicrosecondClock`]).
///
/// It implements both [`Output`] (blocking) and [`OutputAsync`]. Put it in a
/// [`Strip`](ledkit_core::Strip) and call `strip.show()` or `strip.show_async()`.
///
/// # DotStars
///
/// DotStars (APA102 and SK9822) have their own clock wire and don't need this.
/// Using one here is a build error.
///
/// ```ignore
/// // Does not compile: DotStars need a clocked backend such as ledkit-spi.
/// let _ = PioOutput::<Apa102, _, _, _, _>::new(&mut pio, sm0, pin, sys_hz, timer);
/// ```
///
/// # Example
///
/// ```ignore
/// use hal::pio::PIOExt;
/// use ledkit_core::prelude::*;
/// use ledkit_rp::PioOutput;
///
/// let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
/// let pin = pins.gpio16.into_function();
/// let sys_hz = clocks.system_clock.freq().to_Hz();
/// let output = PioOutput::<Ws2812b, _, _, _, _>::new(&mut pio, sm0, pin, sys_hz, timer)?;
/// let mut strip = Strip::<Ws2812b, 30, _>::new(output);
/// strip.fill(Rgb::BLUE);
/// strip.show()?;
/// ```
pub struct PioOutput<C: Chip, P: PIOExt, SM: StateMachineIndex, I: AnyPin, T> {
    machine: Machine<P, SM, I>,
    tx: Tx<(P, SM)>,
    frame: FrameClock<T>,
    /// A frame was started and its end not yet recorded (an async send dropped
    /// half way).
    draining: bool,
    chip: PhantomData<C>,
}

impl<C, P, SM, I, T> PioOutput<C, P, SM, I, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    /// Installs ledkit's program on `pio` and starts state machine `sm` sending to
    /// `pin`.
    ///
    /// - `pin` must already have the PIO block's function: write
    ///   `pins.gpio16.into_function()` and the type follows from `pio`.
    /// - `sys_hz` is the system clock, in hertz:
    ///   `clocks.system_clock.freq().to_Hz()`. ledkit works out a whole clock
    ///   divider and the pulse lengths from it.
    /// - `timer` is the HAL's `Timer`, used to keep frames the chip's reset time
    ///   apart.
    ///
    /// The program takes 4 of the PIO block's 32 instruction slots.
    ///
    /// # Errors
    ///
    /// [`Error::Timing`] if the chip can't be timed from `sys_hz` (not expected for
    /// the catalogue at any usual clock), and [`Error::NoProgramSpace`] if the PIO
    /// block is full.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = PioOutput::<Ws2812b, _, _, _, _>::new(&mut pio, sm0, pin, sys_hz, timer)?;
    /// ```
    pub fn new(
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        pin: I,
        sys_hz: u32,
        timer: T,
    ) -> Result<Self, Error> {
        let (machine, tx, frame) = program::start::<C, P, SM, I, T>(pio, sm, pin, sys_hz, timer)?;
        Ok(Self {
            machine,
            tx,
            frame,
            draining: false,
            chip: PhantomData,
        })
    }

    /// Stops the state machine, removes ledkit's program from `pio`, and gives back
    /// the state machine and the pin, for example to drive the same strip another
    /// way.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (sm0, pin) = output.release(&mut pio);
    /// let output = PioDmaOutput::<Ws2812b, _, _, _, _, _>::new(&mut pio, sm0, pin, ch1, buffer, sys_hz, timer)?;
    /// ```
    pub fn release(self, pio: &mut PIO<P>) -> (UninitStateMachine<(P, SM)>, I) {
        self.machine.release(pio, self.tx)
    }

    /// Sends wire bytes, blocking until the frame is out.
    pub(crate) fn send_blocking(&mut self, wire: impl Iterator<Item = u8>) {
        let mut words = pio_words(wire, const { bytes_per_word::<C>() }).peekable();
        if self.draining {
            // A dropped async send left a frame going out; let it end first.
            while !self.tx.is_empty() {}
            self.frame.finish_frame();
            self.draining = false;
        }
        if words.peek().is_none() {
            return;
        }
        self.frame.wait_for_gap();
        for word in words {
            while !self.tx.write(word) {}
        }
        while !self.tx.is_empty() {}
        self.frame.finish_frame();
    }

    /// Sends wire bytes, yielding to other tasks whenever the FIFO is full.
    pub(crate) async fn send_async(&mut self, wire: impl Iterator<Item = u8>) {
        let mut words = pio_words(wire, const { bytes_per_word::<C>() }).peekable();
        if self.draining {
            // A dropped send left a frame going out; let it end first.
            while !self.tx.is_empty() {
                yield_now().await;
            }
            self.frame.finish_frame_async().await;
            self.draining = false;
        }
        if words.peek().is_none() {
            return;
        }
        self.frame.wait_for_gap_async().await;
        self.draining = true;
        for word in words {
            while !self.tx.write(word) {
                yield_now().await;
            }
        }
        while !self.tx.is_empty() {
            yield_now().await;
        }
        self.frame.finish_frame_async().await;
        self.draining = false;
    }
}

impl<C, P, SM, I, T> Output<C> for PioOutput<C, P, SM, I, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    type Error = Error;

    /// Sends one frame and returns when its last bit is out. If the previous frame
    /// ended less than the chip's reset time ago, it first waits out the rest.
    fn write<It>(&mut self, pixels: It) -> Result<(), Self::Error>
    where
        It: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        // ledkit puts each pixel's channels in the chip's order (green, red, blue
        // for the WS2812B).
        self.send_blocking(wire::clockless_bytes::<C, _>(pixels));
        Ok(())
    }
}

impl<C, P, SM, I, T> OutputAsync<C> for PioOutput<C, P, SM, I, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    T: MicrosecondClock,
{
    type Error = Error;

    /// Sends one frame, yielding to other tasks while the FIFO is full, and
    /// returns when its last bit is out.
    ///
    /// Other tasks run while this one waits for room in the FIFO, so keep their
    /// busy stretches well under the FIFO's 8 LEDs (about 240 us), or the frame
    /// splits. Don't cancel this future (for example inside `select`): a frame
    /// dropped half way leaves a partial frame on the LEDs.
    async fn write<It>(&mut self, pixels: It) -> Result<(), Self::Error>
    where
        It: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.send_async(wire::clockless_bytes::<C, _>(pixels)).await;
        Ok(())
    }
}

impl<C: Chip, P: PIOExt, SM: StateMachineIndex, I: AnyPin, T> core::fmt::Debug
    for PioOutput<C, P, SM, I, T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The HAL's PIO and DMA types have no `Debug`.
        f.debug_struct("PioOutput").finish_non_exhaustive()
    }
}
