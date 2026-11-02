//! [`Ticker`]: a list of short messages scrolling through a box, one after
//! another.

use ledkit_anim::{Surface, Time};

use crate::style::Style;
use crate::{Font, Paint, UpDown};

/// How many messages one call to `render` may move past when it has been a
/// long while since the last call. Past that, the message it got to starts
/// again from the beginning.
const MOST_SKIPS: u32 = 64;

/// Messages scrolling through a box on your panel, one after another, like
/// the news ticker at the bottom of a TV screen.
///
/// Each message comes in from the right edge, scrolls left, and leaves
/// off the left edge; then the box stays blank for a moment (the
/// [`gap`](Ticker::gap)) and the next message comes in. After the last
/// message it starts again with the first. A message short enough to fit in
/// the box doesn't scroll: it stays still in the middle for 3 seconds (the
/// [`hold`](Ticker::hold)), then the gap, then the next.
///
/// Give it the messages with [`set_messages`](Ticker::set_messages), and call
/// [`render`](Ticker::render) with the time each time round your loop. The
/// ticker works out from the time where the text is and when to move on to
/// the next message.
///
/// The ticker keeps its own copy of the messages, up to 256 bytes in all
/// (or `LEN`, see [`with_capacity`](Ticker::with_capacity)). Each message
/// takes one byte per character plus one more to mark where it ends.
///
/// ```
/// use ledkit_layout::{Canvas, MatrixLayout};
/// use ledkit_text::prelude::*;
///
/// let mut pixels = [Rgb::BLACK; 32 * 8];
/// let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(32, 8)).unwrap();
///
/// let mut ticker = Ticker::new(Font::X11_5X7).letters(Rgb::YELLOW).speed(12);
/// ticker.set_messages(&["12:34", "Rain at 4pm", "Standup 9:30"]);
/// // In your loop:
/// ticker.render(&mut panel, Time::from_millis(0));
/// ```
#[derive(Clone)]
#[must_use]
pub struct Ticker<const LEN: usize = 256> {
    style: Style,
    /// False until `at` is called: then the box is the whole surface.
    placed: bool,
    up_down: UpDown,
    /// LEDs per second.
    speed: u16,
    /// Blank LEDs of travel after each message; `None` for the box's width.
    gap: Option<u16>,
    /// Milliseconds a message that fits stays still.
    hold: u32,
    /// The messages being shown.
    list: List<LEN>,
    /// A list waiting for the end of the message being shown.
    pending: List<LEN>,
    has_pending: bool,
    /// Where the message being shown starts in `list`.
    current: usize,
    /// When the message being shown started, in milliseconds; `None` until
    /// the first `render` after the list was set.
    started: Option<u32>,
}

impl Ticker {
    /// A ticker with room for 256 bytes of messages, white letters,
    /// scrolling 20 LEDs a second through the middle of its box. Until you
    /// call [`at`](Ticker::at), the box is the whole surface.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let ticker = Ticker::new(Font::X11_5X7);
    /// ```
    pub const fn new(font: Font) -> Ticker {
        Ticker::with_capacity(font)
    }
}

impl<const LEN: usize> Ticker<LEN> {
    /// A ticker with room for `LEN` bytes of messages. Each message takes one
    /// byte per character plus one, so `["Hi", "Bye"]` takes 3 + 4 = 7. Use it
    /// when 256 is more than you need (to save memory) or not enough.
    ///
    /// The ticker keeps two lists this size (the one showing and a new one
    /// waiting to start), so it uses about `2 * LEN` bytes of memory.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let small = Ticker::<64>::with_capacity(Font::X11_5X7);
    /// ```
    pub const fn with_capacity(font: Font) -> Self {
        Self {
            style: Style::new(font),
            placed: false,
            up_down: UpDown::Middle,
            speed: 20,
            gap: None,
            hold: 3000,
            list: List::EMPTY,
            pending: List::EMPTY,
            has_pending: false,
            current: 0,
            started: None,
        }
    }

