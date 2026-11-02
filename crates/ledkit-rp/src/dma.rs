//! [`PioDmaOutput`]: a DMA channel feeds the state machine's FIFO.

use core::marker::PhantomData;

use ledkit_core::encode::fill_pio_words;
use ledkit_core::{wire, Chip, ClocklessTiming, Output, OutputAsync};

use crate::clock::{yield_now, FrameClock, MicrosecondClock};
use crate::hal::dma::single_buffer::{Config, Transfer};
use crate::hal::dma::{ReadTarget, SingleChannel};
use crate::hal::gpio::AnyPin;
use crate::hal::pio::{PIOExt, StateMachineIndex, Tx, UninitStateMachine, PIO};
use crate::program::{self, bytes_per_word, Machine};
use crate::Error;

/// How many 32-bit words of DMA buffer [`PioDmaOutput`] needs for `led_count` LEDs:
/// one per LED, whatever the chip's channel count.
///
/// This is a `const fn`, so it sizes an array. Using a clocked chip (a DotStar,
/// which needs a clock wire) is a build error.
///
/// # Example
///
/// ```ignore
/// use ledkit_core::Ws2812b;
/// use static_cell::ConstStaticCell;
///
/// const LED_COUNT: usize = 100;
/// static BUFFER: ConstStaticCell<[u32; ledkit_rp::buffer_len::<Ws2812b>(LED_COUNT)]> =
///     ConstStaticCell::new([0; ledkit_rp::buffer_len::<Ws2812b>(LED_COUNT)]);
/// ```
#[must_use]
pub const fn buffer_len<C: Chip>(led_count: usize) -> usize {
    // Clocked chips have no pulse timing; this fails the build for them.
    let _clockless = const { ClocklessTiming::of::<C>() };
    let _fits = const { bytes_per_word::<C>() };
    led_count
}

/// The words of one frame: the first `len` words of the buffer.
///
/// The DMA engine only takes a `ReadTarget`. A plain `&'static mut [u32]` would
/// always send the whole buffer, and a sub-slice of it could not be joined back
/// to the whole afterwards, so this hands the engine a prefix while keeping the
/// whole buffer.
#[derive(Debug)]
pub(crate) struct DmaFrame {
    words: &'static mut [u32],
    len: usize,
}

// SAFETY: `rx_address_count` describes `len` words starting at the start of
// `words`, and `len <= words.len()` always holds (it is only set from
// `fill_pio_words`, which never reports more words than fit). `words` is a
// `&'static mut`, so the memory stays valid for as long as the DMA engine might
// read it, and the `DmaFrame` is moved into the `Transfer` for the whole
// transfer, so nothing else can touch the words meanwhile.
#[allow(unsafe_code)]
unsafe impl ReadTarget for DmaFrame {
    type ReceivedWord = u32;

    fn rx_treq() -> Option<u8> {
        // Memory: let the state machine's FIFO pace the transfer.
        None
    }

    fn rx_address_count(&self) -> (u32, u32) {
        (self.words.as_ptr() as u32, self.len as u32)
    }

    fn rx_increment(&self) -> bool {
        true
    }
}

/// Where the DMA channel, the FIFO and the buffer are between and during frames.
enum State<CH: SingleChannel, P: PIOExt, SM: StateMachineIndex> {
    Idle {
        channel: CH,
        tx: Tx<(P, SM)>,
        frame: DmaFrame,
    },
    Sending(Transfer<CH, DmaFrame, Tx<(P, SM)>>),
    /// Only seen if a panic unwound in the middle of a send.
    Lost,
}

