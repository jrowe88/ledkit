//! [`Time`]: how long the animation has been running.

use core::time::Duration;

/// How long the animation has been running, in milliseconds.
///
/// Animations are drawn from the elapsed time, not by counting frames, so they
/// run at the right speed however fast your loop goes. Make one from a
/// [`Duration`] (what `std::time::Instant::elapsed` gives) or from a number of
/// milliseconds (handy with board clocks, which have their own duration types):
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::Time;
///
/// assert_eq!(Time::from(Duration::from_secs(2)).as_millis(), 2000);
/// assert_eq!(Time::from_millis(2000).as_millis(), 2000);
///
/// // On an ESP32 with esp-hal, for example:
/// // let now = Time::from_millis(start.elapsed().as_millis() as u32);
/// ```
///
/// It counts up to about 49 days and then starts again from 0. An effect whose
/// cycle doesn't divide evenly into that may jump once when that happens.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Time(u32);

impl Time {
    /// A time from a number of milliseconds.
    ///
    /// ```
    /// use ledkit_anim::Time;
    ///
    /// let half_a_second = Time::from_millis(500);
    /// ```
    pub const fn from_millis(millis: u32) -> Self {
        Self(millis)
    }

    /// The time in milliseconds.
    ///
    /// ```
    /// use ledkit_anim::Time;
    ///
    /// assert_eq!(Time::from_millis(500).as_millis(), 500);
    /// ```
    pub const fn as_millis(self) -> u32 {
        self.0
    }
}

impl From<Duration> for Time {
    /// Converts a duration, starting again from 0 every 2^32 milliseconds
    /// (about 49 days).
    fn from(d: Duration) -> Self {
        Self((d.as_millis() % (1u128 << 32)) as u32)
    }
}

/// A setting's duration in milliseconds, capped at `u32::MAX`.
pub(crate) const fn millis(d: Duration) -> u32 {
    let ms = d.as_millis();
    if ms > u32::MAX as u128 {
        u32::MAX
    } else {
        ms as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_from_duration_wraps() {
        let d = Duration::from_millis(u64::from(u32::MAX) + 5);
        assert_eq!(Time::from(d).as_millis(), 4);
    }

    #[test]
    fn setting_durations_saturate() {
        assert_eq!(millis(Duration::from_secs(1)), 1000);
        assert_eq!(millis(Duration::from_secs(u64::MAX)), u32::MAX);
    }
}