    /// Puts the box on the panel: its top-left LED at (`left`, `top`), and
    /// how many LEDs wide and tall it is. The box may hang off the edge of the
    /// panel; only the LEDs that exist are drawn.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // The bottom 8 rows of a 32x16 panel.
    /// let ticker = Ticker::new(Font::X11_5X7).at(0, 8, 32, 8);
    /// ```
    pub const fn at(mut self, left: i32, top: i32, width: u16, height: u16) -> Self {
        self.style.left = left;
        self.style.top = top;
        self.style.width = width;
        self.style.height = height;
        self.placed = true;
        self
    }

    /// Where the text sits up and down in the box. In the middle unless you
    /// say otherwise. (Across, the text scrolls.)
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let ticker = Ticker::new(Font::X11_5X7).up_down(UpDown::Bottom);
    /// ```
    pub const fn up_down(mut self, up_down: UpDown) -> Self {
        self.up_down = up_down;
        self
    }

    /// What colours the letters: an `Rgb`, or any effect. White unless you
    /// say otherwise.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let red = Ticker::new(Font::X11_5X7).letters(Rgb::RED);
    /// let rainbow = Ticker::new(Font::X11_5X7).letters(Rainbow::new());
    /// ```
    pub fn letters(mut self, paint: impl Into<Paint>) -> Self {
        self.style.letters = paint.into();
        self
    }

    /// Draws a line one LED wide around the letters, in this colour or
    /// effect. Outlined letters usually look best with a gap of 2 between
    /// them ([`Font::spacing`]).
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let font = Font::X11_5X7.spacing(2);
    /// let ticker = Ticker::new(font).outline(Rgb::BLUE);
    /// ```
    pub fn outline(mut self, paint: impl Into<Paint>) -> Self {
        self.style.outline = Some(paint.into());
        self
    }

    /// Lights the rest of the box (behind the letters, and the whole box
    /// during the gap) with this colour or effect. Without it, the rest of
    /// the box is off (black).
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let ticker = Ticker::new(Font::X11_5X7).behind(Plasma::new());
    /// ```
    pub fn behind(mut self, paint: impl Into<Paint>) -> Self {
        self.style.behind = Some(paint.into());
        self
    }

    /// True to swap round: the box is lit with the letter colour, and the
    /// letters themselves are dark (or show what's behind).
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Black letters on a green box.
    /// let ticker = Ticker::new(Font::X11_5X7).letters(Rgb::GREEN).reverse(true);
    /// ```
    pub const fn reverse(mut self, reverse: bool) -> Self {
        self.style.reverse = reverse;
        self
    }

    /// How fast the text scrolls, in LEDs per second. 20 unless you say
    /// otherwise; 10 is slow and easy to read, 40 is quick. 0 counts as 1.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let slow = Ticker::new(Font::X11_5X7).speed(10);
    /// ```
    pub const fn speed(mut self, leds_per_second: u16) -> Self {
        self.speed = leds_per_second;
        self
    }

    /// How many LEDs of blank scroll by between one message leaving and the
    /// next coming in. Unless you say otherwise, one box width: the box is
    /// empty for as long as it takes the text to cross it.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Hardly any pause between messages.
    /// let busy = Ticker::new(Font::X11_5X7).gap(2);
    /// ```
    pub const fn gap(mut self, leds: u16) -> Self {
        self.gap = Some(leds);
        self
    }

    /// How long a message short enough to fit in the box stays still, in
    /// milliseconds, before the gap and the next message. 3000 (3 seconds)
    /// unless you say otherwise.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// // Show "12:34" for 5 seconds each time round.
    /// let ticker = Ticker::new(Font::X11_5X7).hold(5000);
    /// ```
    pub const fn hold(mut self, milliseconds: u32) -> Self {
        self.hold = milliseconds;
        self
    }