/// A chain of clockless LEDs of type `C` (for example [`ledkit_core::Ws2812b`]) on
/// one pin, driven by a PIO state machine that a **DMA channel** feeds.
///
/// The processor packs the frame into a buffer (one 32-bit word per LED), then
/// the DMA channel streams it into the state machine's FIFO as fast as the LEDs
/// take it. Interrupts and other work can't starve the FIFO mid-frame, and with
/// [`OutputAsync`] other tasks run while the frame goes out. This is the output
/// to use for long strips.
///
/// - `C` is your LED type.
/// - `P` and `SM` are the PIO block and state machine (from `PIO::split`).
/// - `I` is the data pin, already switched to that PIO block's function.
/// - `CH` is the DMA channel (from `pac.DMA.split`).
/// - `T` is the timer (see [`MicrosecondClock`]).
///
/// The buffer must live for the whole program (`&'static mut`), because the DMA
/// engine reads it in the background. `static_cell`'s `ConstStaticCell` makes one
/// on both Arm and RISC-V; size it with [`buffer_len`].
///
/// # Example
///
/// ```ignore
/// use hal::dma::DMAExt;
/// use ledkit_core::prelude::*;
/// use ledkit_rp::PioDmaOutput;
/// use static_cell::ConstStaticCell;
///
/// const LED_COUNT: usize = 100;
/// static BUFFER: ConstStaticCell<[u32; LED_COUNT]> = ConstStaticCell::new([0; LED_COUNT]);
///
/// let dma = pac.DMA.split(&mut pac.RESETS);
/// let output = PioDmaOutput::<Ws2812b, _, _, _, _, _>::new(
///     &mut pio, sm0, pin, dma.ch1, BUFFER.take(), sys_hz, timer,
/// )?;
/// let mut strip = Strip::<Ws2812b, LED_COUNT, _>::new(output);
/// strip.fill(Rgb::GREEN);
/// strip.show()?;
/// ```
pub struct PioDmaOutput<C: Chip, P: PIOExt, SM: StateMachineIndex, I: AnyPin, CH: SingleChannel, T>
{
    machine: Machine<P, SM, I>,
    state: State<CH, P, SM>,
    /// A frame was started and its end not yet recorded.
    draining: bool,
    frame_clock: FrameClock<T>,
    chip: PhantomData<C>,
}

impl<C, P, SM, I, CH, T> PioDmaOutput<C, P, SM, I, CH, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    CH: SingleChannel,
    T: MicrosecondClock,
{
    /// Installs ledkit's program on `pio`, starts state machine `sm` sending to
    /// `pin`, and sets `channel` up to feed it from `buffer`.
    ///
    /// `pin`, `sys_hz` and `timer` are as for
    /// [`PioOutput::new`](crate::PioOutput::new). `buffer` holds the frame: one
    /// word per LED (see [`buffer_len`]). A frame with more LEDs than that is
    /// refused with [`Error::BufferTooSmall`]; a shorter one sends only its own
    /// LEDs.
    ///
    /// # Errors
    ///
    /// [`Error::Timing`] if the chip can't be timed from `sys_hz`, and
    /// [`Error::NoProgramSpace`] if the PIO block is full.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let output = PioDmaOutput::<Ws2812b, _, _, _, _, _>::new(
    ///     &mut pio, sm0, pin, dma.ch1, BUFFER.take(), sys_hz, timer,
    /// )?;
    /// ```
    pub fn new(
        pio: &mut PIO<P>,
        sm: UninitStateMachine<(P, SM)>,
        pin: I,
        channel: CH,
        buffer: &'static mut [u32],
        sys_hz: u32,
        timer: T,
    ) -> Result<Self, Error> {
        let (machine, tx, frame_clock) =
            program::start::<C, P, SM, I, T>(pio, sm, pin, sys_hz, timer)?;
        Ok(Self {
            machine,
            state: State::Idle {
                channel,
                tx,
                frame: DmaFrame {
                    words: buffer,
                    len: 0,
                },
            },
            draining: false,
            frame_clock,
            chip: PhantomData,
        })
    }

    /// Waits for any frame still going out, stops the state machine, removes
    /// ledkit's program from `pio`, and gives back the state machine, the pin, the
    /// DMA channel and the buffer.
    ///
    /// # Errors
    ///
    /// [`Error::ChannelLost`] if a panic unwound in the middle of a send.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let (sm0, pin, ch1, buffer) = output.release(&mut pio)?;
    /// ```
    #[allow(clippy::type_complexity)] // the four parts it was made from
    pub fn release(
        mut self,
        pio: &mut PIO<P>,
    ) -> Result<(UninitStateMachine<(P, SM)>, I, CH, &'static mut [u32]), Error> {
        self.finish_blocking();
        match self.state {
            State::Idle { channel, tx, frame } => {
                let (sm, pin) = self.machine.release(pio, tx);
                Ok((sm, pin, channel, frame.words))
            }
            State::Sending(_) | State::Lost => Err(Error::ChannelLost),
        }
    }

    /// If a frame is going out (only after an async send was dropped half way),
    /// waits until its last bit is out.
    fn finish_blocking(&mut self) {
        if let State::Sending(transfer) = core::mem::replace(&mut self.state, State::Lost) {
            let (channel, frame, tx) = transfer.wait();
            self.state = State::Idle { channel, tx, frame };
        }
        if let State::Idle { tx, .. } = &self.state {
            if self.draining {
                while !tx.is_empty() {}
                self.frame_clock.finish_frame();
                self.draining = false;
            }
        }
    }

    /// The async twin of `finish_blocking`. Dropping it half way is safe: the next
    /// call picks up where it stopped.
    async fn finish_async(&mut self) {
        while let State::Sending(transfer) = &self.state {
            if transfer.is_done() {
                break;
            }
            yield_now().await;
        }
        if let State::Sending(transfer) = core::mem::replace(&mut self.state, State::Lost) {
            let (channel, frame, tx) = transfer.wait();
            self.state = State::Idle { channel, tx, frame };
        }
        if let State::Idle { tx, .. } = &self.state {
            if self.draining {
                while !tx.is_empty() {
                    yield_now().await;
                }
                self.frame_clock.finish_frame_async().await;
                self.draining = false;
            }
        }
    }

    /// Packs the wire bytes into the buffer. Returns the channel, FIFO and buffer
    /// ready to start, or `None` for an empty frame (nothing to send).
    fn prepare(
        &mut self,
        wire: impl Iterator<Item = u8>,
    ) -> Result<Option<(CH, Tx<(P, SM)>, DmaFrame)>, Error> {
        let State::Idle {
            channel,
            tx,
            mut frame,
        } = core::mem::replace(&mut self.state, State::Lost)
        else {
            return Err(Error::ChannelLost);
        };
        match fill_pio_words(wire, const { bytes_per_word::<C>() }, frame.words) {
            Ok(0) => {
                self.state = State::Idle { channel, tx, frame };
                Ok(None)
            }
            Ok(used) => {
                frame.len = used;
                Ok(Some((channel, tx, frame)))
            }
            Err(e) => {
                self.state = State::Idle { channel, tx, frame };
                Err(e.into())
            }
        }
    }

    /// Sends wire bytes, blocking until the frame is out.
    pub(crate) fn send_blocking(&mut self, wire: impl Iterator<Item = u8>) -> Result<(), Error> {
        self.finish_blocking();
        let Some((channel, tx, frame)) = self.prepare(wire)? else {
            return Ok(());
        };
        self.frame_clock.wait_for_gap();
        self.draining = true;
        self.state = State::Sending(Config::new(channel, frame, tx).start());
        self.finish_blocking();
        Ok(())
    }

    /// Sends wire bytes, yielding to other tasks until the frame is out.
    pub(crate) async fn send_async(&mut self, wire: impl Iterator<Item = u8>) -> Result<(), Error> {
        self.finish_async().await;
        let Some((channel, tx, frame)) = self.prepare(wire)? else {
            return Ok(());
        };
        self.frame_clock.wait_for_gap_async().await;
        self.draining = true;
        self.state = State::Sending(Config::new(channel, frame, tx).start());
        self.finish_async().await;
        Ok(())
    }
}

