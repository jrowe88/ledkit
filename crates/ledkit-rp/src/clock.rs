//! Time, for the reset gap between frames.

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use ledkit_core::ResetGap;

use crate::hal;

/// Something that tells the time in microseconds, counting up from boot.
///
/// ledkit uses it to keep frames at least the chip's reset time apart. It is
/// implemented for the HAL's `Timer`, which is `Copy`, so pass your `timer` and
/// keep using it yourself:
///
/// - `rp2040_hal::Timer` (feature `rp2040`);
/// - `rp235x_hal::Timer<_>`, from `Timer::new_timer0` or `new_timer1` (feature
///   `rp235x`).
///
/// # Example
///
/// ```ignore
/// use ledkit_rp::MicrosecondClock;
///
/// let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
/// let started = timer.now_us();
/// ```
pub trait MicrosecondClock {
    /// Microseconds since the timer started.
    fn now_us(&self) -> u64;
}

#[cfg(feature = "rp2040")]
impl MicrosecondClock for hal::Timer {
    fn now_us(&self) -> u64 {
        self.get_counter().ticks()
    }
}

#[cfg(all(feature = "rp235x", not(feature = "rp2040")))]
impl<D: hal::timer::TimerDevice> MicrosecondClock for hal::Timer<D> {
    fn now_us(&self) -> u64 {
        self.get_counter().ticks()
    }
}

impl<T: MicrosecondClock> MicrosecondClock for &T {
    fn now_us(&self) -> u64 {
        (**self).now_us()
    }
}

/// When the last frame ended, and how long the next one must wait.
#[derive(Debug)]
pub(crate) struct FrameClock<T> {
    clock: T,
    gap: ResetGap,
    /// Microseconds from "the FIFO is empty" to "the last bit is out": the word
    /// still in the shift register plus the bit in progress, rounded up.
    drain_us: u64,
    /// When the last frame ended (at first, when the pin was driven low).
    last_end_us: u64,
}

impl<T: MicrosecondClock> FrameClock<T> {
    /// Call right after the pin has been driven low. That moment counts as the end
    /// of a frame, so the first frame also waits a whole reset gap: the pin may
    /// have been high, or carrying another frame, just before.
    pub(crate) fn new(clock: T, gap: ResetGap, drain_us: u64) -> Self {
        let last_end_us = clock.now_us();
        Self {
            clock,
            gap,
            drain_us,
            last_end_us,
        }
    }

    /// Microseconds before the next frame may start.
    fn remaining_us(&self) -> u32 {
        self.gap
            .remaining_us(self.clock.now_us().saturating_sub(self.last_end_us))
    }

    /// Waits until the reset gap since the last frame is over.
    pub(crate) fn wait_for_gap(&self) {
        while self.remaining_us() > 0 {}
    }

    /// The async twin of [`wait_for_gap`](Self::wait_for_gap): yields to other
    /// tasks between checks.
    pub(crate) async fn wait_for_gap_async(&self) {
        while self.remaining_us() > 0 {
            yield_now().await;
        }
    }

    /// Call when the FIFO has just run empty: waits until the last bit is out
    /// and records that moment as the end of the frame.
    pub(crate) fn finish_frame(&mut self) {
        let end = self.clock.now_us() + self.drain_us;
        while self.clock.now_us() < end {}
        self.last_end_us = end;
    }

    /// The async twin of [`finish_frame`](Self::finish_frame).
    pub(crate) async fn finish_frame_async(&mut self) {
        let end = self.clock.now_us() + self.drain_us;
        while self.clock.now_us() < end {
            yield_now().await;
        }
        self.last_end_us = end;
    }
}

/// How long the state machine may still be sending once its FIFO is empty, in
/// microseconds: one word of `bits_per_word` bits in the shift register plus the
/// bit in progress, plus one for the timer's whole-microsecond reading.
pub(crate) fn drain_us(bits_per_word: u32, period_ticks: u32, divider: u32, sys_hz: u32) -> u64 {
    let cycles = u64::from(bits_per_word + 1) * u64::from(period_ticks) * u64::from(divider);
    (cycles * 1_000_000).div_ceil(u64::from(sys_hz.max(1))) + 1
}

/// Lets other async tasks run once, then carries on.
///
/// rp-hal has no interrupt-driven wakers for the PIO or DMA, so the async
/// outputs wait by yielding: the task asks to be polled again straight away.
pub(crate) fn yield_now() -> YieldNow {
    YieldNow { yielded: false }
}

/// The future of [`yield_now`].
pub(crate) struct YieldNow {
    yielded: bool,
}

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