    /// Changes the messages. The message showing now finishes first (it
    /// scrolls off, then the gap); then the new list starts at its first
    /// message. Before the first [`render`](Ticker::render), or when there
    /// were no messages, the new list starts straight away.
    ///
    /// Letters other than plain ASCII (such as `é`) show as `?`. Messages
    /// past the ticker's room (256 bytes unless you chose otherwise, each
    /// message one byte per character plus one) are left out, and the one
    /// that runs out of room is cut short. An empty message is a pause.
    /// No messages at all makes the box blank.
    ///
    /// ```
    /// use ledkit_text::prelude::*;
    ///
    /// let mut ticker = Ticker::new(Font::X11_5X7);
    /// ticker.set_messages(&["Hello", "Rain at 4pm"]);
    /// ```
    pub fn set_messages(&mut self, messages: &[&str]) {
        if self.started.is_none() || self.list.is_empty() {
            self.list.fill(messages);
            self.has_pending = false;
            self.current = 0;
            self.started = None;
        } else {
            self.pending.fill(messages);
            self.has_pending = true;
        }
    }

    /// Draws the ticker onto `surface` (a matrix `Canvas`, a strip, ...) at
    /// the time `now`, moving on to the next message when one has finished.
    /// Only LEDs inside the box change.
    ///
    /// The first call starts the first message at `now`. If `now` is earlier
    /// than last time (the clock was reset, or it went round past its
    /// biggest value), the message showing starts again.
    ///
    /// ```
    /// use ledkit_layout::{Canvas, MatrixLayout};
    /// use ledkit_text::prelude::*;
    ///
    /// let mut pixels = [Rgb::BLACK; 16 * 8];
    /// let mut panel = Canvas::new(&mut pixels, MatrixLayout::new(16, 8)).unwrap();
    ///
    /// let mut ticker = Ticker::new(Font::X11_5X7);
    /// ticker.set_messages(&["Hello"]);
    /// for millis in (0..1000).step_by(50) {
    ///     ticker.render(&mut panel, Time::from_millis(millis));
    /// }
    /// ```
    pub fn render(&mut self, surface: &mut (impl Surface + ?Sized), now: impl Into<Time>) {
        let now = now.into();
        let mut style = self.style;
        if !self.placed {
            style.left = 0;
            style.top = 0;
            style.width = surface.width();
            style.height = surface.height();
        }
        let box_width = style.width;
        let gap = self.gap.unwrap_or(box_width);
        let now_ms = now.as_millis();
        // Time going backwards (or round past u32::MAX) restarts the message.
        let mut start = match self.started {
            Some(start) if start <= now_ms => start,
            _ => now_ms,
        };
        // Move on past every message that has finished by now.
        let mut skips = 0;
        while !self.list.is_empty() {
            let text_width = style.font.text_width(self.list.message(self.current));
            let duration = self.duration(text_width, box_width, gap);
            if u64::from(now_ms - start) < duration {
                break;
            }
            if skips == MOST_SKIPS {
                start = now_ms;
                break;
            }
            // `duration` is at most `now_ms - start`, so this fits.
            start += u32::try_from(duration).unwrap_or(0);
            self.switch();
            skips += 1;
        }
        self.started = Some(start);

        let text = self.list.message(self.current);
        let text_width = style.font.text_width(text);
        let elapsed = now_ms - start;
        let text_top = self
            .up_down
            .offset(i32::from(style.height) - i32::from(style.font.height()));
        match self.text_left(elapsed, text_width, box_width) {
            Some(text_left) => {
                let (text, text_left) = visible_part(style.font, text, text_left, box_width);
                style.draw(surface, now, text, text_left, text_top);
            }
            None => style.draw(surface, now, b"", 0, text_top),
        }
    }

    /// The speed in LEDs per second, at least 1.
    fn leds_per_second(&self) -> u64 {
        u64::from(self.speed.max(1))
    }

