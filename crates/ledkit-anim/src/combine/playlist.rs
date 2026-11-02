//! [`Playlist`]: animations one after another, with transitions.

use core::time::Duration;

use ledkit_color::Rgb;

use crate::math::scale_u8;
use crate::time::millis;
use crate::{Animation, Frame, Position, Transition};

/// One step of a [`Playlist`]: an animation, how long it plays, and how it
/// comes in.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let rainbow = Rainbow::new();
/// let step = Entry::new(&rainbow, Duration::from_secs(10))
///     .transition(Transition::fade(Duration::from_secs(1)));
/// ```
#[derive(Copy, Clone)]
#[must_use]
pub struct Entry<'a> {
    animation: &'a dyn Animation,
    duration: u32,
    transition: Transition,
}

impl<'a> Entry<'a> {
    /// `animation`, playing for `duration`. It comes in with a cut unless you
    /// set a [`transition`](Entry::transition).
    pub const fn new(animation: &'a dyn Animation, duration: Duration) -> Self {
        Self {
            animation,
            duration: millis(duration),
            transition: Transition::cut(),
        }
    }

    /// How this entry comes in, replacing the one before it. The transition
    /// happens during the first part of this entry's time.
    pub const fn transition(mut self, transition: Transition) -> Self {
        self.transition = transition;
        self
    }
}

impl core::fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Entry")
            .field("duration", &self.duration)
            .field("transition", &self.transition)
            .finish_non_exhaustive()
    }
}

/// Animations played one after another, each for its own time, with
/// transitions between them. Loops forever unless you call
/// [`play_once`](Playlist::play_once).
///
/// Each animation starts its own clock at 0 when its turn comes, so it always
/// starts from its beginning. During a transition, the one going out keeps
/// running.
///
/// # Example
///
/// ```
/// use core::time::Duration;
/// use ledkit_anim::prelude::*;
///
/// let swirl = Swirl::new();
/// let rings = Rings::new();
/// let flames = Flames::new();
/// let fade = Transition::fade(Duration::from_secs(1));
///
/// let entries = [
///     Entry::new(&swirl, Duration::from_secs(8)),
///     Entry::new(&rings, Duration::from_secs(8)).transition(fade),
///     Entry::new(&flames, Duration::from_secs(8)).transition(fade),
/// ];
/// let show = Playlist::new(&entries);
///
/// let mut leds = [Rgb::BLACK; 30];
/// show.render(&mut leds, Time::from_millis(12_000)); // rings, 4 seconds in
/// ```
#[derive(Copy, Clone, Debug)]
#[must_use]
pub struct Playlist<'a> {
    entries: &'a [Entry<'a>],
    looping: bool,
}

impl<'a> Playlist<'a> {
    /// A playlist of `entries` that starts again from the first after the last.
    pub const fn new(entries: &'a [Entry<'a>]) -> Self {
        Self {
            entries,
            looping: true,
        }
    }

    /// Plays the entries once, then holds the last one's final frame.
    pub const fn play_once(mut self) -> Self {
        self.looping = false;
        self
    }

    /// How long one pass through all the entries takes, in milliseconds.
    ///
    /// ```
    /// use core::time::Duration;
    /// use ledkit_anim::prelude::*;
    ///
    /// let solid = Solid::new();
    /// let entries = [Entry::new(&solid, Duration::from_secs(3)); 2];
    /// assert_eq!(Playlist::new(&entries).total_millis(), 6000);
    /// ```
    pub fn total_millis(&self) -> u64 {
        self.entries.iter().map(|e| u64::from(e.duration)).sum()
    }
}

