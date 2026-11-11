//! Timing and power descriptions of LED chips.

use crate::Chip;

/// One timing window of a clockless LED signal, in nanoseconds.
///
/// `typical_ns` is the value ledkit aims for; `min_ns` and `max_ns` are the
/// limits the chip's datasheet allows.
///
/// # Example
///
/// ```
/// use ledkit_core::Phase;
///
/// let p = Phase::new(400, 220, 450);
/// assert!(p.contains(400));
/// assert!(!p.contains(451));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Phase {
    /// The ideal length, in nanoseconds.
    pub typical_ns: u32,
    /// The shortest length the chip accepts, in nanoseconds.
    pub min_ns: u32,
    /// The longest length the chip accepts, in nanoseconds.
    pub max_ns: u32,
}

impl Phase {
    /// Creates a phase from its typical, minimum and maximum lengths in nanoseconds.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Phase;
    ///
    /// // Aim for 400 ns; anything from 220 ns to 450 ns works.
    /// let p = Phase::new(400, 220, 450);
    /// assert_eq!(p.typical_ns, 400);
    /// ```
    #[must_use]
    pub const fn new(typical_ns: u32, min_ns: u32, max_ns: u32) -> Self {
        Self {
            typical_ns,
            min_ns,
            max_ns,
        }
    }

    /// Whether `ns` is inside the allowed window (both ends included).
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::Phase;
    ///
    /// let p = Phase::new(400, 220, 450);
    /// assert!(p.contains(220) && p.contains(450));
    /// assert!(!p.contains(451));
    /// ```
    #[must_use]
    pub const fn contains(&self, ns: u32) -> bool {
        ns >= self.min_ns && ns <= self.max_ns
    }
}

/// The wire timing of a one-wire ("clockless") LED chip such as the WS2812B.
///
/// A `0` bit is a short high pulse followed by a long low; a `1` bit is a long
/// high pulse followed by a short low. After the last bit the line stays low
/// for `reset_us` microseconds to latch the colors.
///
/// # Example
///
/// ```
/// use ledkit_core::{Chip, Protocol, Ws2812b};
///
/// if let Protocol::Clockless(t) = Ws2812b::PROTOCOL {
///     assert_eq!(t.reset_us, 280);
/// }
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ClocklessTiming {
    /// High time of a `0` bit.
    pub zero_high: Phase,
    /// Low time of a `0` bit.
    pub zero_low: Phase,
    /// High time of a `1` bit.
    pub one_high: Phase,
    /// Low time of a `1` bit.
    pub one_low: Phase,
    /// How long the line must stay low to latch, in microseconds.
    pub reset_us: u32,
}

/// How long a clockless chip needs the data line idle between frames.
///
/// RMT and PIO backends use this before each frame: remember when the last
/// frame ended, then wait [`ResetGap::remaining_us`] microseconds before
/// starting the next, however often `show()` is called. SPI backends do not
/// need it, because they send the reset as trailing zero bytes.
///
/// # Example
///
/// ```
/// use ledkit_core::{ResetGap, Ws2812b};
///
/// let gap = ResetGap::of::<Ws2812b>();
/// // 100 us have passed since the last frame ended, so wait 180 us more.
/// assert_eq!(gap.remaining_us(100), 180);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ResetGap {
    reset_us: u32,
}

impl ResetGap {
    /// Gets the reset gap of a clockless chip, for example `ResetGap::of::<Ws2812b>()`.
    ///
    /// Using a clocked chip such as the APA102 is a compile error when this
    /// is evaluated in a `const`, and a panic otherwise.
    ///
    /// # Panics
    ///
    /// Panics with "this chip is clocked, not clockless" if `C` is a clocked chip.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ResetGap, Ws2812b};
    ///
    /// const GAP: ResetGap = ResetGap::of::<Ws2812b>();
    /// assert_eq!(GAP.reset_us(), 280);
    /// ```
    ///
    /// A clocked chip does not compile:
    ///
    /// ```compile_fail
    /// use ledkit_core::{Apa102, ResetGap};
    ///
    /// const GAP: ResetGap = ResetGap::of::<Apa102>();
    /// ```
    #[must_use]
    pub const fn of<C: Chip>() -> Self {
        Self {
            reset_us: ClocklessTiming::of::<C>().reset_us,
        }
    }