    /// How long a message `text_width` wide takes, gap included, in
    /// milliseconds.
    fn duration(&self, text_width: u16, box_width: u16, gap: u16) -> u64 {
        let speed = self.leds_per_second();
        if text_width > box_width {
            // Rounded up: the first millisecond the whole travel is done.
            let travel = u64::from(box_width) + u64::from(text_width) + u64::from(gap);
            (travel * 1000).div_ceil(speed)
        } else {
            u64::from(self.hold) + u64::from(gap) * 1000 / speed
        }
    }

    /// Where the left edge of the text is in the box, `elapsed` milliseconds
    /// into a message, or `None` when the box is blank (the gap).
    fn text_left(&self, elapsed: u32, text_width: u16, box_width: u16) -> Option<i32> {
        let (text_width, box_width) = (i64::from(text_width), i64::from(box_width));
        if text_width > box_width {
            // How many LEDs it has moved: at most u32::MAX * 65535 / 1000.
            let moved = u64::from(elapsed) * self.leds_per_second() / 1000;
            let moved = i64::try_from(moved).unwrap_or(i64::MAX);
            // `box_width - moved` is then between -text_width and box_width.
            (moved < box_width + text_width).then(|| (box_width - moved) as i32)
        } else {
            (elapsed < self.hold).then(|| ((box_width - text_width) / 2) as i32)
        }
    }

    /// Moves on to the next message, or to the first of a new list if one is
    /// waiting.
    fn switch(&mut self) {
        if self.has_pending {
            core::mem::swap(&mut self.list, &mut self.pending);
            self.has_pending = false;
            self.current = 0;
        } else {
            self.current = self.list.next(self.current);
        }
    }
}

/// The letters of `text` that can show in a box `box_width` wide when the
/// text starts at `text_left`, and where they start.
///
/// The segment fonts work out each LED by walking the text from its start,
/// so they get the whole text. For the bitmap fonts, whose letters are all
/// the same width, only the letters that can show are kept (and one either
/// side, for the outline), so a long message costs no more per LED than a
/// short one.
fn visible_part(font: Font, text: &[u8], text_left: i32, box_width: u16) -> (&[u8], i32) {
    let Some(advance) = font.fixed_advance() else {
        return (text, text_left);
    };
    let advance = i64::from(advance.max(1));
    let left = i64::from(text_left);
    // The outline looks one LED past each edge of the box.
    let first_column = (-left - 1).max(0);
    let last_column = i64::from(box_width) - left;
    let first = usize::try_from(first_column / advance)
        .unwrap_or(usize::MAX)
        .min(text.len());
    let end = usize::try_from(last_column.div_euclid(advance) + 1)
        .unwrap_or(0)
        .clamp(first, text.len());
    // `first` letters are at most a u16 of LEDs wide, so this fits.
    let skipped = i64::try_from(first).unwrap_or(0) * advance;
    (&text[first..end], (left + skipped) as i32)
}

/// Messages kept back to back, each followed by a 0 byte.
#[derive(Copy, Clone)]
struct List<const LEN: usize> {
    bytes: [u8; LEN],
    /// How many bytes of `bytes` are in use.
    used: usize,
}

impl<const LEN: usize> List<LEN> {
    const EMPTY: Self = Self {
        bytes: [0; LEN],
        used: 0,
    };

    /// True when there are no messages.
    const fn is_empty(&self) -> bool {
        self.used == 0
    }

    /// Replaces the messages with as many of `messages` as fit.
    fn fill(&mut self, messages: &[&str]) {
        self.used = 0;
        for message in messages {
            let room = LEN - self.used;
            // An empty message needs one byte; any other at least two.
            if room == 0 || (room == 1 && !message.is_empty()) {
                break;
            }
            for character in message.chars() {
                // Keep one byte for the end marker.
                if self.used + 1 >= LEN {
                    break;
                }
                self.bytes[self.used] = if character.is_ascii() && character != '\0' {
                    character as u8
                } else {
                    b'?'
                };
                self.used += 1;
            }
            self.bytes[self.used] = 0;
            self.used += 1;
        }
    }

