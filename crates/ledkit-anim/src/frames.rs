//! [`Clock`] and the frame loops [`Frames`] and [`FramesAsync`]: "give me the time,
//! about N times a second".

use crate::Time;

/// Anything that can say how long it has been running, as a [`Time`].
///
/// Board crates provide one (for example `ledkit::esp::Clock`), and any closure that
/// returns a [`Time`] is a clock too, so a hand-written clock is one line:
///
/// ```
/// use ledkit_anim::{Clock, Time};
///
/// // On an ESP32 with esp-hal, for example:
/// // let start = esp_hal::time::Instant::now();
/// // let clock = || Time::from_millis(start.elapsed().as_millis() as u32);
/// let clock = || Time::from_millis(1234);
/// assert_eq!(clock.now().as_millis(), 1234);
/// ```
pub trait Clock {
    /// How long the clock has been running.
    ///
    /// ```
    /// use ledkit_anim::{Clock, Time};
    ///
    /// let clock = || Time::from_millis(5);
    /// assert_eq!(clock.now(), Time::from_millis(5));
    /// ```
    fn now(&self) -> Time;
}

impl<F: Fn() -> Time> Clock for F {
    fn now(&self) -> Time {
        self()
    }
}

/// The frame rate [`Frames`] and [`FramesAsync`] start with: 60 frames a second.
pub const DEFAULT_FRAMES_PER_SECOND: u16 = 60;

/// When the frames are due. Shared by the blocking and the async loop.
#[derive(Copy, Clone, Debug)]
struct Schedule {
    /// Whole milliseconds per frame.
    period_ms: u32,
    /// What's left over from `1000 / fps`, handed out one millisecond at a time so the
    /// average rate is exact (60 fps is 16, 17, 17, 16, 17, 17, ... ms).
    remainder: u32,
    fps: u32,
    /// The remainder collected so far.
    carry: u32,
    /// The clock reading at the first frame.
    start: u32,
    /// When the next frame is due, in milliseconds after `start`.
    next_due: u32,
    started: bool,
}

impl Schedule {
    const fn new(fps: u16) -> Self {
        let fps = if fps == 0 {
            1
        } else if fps > 1000 {
            1000
        } else {
            fps as u32
        };
        Self {
            period_ms: 1000 / fps,
            remainder: 1000 % fps,
            fps,
            carry: 0,
            start: 0,
            next_due: 0,
            started: false,
        }
    }

    /// How long to wait, in milliseconds, before the next frame is due (0: draw now).
    fn wait_before(&mut self, now: u32) -> u32 {
        if !self.started {
            self.started = true;
            self.start = now;
            self.next_due = 0;
            self.carry = 0;
            return 0;
        }
        let elapsed = now.wrapping_sub(self.start);
        // Signed on purpose: wrapping subtraction read as signed gives "how late" even
        // across the clock wrapping, as long as the gap is under 24 days.
        #[allow(clippy::cast_possible_wrap)]
        let late = elapsed.wrapping_sub(self.next_due) as i32;
        if late < 0 {
            return late.unsigned_abs();
        }
        // More than a whole frame behind (drawing took too long, or the program did
        // something else for a while): skip the missed frames instead of rushing
        // through them.
        if late.unsigned_abs() >= self.period_ms.max(1) {
            self.next_due = elapsed;
        }
        0
    }

    /// The frame's time, and schedules the one after it.
    fn frame_at(&mut self, now: u32) -> Time {
        let elapsed = now.wrapping_sub(self.start);
        self.carry += self.remainder;
        let mut period = self.period_ms;
        if self.carry >= self.fps {
            self.carry -= self.fps;
            period += 1;
        }
        self.next_due = self.next_due.wrapping_add(period);
        Time::from_millis(elapsed)
    }
}

/// A steady frame loop: each [`next_frame`](Self::next_frame) waits until the next
/// frame is due, then gives the time since the first frame.
///
/// It waits until a *deadline*, not for a fixed time, so the frame rate stays steady
/// whether drawing a frame takes 1 ms or 10. If a frame takes longer than a whole
/// frame period, the missed frames are skipped rather than rushed.
///
/// Give it a [`Clock`] and any `embedded-hal` delay (on esp-hal, `esp_hal::delay::Delay`).
///
/// ```
/// use ledkit_anim::{Frames, Time};
/// # struct Delay;
/// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
/// # let delay = Delay;
///
/// let clock = || Time::from_millis(0); // a real board clock here
/// let mut frames = Frames::new(clock, delay).per_second(30);
///
/// let now = frames.next_frame(); // the first frame comes at once, at time 0
/// assert_eq!(now, Time::from_millis(0));
/// // In a program:
/// // loop {
/// //     let now = frames.next_frame();
/// //     rainbow.render(&mut strip, now);
/// //     strip.show()?;
/// // }
/// ```
#[derive(Debug)]
pub struct Frames<C, D> {
    clock: C,
    delay: D,
    schedule: Schedule,
}