    /// The full reset gap, in microseconds.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ResetGap, Ws2812b};
    ///
    /// assert_eq!(ResetGap::of::<Ws2812b>().reset_us(), 280);
    /// ```
    #[must_use]
    pub const fn reset_us(&self) -> u32 {
        self.reset_us
    }

    /// How many microseconds must still pass before the next frame may
    /// start, given that `elapsed_us` have passed since the last one ended.
    /// It is zero once the gap is over.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{ResetGap, Ws2812b};
    ///
    /// let gap = ResetGap::of::<Ws2812b>();
    /// assert_eq!(gap.remaining_us(0), 280);
    /// assert_eq!(gap.remaining_us(1_000), 0);
    /// ```
    #[must_use]
    pub const fn remaining_us(&self, elapsed_us: u64) -> u32 {
        if elapsed_us >= self.reset_us as u64 {
            0
        } else {
            // elapsed < reset_us <= u32::MAX, so the difference fits.
            (self.reset_us as u64 - elapsed_us) as u32
        }
    }
}

/// The data format of chips that use a separate clock wire.
///
/// Both formats share the same start frame and LED data. They differ only in
/// the end frame, which is what makes the last LEDs show their colors. See
/// [`wire::clocked_frame`](crate::wire::clocked_frame) for the details.
///
/// # Example
///
/// ```
/// use ledkit_core::ClockedFormat;
///
/// assert_eq!(ClockedFormat::Apa102.end_frame_len(64), 4);
/// assert_eq!(ClockedFormat::Sk9822.end_frame_len(64), 4 + 4);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClockedFormat {
    /// The APA102 frame format: the end frame is `ceil(n / 64)` black LED
    /// frames (`0xFF 00 00 00`), so LEDs past the end of the frame stay dark.
    Apa102,
    /// The SK9822 frame format: the end frame is a 4-byte reset frame of
    /// `0x00`, then `ceil(n / 16)` more `0x00` bytes. The SK9822 needs the
    /// 32-zero-bit reset to latch its colors. Not yet verified on hardware
    /// in ledkit.
    Sk9822,
}

impl ClockedFormat {
    /// The format a chip uses. A chip that is not clocked gets
    /// [`Apa102`](ClockedFormat::Apa102), so using the wrong function never
    /// panics.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::{Apa102, ClockedFormat, Sk9822};
    ///
    /// assert_eq!(ClockedFormat::of::<Apa102>(), ClockedFormat::Apa102);
    /// assert_eq!(ClockedFormat::of::<Sk9822>(), ClockedFormat::Sk9822);
    /// ```
    #[must_use]
    pub const fn of<C: crate::Chip>() -> Self {
        match C::PROTOCOL {
            Protocol::Clocked(format) => format,
            _ => ClockedFormat::Apa102,
        }
    }

    /// How many bytes the end frame has for `led_count` LEDs.
    ///
    /// - APA102: one 4-byte black LED frame for every 64 LEDs (rounded up).
    /// - SK9822: 4 reset bytes plus `ceil(n / 16)` bytes.
    ///
    /// Either way that is at least `n / 2` bits, the extra clock edges the
    /// last LEDs need.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::ClockedFormat;
    ///
    /// assert_eq!(ClockedFormat::Apa102.end_frame_len(17), 4);
    /// assert_eq!(ClockedFormat::Apa102.end_frame_len(65), 8);
    /// assert_eq!(ClockedFormat::Sk9822.end_frame_len(17), 6);
    /// ```
    #[must_use]
    pub const fn end_frame_len(self, led_count: usize) -> usize {
        match self {
            ClockedFormat::Apa102 => 4 * led_count.div_ceil(64),
            ClockedFormat::Sk9822 => 4 + led_count.div_ceil(16),
        }
    }