impl Animation for Playlist<'_> {
    fn color_at(&self, at: Position, frame: &Frame) -> Rgb {
        let (Some(first), Some(last)) = (self.entries.first(), self.entries.last()) else {
            return Rgb::BLACK;
        };
        let total = self.total_millis();
        if total == 0 {
            return first.animation.color_at(at, &frame.at_millis(0));
        }
        let t = u64::from(frame.millis);
        if !self.looping && t >= total {
            // Hold the last entry that was actually shown.
            let shown = self
                .entries
                .iter()
                .rev()
                .find(|e| e.duration > 0)
                .unwrap_or(last);
            let end = shown.duration.saturating_sub(1);
            return shown.animation.color_at(at, &frame.at_millis(end));
        }
        let (lap, mut into) = (t / total, t % total);
        // Find the entry playing now, and the one before it.
        let shown_last = self.entries.iter().rev().find(|e| e.duration > 0);
        let mut previous = if lap > 0 { shown_last } else { None };
        for entry in self.entries {
            let duration = u64::from(entry.duration);
            if into < duration {
                let local = into as u32;
                let now = entry.animation.color_at(at, &frame.at_millis(local));
                let length = entry.transition.length_millis().min(entry.duration);
                return match previous {
                    Some(before) if local < length => {
                        // The one going out is past its end, still running.
                        let its_time = before.duration.saturating_add(local);
                        let then = before.animation.color_at(at, &frame.at_millis(its_time));
                        entry
                            .transition
                            .mix(then, now, scale_u8(local, length), at, frame)
                    }
                    _ => now,
                };
            }
            into -= duration;
            // An entry with no time is never shown, so nothing fades out of it.
            if duration > 0 {
                previous = Some(entry);
            }
        }
        Rgb::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combine::then::tests::{time_shown, Clock};
    use crate::effects::test_support::{exercise, strip_at};
    use crate::Solid;

    const SECOND: Duration = Duration::from_secs(1);
    const RED: Solid = Solid::new().color(Rgb::RED);
    const BLUE: Solid = Solid::new().color(Rgb::BLUE);

    #[test]
    fn plays_in_turn_and_loops() {
        let entries = [Entry::new(&RED, SECOND), Entry::new(&Clock, SECOND)];
        let show = Playlist::new(&entries);
        assert_eq!(strip_at(&show, 0, 5, 500), Rgb::RED);
        assert_eq!(time_shown(strip_at(&show, 0, 5, 1500)), 500);
        assert_eq!(strip_at(&show, 0, 5, 2500), Rgb::RED);
        assert_eq!(time_shown(strip_at(&show, 0, 5, 3250)), 250);
    }

    #[test]
    fn fades_between_entries() {
        let fade = Transition::fade(Duration::from_millis(200));
        let entries = [
            Entry::new(&RED, SECOND),
            Entry::new(&BLUE, SECOND).transition(fade),
        ];
        let show = Playlist::new(&entries);
        let middle = strip_at(&show, 0, 5, 1100);
        assert!(middle.r > 50 && middle.b > 50, "{middle:?}");
        assert_eq!(strip_at(&show, 0, 5, 1200), Rgb::BLUE);
        // The very first pass starts with a cut: nothing came before.
        let looping = [
            Entry::new(&RED, SECOND).transition(fade),
            Entry::new(&BLUE, SECOND),
        ];
        let show = Playlist::new(&looping);
        assert_eq!(strip_at(&show, 0, 5, 50), Rgb::RED);
        let second_lap = strip_at(&show, 0, 5, 2100);
        assert!(second_lap.r > 50 && second_lap.b > 50, "{second_lap:?}");
    }

    #[test]
    fn zero_length_entries_are_skipped() {
        let green = Solid::new().color(Rgb::GREEN);
        let fade = Transition::fade(Duration::from_millis(500)).easing(crate::Easing::Linear);
        let entries = [
            Entry::new(&RED, SECOND),
            Entry::new(&green, Duration::ZERO),
            Entry::new(&BLUE, SECOND).transition(fade),
            Entry::new(&green, Duration::ZERO),
        ];
        let middle = strip_at(&Playlist::new(&entries), 0, 5, 1250);
        assert_eq!(middle.g, 0, "{middle:?}");
        assert_eq!(
            strip_at(&Playlist::new(&entries).play_once(), 0, 5, 9000),
            Rgb::BLUE
        );
    }

    #[test]
    fn play_once_holds_the_last_frame() {
        let entries = [Entry::new(&RED, SECOND), Entry::new(&Clock, SECOND)];
        let show = Playlist::new(&entries).play_once();
        assert_eq!(time_shown(strip_at(&show, 0, 5, 10_000)), 999);
    }

    #[test]
    fn edge_cases_do_not_panic() {
        let empty: [Entry; 0] = [];
        assert_eq!(strip_at(&Playlist::new(&empty), 0, 5, 100), Rgb::BLACK);
        exercise(&Playlist::new(&empty));
        let zero = [
            Entry::new(&RED, Duration::ZERO),
            Entry::new(&BLUE, Duration::ZERO),
        ];
        assert_eq!(strip_at(&Playlist::new(&zero), 0, 5, 100), Rgb::RED);
        exercise(&Playlist::new(&zero).play_once());
        let long_fade = Transition::fade(Duration::from_secs(10));
        let short = [
            Entry::new(&RED, SECOND).transition(long_fade),
            Entry::new(&BLUE, SECOND).transition(long_fade),
        ];
        exercise(&Playlist::new(&short));
        let one = [Entry::new(&Clock, SECOND).transition(long_fade)];
        exercise(&Playlist::new(&one));
    }
}