impl<C, D> Frames<C, D> {
    /// A frame loop at 60 frames a second; change it with
    /// [`per_second`](Self::per_second).
    ///
    /// ```
    /// use ledkit_anim::{Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = Frames::new(|| Time::from_millis(0), Delay);
    /// ```
    pub const fn new(clock: C, delay: D) -> Self {
        Self {
            clock,
            delay,
            schedule: Schedule::new(DEFAULT_FRAMES_PER_SECOND),
        }
    }

    /// How many frames a second, from 1 to 1000 (other values are clamped to that).
    ///
    /// LEDs look smooth from about 30 frames a second; a long strip may not manage more,
    /// because sending a frame takes about 30 µs per NeoPixel.
    ///
    /// ```
    /// use ledkit_anim::{Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = Frames::new(|| Time::from_millis(0), Delay).per_second(100);
    /// ```
    #[must_use]
    pub const fn per_second(mut self, frames_per_second: u16) -> Self {
        self.schedule = Schedule::new(frames_per_second);
        self
    }

    /// Starts the time again from 0 at the next frame, for example when a new scene
    /// starts.
    ///
    /// ```
    /// use ledkit_anim::{Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = Frames::new(|| Time::from_millis(500), Delay);
    /// frames.restart();
    /// assert_eq!(frames.next_frame(), Time::from_millis(0));
    /// ```
    pub fn restart(&mut self) {
        self.schedule.started = false;
    }

    /// The clock, for example to time something else against it.
    ///
    /// ```
    /// use ledkit_anim::{Clock, Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = Frames::new(|| Time::from_millis(7), Delay);
    /// assert_eq!(frames.clock().now(), Time::from_millis(7));
    /// ```
    pub const fn clock(&self) -> &C {
        &self.clock
    }

    /// The delay, for example for a pause between scenes.
    ///
    /// ```
    /// use embedded_hal::delay::DelayNs;
    /// use ledkit_anim::{Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = Frames::new(|| Time::from_millis(0), Delay);
    /// frames.delay().delay_ms(1000);
    /// ```
    pub fn delay(&mut self) -> &mut D {
        &mut self.delay
    }
}

impl<C: Clock, D: embedded_hal::delay::DelayNs> Frames<C, D> {
    /// Waits until the next frame is due, then gives the time since the first frame
    /// (or since [`restart`](Self::restart)). The first frame comes at once, at time 0.
    ///
    /// ```
    /// use ledkit_anim::{Frames, Time};
    /// # struct Delay;
    /// # impl embedded_hal::delay::DelayNs for Delay { fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = Frames::new(|| Time::from_millis(0), Delay);
    /// assert_eq!(frames.next_frame(), Time::from_millis(0));
    /// ```
    pub fn next_frame(&mut self) -> Time {
        let wait = self.schedule.wait_before(self.clock.now().as_millis());
        if wait > 0 {
            self.delay.delay_ms(wait);
        }
        self.schedule.frame_at(self.clock.now().as_millis())
    }
}

/// [`Frames`] for async code (Embassy and friends): `frames.next_frame().await`.
///
/// Give it a [`Clock`] and any `embedded-hal-async` delay (on esp-hal, an async
/// `OneShotTimer`; with Embassy, `embassy_time::Delay`).
///
/// ```
/// use ledkit_anim::{FramesAsync, Time};
/// # struct Delay;
/// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
///
/// let mut frames = FramesAsync::new(|| Time::from_millis(0), Delay).per_second(30);
/// # embassy_futures::block_on(async {
/// let now = frames.next_frame().await;
/// assert_eq!(now, Time::from_millis(0));
/// # });
/// ```
#[derive(Debug)]
pub struct FramesAsync<C, D> {
    clock: C,
    delay: D,
    schedule: Schedule,
}

impl<C, D> FramesAsync<C, D> {
    /// An async frame loop at 60 frames a second; change it with
    /// [`per_second`](Self::per_second).
    ///
    /// ```
    /// use ledkit_anim::{FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = FramesAsync::new(|| Time::from_millis(0), Delay);
    /// ```
    pub const fn new(clock: C, delay: D) -> Self {
        Self {
            clock,
            delay,
            schedule: Schedule::new(DEFAULT_FRAMES_PER_SECOND),
        }
    }

    /// How many frames a second, from 1 to 1000 (other values are clamped to that).
    ///
    /// ```
    /// use ledkit_anim::{FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = FramesAsync::new(|| Time::from_millis(0), Delay).per_second(50);
    /// ```
    #[must_use]
    pub const fn per_second(mut self, frames_per_second: u16) -> Self {
        self.schedule = Schedule::new(frames_per_second);
        self
    }

    /// Starts the time again from 0 at the next frame.
    ///
    /// ```
    /// use ledkit_anim::{FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = FramesAsync::new(|| Time::from_millis(500), Delay);
    /// frames.restart();
    /// ```
    pub fn restart(&mut self) {
        self.schedule.started = false;
    }

    /// The clock.
    ///
    /// ```
    /// use ledkit_anim::{Clock, FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let frames = FramesAsync::new(|| Time::from_millis(7), Delay);
    /// assert_eq!(frames.clock().now(), Time::from_millis(7));
    /// ```
    pub const fn clock(&self) -> &C {
        &self.clock
    }