    /// The message starting at byte `start`.
    fn message(&self, start: usize) -> &[u8] {
        let rest = &self.bytes[start.min(self.used)..self.used];
        let end = rest
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(rest.len());
        &rest[..end]
    }

    /// Where the message after the one at `start` starts, going back to the
    /// first after the last.
    fn next(&self, start: usize) -> usize {
        let next = start + self.message(start).len() + 1;
        if next >= self.used {
            0
        } else {
            next
        }
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
mod tests {
    use super::*;
    use ledkit_color::Rgb;
    use ledkit_layout::{Canvas, MatrixLayout};

    const FONT: Font = Font::X11_5X7;

    /// An 8x8 panel's colours, row by row.
    type Leds = [[Rgb; 8]; 8];

    /// Renders `ticker` at `millis` onto an 8x8 panel and returns it.
    fn render<const LEN: usize>(ticker: &mut Ticker<LEN>, millis: u32) -> Leds {
        let mut pixels = [Rgb::new(1, 2, 3); 64];
        let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(8, 8)).unwrap();
        ticker.render(&mut canvas, Time::from_millis(millis));
        let mut leds = [[Rgb::BLACK; 8]; 8];
        for (y, row) in leds.iter_mut().enumerate() {
            for (x, led) in row.iter_mut().enumerate() {
                *led = canvas.get(x as u16, y as u16).unwrap();
            }
        }
        leds
    }

    /// `text` in white on black with its top left at (`left`, 0), as 5x7 in
    /// an 8 tall box sits (the middle of 1 spare row rounds down to 0).
    fn showing(text: &[u8], left: i32) -> Leds {
        let mut leds = [[Rgb::BLACK; 8]; 8];
        for (y, row) in leds.iter_mut().enumerate() {
            for (x, led) in row.iter_mut().enumerate() {
                *led = match FONT.coverage(text, x as i32 - left, y as i32) {
                    255 => Rgb::WHITE,
                    0 => Rgb::BLACK,
                    other => panic!("partial coverage {other}"),
                };
            }
        }
        leds
    }

    const BLANK: Leds = [[Rgb::BLACK; 8]; 8];

    #[test]
    fn enters_from_the_right() {
        let mut ticker = Ticker::new(FONT).speed(20);
        ticker.set_messages(&["ABCD"]);
        assert_eq!(render(&mut ticker, 0), BLANK);
        // 50 ms at 20 LEDs a second: one LED in.
        assert_eq!(render(&mut ticker, 50), showing(b"ABCD", 7));
        assert_eq!(render(&mut ticker, 500), showing(b"ABCD", -2));
        // The first render sets the start, whatever the time.
        let mut later = Ticker::new(FONT);
        later.set_messages(&["ABCD"]);
        assert_eq!(render(&mut later, 10_000), BLANK);
        assert_eq!(render(&mut later, 10_050), showing(b"ABCD", 7));
    }

    #[test]
    fn scrolls_off_then_gap_then_next() {
        let mut ticker = Ticker::new(FONT).speed(20);
        ticker.set_messages(&["ABCD", "WXYZ"]);
        assert_eq!(FONT.text_width(b"ABCD"), 20);
        let _ = render(&mut ticker, 0);
        assert_eq!(render(&mut ticker, 1000), showing(b"ABCD", -12));
        // d = 28 (8 + 20) to d = 36 (+ the gap of one box width).
        for millis in (1400..1800).step_by(50) {
            assert_eq!(render(&mut ticker, millis), BLANK, "{millis} ms");
        }
        assert_eq!(render(&mut ticker, 1850), showing(b"WXYZ", 7));
        // Then back to the first.
        assert_eq!(render(&mut ticker, 3650), showing(b"ABCD", 7));
    }

    #[test]
    fn skips_ahead_when_called_rarely() {
        let mut ticker = Ticker::new(FONT);
        ticker.set_messages(&["ABCD", "WXYZ"]);
        let _ = render(&mut ticker, 0);
        // Straight to the second message, without the frames in between.
        assert_eq!(render(&mut ticker, 1850), showing(b"WXYZ", 7));
    }

    #[test]
    fn short_message_holds() {
        let mut ticker = Ticker::new(FONT).speed(20);
        ticker.set_messages(&["1"]);
        // Centred: (8 - 5) / 2 rounds down to 1.
        for millis in [0, 1000, 2000, 2999] {
            assert_eq!(render(&mut ticker, millis), showing(b"1", 1), "{millis} ms");
        }
        // The gap: 8 LEDs at 20 a second is 400 ms.
        for millis in [3000, 3200, 3399] {
            assert_eq!(render(&mut ticker, millis), BLANK, "{millis} ms");
        }
        assert_eq!(render(&mut ticker, 3400), showing(b"1", 1));
        assert_eq!(render(&mut ticker, 6399), showing(b"1", 1));
        assert_eq!(render(&mut ticker, 6400), BLANK);
    }

    #[test]
    fn hold_gap_and_speed_settings() {
        let mut ticker = Ticker::new(FONT).hold(1000).gap(4).speed(40);
        ticker.set_messages(&["1"]);
        let _ = render(&mut ticker, 0);
        assert_eq!(render(&mut ticker, 999), showing(b"1", 1));
        // 4 LEDs at 40 a second: 100 ms.
        assert_eq!(render(&mut ticker, 1000), BLANK);
        assert_eq!(render(&mut ticker, 1100), showing(b"1", 1));
    }

    #[test]
    fn new_list_waits_for_the_gap() {
        let mut ticker = Ticker::new(FONT).speed(20);
        ticker.set_messages(&["ABCD"]);
        let _ = render(&mut ticker, 0);
        let _ = render(&mut ticker, 500);
        ticker.set_messages(&["WXYZ", "1"]);
        assert_eq!(render(&mut ticker, 1000), showing(b"ABCD", -12));
        assert_eq!(render(&mut ticker, 1500), BLANK);
        // Set during the gap: still the next thing shown.
        ticker.set_messages(&["JKLM"]);
        assert_eq!(render(&mut ticker, 1850), showing(b"JKLM", 7));
        // The new list starts at its first message, and loops on its own.
        assert_eq!(render(&mut ticker, 3650), showing(b"JKLM", 7));
    }

    #[test]
    fn set_before_the_first_render_starts_now() {
        let mut ticker = Ticker::new(FONT);
        ticker.set_messages(&["ABCD"]);
        ticker.set_messages(&["WXYZ"]);
        let _ = render(&mut ticker, 0);
        assert_eq!(render(&mut ticker, 50), showing(b"WXYZ", 7));
    }

    #[test]
    fn empty_and_backwards_time() {
        // No messages: the box shows behind, or black.
        let mut ticker = Ticker::new(FONT);
        assert_eq!(render(&mut ticker, 0), BLANK);
        let mut blue = Ticker::new(FONT).behind(Rgb::BLUE);
        blue.set_messages(&[]);
        assert_eq!(render(&mut blue, 100), [[Rgb::BLUE; 8]; 8]);
        // Messages after none start at the next render.
        ticker.set_messages(&["ABCD"]);
        assert_eq!(render(&mut ticker, 5000), BLANK);
        assert_eq!(render(&mut ticker, 5050), showing(b"ABCD", 7));
        // Time goes backwards: the message starts again.
        assert_eq!(render(&mut ticker, 1000), BLANK);
        assert_eq!(render(&mut ticker, 1050), showing(b"ABCD", 7));
        // The millisecond clock going round past u32::MAX is the same.
        let _ = render(&mut ticker, u32::MAX - 10);
        assert_eq!(render(&mut ticker, 40), BLANK);
        assert_eq!(render(&mut ticker, 90), showing(b"ABCD", 7));
        // An empty list set while showing: blank after the gap.
        ticker.set_messages(&[]);
        assert_eq!(render(&mut ticker, 140), showing(b"ABCD", 6));
        assert_eq!(render(&mut ticker, 1900), BLANK);
        assert_eq!(render(&mut ticker, 9000), BLANK);
    }

    #[test]
    fn never_stuck() {
        // Nothing to wait for between messages: each takes no time at all.
        let mut ticker = Ticker::new(FONT).hold(0).gap(0);
        ticker.set_messages(&["1", "2"]);
        let _ = render(&mut ticker, 0);
        assert_eq!(render(&mut ticker, u32::MAX), BLANK);
        // Speed 0 counts as 1, and very long waits are fine.
        let mut slow = Ticker::new(FONT).speed(0).gap(u16::MAX);
        slow.set_messages(&["ABCD"]);
        let _ = render(&mut slow, 0);
        assert_eq!(render(&mut slow, 1000), showing(b"ABCD", 7));
        let _ = render(&mut slow, u32::MAX);
        // A box with no width.
        let mut pixels = [Rgb::BLACK; 64];
        let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(8, 8)).unwrap();
        let mut none = Ticker::new(FONT).at(0, 0, 0, 0);
        none.set_messages(&["", "1"]);
        for millis in [0, 1, 1000, u32::MAX] {
            none.render(&mut canvas, Time::from_millis(millis));
        }
        let mut empty: [Rgb; 0] = [];
        none.render(&mut empty, Time::from_millis(0));
    }