    /// The byte at `index` (counting from 0) of the end frame.
    ///
    /// - APA102: black LED frames, `0xFF 00 00 00` over and over, so `0xFF`
    ///   at every fourth byte and `0x00` in between.
    /// - SK9822: always `0x00`.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::ClockedFormat;
    ///
    /// let apa102: Vec<u8> = (0..8).map(|i| ClockedFormat::Apa102.end_frame_byte_at(i)).collect();
    /// assert_eq!(apa102, [0xFF, 0, 0, 0, 0xFF, 0, 0, 0]);
    /// assert_eq!(ClockedFormat::Sk9822.end_frame_byte_at(0), 0x00);
    /// ```
    #[must_use]
    pub const fn end_frame_byte_at(self, index: usize) -> u8 {
        match self {
            // The level byte of a black LED, then its three zero colors.
            ClockedFormat::Apa102 if index % 4 == 0 => 0xFF,
            ClockedFormat::Apa102 | ClockedFormat::Sk9822 => 0x00,
        }
    }

    /// The first byte of the end frame: `0xFF` for the APA102, `0x00` for
    /// the SK9822.
    ///
    /// Up to ledkit 0.1.0 every byte of the end frame had this value. The
    /// APA102 end frame is now made of black LED frames, so its bytes differ;
    /// use [`end_frame_byte_at`](Self::end_frame_byte_at).
    ///
    /// # Example
    ///
    /// ```
    /// # #![allow(deprecated)]
    /// use ledkit_core::ClockedFormat;
    ///
    /// assert_eq!(ClockedFormat::Apa102.end_frame_byte(), 0xFF);
    /// assert_eq!(ClockedFormat::Sk9822.end_frame_byte(), 0x00);
    /// ```
    #[deprecated(
        note = "the APA102 end frame is no longer one repeated byte; use `end_frame_byte_at`"
    )]
    #[must_use]
    pub const fn end_frame_byte(self) -> u8 {
        self.end_frame_byte_at(0)
    }
}

/// How a chip is talked to: one data wire with strict timing, or data plus clock.
///
/// # Example
///
/// ```
/// use ledkit_core::{Apa102, Chip, ClockedFormat, Protocol};
///
/// assert_eq!(Apa102::PROTOCOL, Protocol::Clocked(ClockedFormat::Apa102));
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Protocol {
    /// One data wire, timing carries the bits (NeoPixels).
    Clockless(ClocklessTiming),
    /// A data wire and a clock wire (DotStars).
    Clocked(ClockedFormat),
}

/// A rough model of how much current a chip draws.
///
/// # Example
///
/// ```
/// use ledkit_core::PowerModel;
///
/// assert_eq!(PowerModel::TYPICAL_5V.channel_full_microamps, 20_000);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PowerModel {
    /// Current drawn by one color channel at full brightness, in microamps.
    pub channel_full_microamps: u32,
    /// Current drawn by an LED that is switched off, in microamps.
    pub idle_microamps: u32,
}

impl PowerModel {
    /// A typical 5 V LED: 20 mA per channel at full brightness, 1 mA idle.
    ///
    /// # Example
    ///
    /// ```
    /// use ledkit_core::power::estimate_microamps;
    /// use ledkit_core::{PowerModel, Rgb};
    ///
    /// // One red LED: 20 mA for the red channel plus 1 mA idle.
    /// assert_eq!(estimate_microamps(&[Rgb::RED], PowerModel::TYPICAL_5V), 21_000);
    /// ```
    pub const TYPICAL_5V: PowerModel = PowerModel {
        channel_full_microamps: 20_000,
        idle_microamps: 1_000,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Chip, Ws2812b};

    #[test]
    fn phase_window_includes_both_edges_only() {
        let p = Phase::new(400, 220, 450);
        assert!(p.contains(220) && p.contains(450));
        assert!(!p.contains(219) && !p.contains(451));
    }

    #[test]
    fn reset_gap_waits_only_for_what_is_left() {
        let gap = ResetGap::of::<Ws2812b>();
        assert_eq!(gap.reset_us(), 280);
        assert_eq!(gap.remaining_us(0), 280);
        assert_eq!(gap.remaining_us(100), 180);
        assert_eq!(gap.remaining_us(280), 0);
        assert_eq!(gap.remaining_us(u64::MAX), 0);
    }

    #[test]
    fn default_power_model() {
        assert_eq!(Ws2812b::POWER, PowerModel::TYPICAL_5V);
    }
}