    /// The delay, for example for a pause between scenes.
    ///
    /// ```
    /// use ledkit_anim::{FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = FramesAsync::new(|| Time::from_millis(0), Delay);
    /// let delay = frames.delay();
    /// ```
    pub fn delay(&mut self) -> &mut D {
        &mut self.delay
    }
}

impl<C: Clock, D: embedded_hal_async::delay::DelayNs> FramesAsync<C, D> {
    /// Waits until the next frame is due, then gives the time since the first frame
    /// (or since [`restart`](Self::restart)). The first frame comes at once, at time 0.
    ///
    /// ```
    /// use ledkit_anim::{FramesAsync, Time};
    /// # struct Delay;
    /// # impl embedded_hal_async::delay::DelayNs for Delay { async fn delay_ns(&mut self, _: u32) {} }
    ///
    /// let mut frames = FramesAsync::new(|| Time::from_millis(0), Delay);
    /// # embassy_futures::block_on(async {
    /// assert_eq!(frames.next_frame().await, Time::from_millis(0));
    /// # });
    /// ```
    pub async fn next_frame(&mut self) -> Time {
        let wait = self.schedule.wait_before(self.clock.now().as_millis());
        if wait > 0 {
            self.delay.delay_ms(wait).await;
        }
        self.schedule.frame_at(self.clock.now().as_millis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cell::Cell;

    /// A delay that moves a fake clock forward instead of waiting.
    struct FakeDelay<'a>(&'a Cell<u32>);

    impl embedded_hal::delay::DelayNs for FakeDelay<'_> {
        fn delay_ns(&mut self, ns: u32) {
            self.0.set(self.0.get().wrapping_add(ns / 1_000_000));
        }
        fn delay_ms(&mut self, ms: u32) {
            self.0.set(self.0.get().wrapping_add(ms));
        }
    }

    impl embedded_hal_async::delay::DelayNs for FakeDelay<'_> {
        async fn delay_ns(&mut self, ns: u32) {
            embedded_hal::delay::DelayNs::delay_ns(self, ns);
        }
        async fn delay_ms(&mut self, ms: u32) {
            embedded_hal::delay::DelayNs::delay_ms(self, ms);
        }
    }

    fn times(fps: u16, count: usize, start: u32, work_ms: u32) -> std::vec::Vec<u32> {
        let now = Cell::new(start);
        let mut frames =
            Frames::new(|| Time::from_millis(now.get()), FakeDelay(&now)).per_second(fps);
        (0..count)
            .map(|_| {
                let t = frames.next_frame().as_millis();
                now.set(now.get().wrapping_add(work_ms));
                t
            })
            .collect()
    }

    #[test]
    fn sixty_frames_a_second_averages_exactly() {
        let t = times(60, 61, 1000, 0);
        assert_eq!(&t[..4], &[0, 16, 33, 50]);
        assert_eq!(t[60], 1000);
    }

    #[test]
    fn drawing_time_does_not_slow_the_rate() {
        // 5 ms of drawing each frame, at 50 fps (20 ms): still a frame every 20 ms.
        assert_eq!(times(50, 4, 0, 5), [0, 20, 40, 60]);
    }

    #[test]
    fn missed_frames_are_skipped_not_rushed() {
        // 45 ms of drawing at 50 fps: each frame is late by more than a period, so the
        // next one comes at once and the schedule starts over from there.
        assert_eq!(times(50, 4, 0, 45), [0, 45, 90, 135]);
    }

    #[test]
    fn a_little_late_catches_up() {
        // 25 ms of work at 50 fps is 5 ms late: no wait, and the deadline stays put.
        let t = times(50, 3, 0, 25);
        assert_eq!(t, [0, 25, 50]);
    }

    #[test]
    fn survives_the_clock_wrapping() {
        let t = times(10, 4, u32::MAX - 150, 0);
        assert_eq!(t, [0, 100, 200, 300]);
    }

    #[test]
    fn restart_starts_from_zero() {
        let now = Cell::new(0);
        let mut frames = Frames::new(|| Time::from_millis(now.get()), FakeDelay(&now));
        frames.next_frame();
        frames.next_frame();
        frames.restart();
        assert_eq!(frames.next_frame(), Time::from_millis(0));
        assert_eq!(frames.next_frame(), Time::from_millis(16));
    }

    #[test]
    fn frame_rates_are_clamped() {
        assert_eq!(times(0, 2, 0, 0), [0, 1000]);
        assert_eq!(times(5000, 3, 0, 0), [0, 1, 2]);
    }

    #[test]
    fn async_frames_match_blocking_ones() {
        let now = Cell::new(0);
        let mut frames =
            FramesAsync::new(|| Time::from_millis(now.get()), FakeDelay(&now)).per_second(50);
        let t: std::vec::Vec<u32> = (0..3)
            .map(|_| embassy_futures::block_on(frames.next_frame()).as_millis())
            .collect();
        assert_eq!(t, [0, 20, 40]);
    }
}