impl<C, P, SM, I, CH, T> Output<C> for PioDmaOutput<C, P, SM, I, CH, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    CH: SingleChannel,
    T: MicrosecondClock,
{
    type Error = Error;

    /// Packs the frame into the buffer, has the DMA channel send it, and returns
    /// when its last bit is out. If the previous frame ended less than the chip's
    /// reset time ago, it first waits out the rest.
    fn write<It>(&mut self, pixels: It) -> Result<(), Self::Error>
    where
        It: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.send_blocking(wire::clockless_bytes::<C, _>(pixels))
    }
}

impl<C, P, SM, I, CH, T> OutputAsync<C> for PioDmaOutput<C, P, SM, I, CH, T>
where
    C: Chip,
    P: PIOExt,
    SM: StateMachineIndex,
    I: AnyPin<Function = P::PinFunction>,
    CH: SingleChannel,
    T: MicrosecondClock,
{
    type Error = Error;

    /// Packs the frame into the buffer, has the DMA channel send it, and yields to
    /// other tasks until its last bit is out.
    ///
    /// If this future is dropped half way (for example inside `select`), the DMA
    /// channel still finishes the frame, and the next send waits for it first.
    async fn write<It>(&mut self, pixels: It) -> Result<(), Self::Error>
    where
        It: Iterator<Item = C::Pixel> + ExactSizeIterator,
    {
        self.send_async(wire::clockless_bytes::<C, _>(pixels)).await
    }
}

impl<C: Chip, P: PIOExt, SM: StateMachineIndex, I: AnyPin, CH: SingleChannel, T> core::fmt::Debug
    for PioDmaOutput<C, P, SM, I, CH, T>
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // The HAL's PIO and DMA types have no `Debug`.
        f.debug_struct("PioDmaOutput").finish_non_exhaustive()
    }
}