    #[test]
    fn capacity_and_ascii() {
        let mut list = List::<10>::EMPTY;
        list.fill(&["héllo", "abcdef", "x"]);
        // "h?llo" and its end take 6; "abc" and its end the other 4.
        assert_eq!(list.message(0), b"h?llo");
        assert_eq!(list.message(list.next(0)), b"abc");
        assert_eq!(list.next(list.next(0)), 0);
        // Room for one byte: an empty message fits, any other is left out.
        list.fill(&["abcdefgh", "", "x"]);
        assert_eq!(list.message(0), b"abcdefgh");
        assert_eq!(list.message(9), b"");
        assert_eq!(list.used, 10);
        list.fill(&["abcdefgh", "x"]);
        assert_eq!(list.used, 9);
        assert_eq!(list.next(0), 0);
        list.fill(&["nul\0"]);
        assert_eq!(list.message(0), b"nul?");
        let mut none = List::<0>::EMPTY;
        none.fill(&["anything", ""]);
        assert!(none.is_empty());
        let mut ticker = Ticker::<0>::with_capacity(FONT);
        ticker.set_messages(&["anything"]);
        assert_eq!(render(&mut ticker, 0), BLANK);
    }

    #[test]
    fn long_message_is_drawn_whole() {
        // Only part of a long message is handed to the font; it must look
        // the same as the whole message, outline included. With no gap
        // between letters, '#' reaches into the next letter's outline.
        let font = FONT;
        let text = "#$+# The quick brown fox jumps over the lazy dog #$+#";
        let mut ticker = Ticker::new(font)
            .outline(Rgb::RED)
            .behind(Rgb::BLUE)
            .speed(1000);
        ticker.set_messages(&[text]);
        let width = i32::from(font.text_width(text.as_bytes()));
        let mut style = Style::new(font);
        style.outline = Some(Rgb::RED.into());
        style.behind = Some(Rgb::BLUE.into());
        (style.width, style.height) = (8, 8);
        let _ = render(&mut ticker, 0);
        for d in 0..width + 8 {
            let got = render(&mut ticker, d as u32);
            let mut pixels = [Rgb::BLACK; 64];
            let mut canvas = Canvas::new(&mut pixels, MatrixLayout::new(8, 8)).unwrap();
            style.draw(&mut canvas, Time::from_millis(0), text.as_bytes(), 8 - d, 0);
            for (y, row) in got.iter().enumerate() {
                for (x, &led) in row.iter().enumerate() {
                    assert_eq!(led, canvas.get(x as u16, y as u16).unwrap(), "d {d}");
                }
            }
        }
    }
}
