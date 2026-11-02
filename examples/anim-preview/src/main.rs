//! See ledkit animations in your terminal, without any LEDs.
//!
//! ```text
//! cargo run -- swirl            # one effect on an 8x8 matrix
//! cargo run -- comet --strip 30 # one effect on a 30-LED strip
//! cargo run -- demo --size 64x32 # a bigger matrix, width x height
//! cargo run -- demo --char 254  # square LEDs (■) instead of round ones
//! cargo run -- demo --blocks    # solid blocks with no gaps, unlit LEDs black
//! cargo run -- demo             # the playlist from the ESP32 example
//! cargo run -- list             # the names you can use
//! cargo run -- text "Hello!"    # a ticker scrolling a message (32x8 unless --size)
//! cargo run -- text "Hi" --size 64x16 --font 10x20
//! cargo run -- clock            # date and a 7-segment time, 64x64, system clock (UTC)
//! ```
//!
//! `text` and `clock` use `ledkit-text`. Add `--frames N` to any command to stop
//! after N frames. When it stops, it prints the average time it took to draw
//! one frame, which is a good guide to how fast a microcontroller would be (a
//! lot slower than a computer).
//!
//! Each LED is drawn as a colored dot, and unlit LEDs as dark grey dots, the way
//! a real panel looks. Your terminal needs "true color"
//! (24-bit) support; Windows Terminal, macOS Terminal, iTerm2 and most Linux
//! terminals have it.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::{Duration, Instant};

use ledkit_anim::prelude::*;
use ledkit_layout::{Canvas, MatrixLayout};
use ledkit_text::{Font, TextArea, Ticker};

/// The matrix size shown unless `--size` or `--strip` is given.
const WIDTH: u16 = 8;
const HEIGHT: u16 = 8;

/// The longest side `--size` accepts.
const MAX_SIDE: u16 = 256;

/// Code page 437 (the old IBM PC character set) from 128 to 255, so that
/// `--char 219` is `█` as it was in DOS.
const CP437_HIGH: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»\
    ░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀\
    αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";

/// The color of an unlit LED (unless `--blocks`): dark grey, as a real panel's
/// LEDs look when they're off.
const UNLIT: Rgb = Rgb::new(40, 40, 40);

/// The LED shape unless `--char` is given: a large circle (U+2B24). Fonts
/// without it fall back to another font; `--char 9679` (●) is smaller but in
/// nearly every font.
const BIG_DOT: char = '⬤';

/// How long to run before stopping (press Ctrl-C to stop sooner).
const RUN_FOR: Duration = Duration::from_secs(20);

/// The panel size for `clock` unless `--size` is given.
const CLOCK_SIZE: (u16, u16) = (64, 64);

/// The panel size for `text` unless `--size` is given.
const TEXT_SIZE: (u16, u16) = (32, 8);

/// The font names `--font` accepts, with what each one is.
const FONTS: [(&str, &str); 15] = [
    ("4x6", "bitmap, 4 wide, 6 tall"),
    ("5x7", "bitmap, 5 wide, 7 tall"),
    ("5x8", "bitmap, 5 wide, 8 tall"),
    ("6x10", "bitmap, 6 wide, 10 tall"),
    ("6x13", "bitmap, bold, 13 tall"),
    ("7x13", "bitmap, bold, 13 tall"),
    ("7x14", "bitmap, bold, 14 tall"),
    ("8x13", "bitmap, bold, 13 tall"),
    ("9x15", "bitmap, bold, 15 tall"),
    ("9x18", "bitmap, bold, 18 tall"),
    ("10x20", "bitmap, 10 wide, 20 tall"),
    ("tiny", "bitmap, 3 wide, 5 tall, capitals only"),
    (
        "7seg",
        "seven-segment display, 2 rows shorter than the panel",
    ),
    ("7seg-slanted", "slanted seven-segment display"),
    (
        "14seg",
        "fourteen-segment display, 2 rows shorter than the panel",
    ),
];

/// About 30 frames a second.
const FRAME: Duration = Duration::from_millis(33);

fn main() {
    // The arguments: an effect name (or `list` or `demo`), `--size WxH`,
    // `--strip N`, `--char C` and `--blocks`.
    let mut name = String::from("demo");
    let mut strip_len = None;
    let mut size = None;
    let mut glyph = None;
    let mut dots = true;
    let mut font_name = None;
    let mut max_frames = None;
    // The words that are not options: the effect name, then (for `text`) the message.
    let mut words: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--strip" {
            match args.next().and_then(|n| n.parse::<u16>().ok()) {
                Some(len) if len > 0 => strip_len = Some(len),
                _ => {
                    eprintln!("`--strip` needs a number of LEDs, from 1 to 65535.");
                    std::process::exit(1);
                }
            }
        } else if arg == "--size" {
            size = args.next().as_deref().and_then(parse_size);
            if size.is_none() {
                eprintln!(
                    "`--size` needs a width and height, like 32x16 or 64x64 (each 1 to {MAX_SIDE})."
                );
                std::process::exit(1);
            }
        } else if arg == "--char" {
            glyph = args.next().as_deref().and_then(parse_char);
            if glyph.is_none() {
                eprintln!(
                    "`--char` needs one character, like # or █, or its code: 32 to 126 for \
                         ASCII, 128 to 255 for the DOS (code page 437) set, so 219 is █, or a \
                         Unicode code point such as 9679 or U+25CF for ●."
                );
                std::process::exit(1);
            }
        } else if arg == "--blocks" {
            dots = false;
        } else if arg == "--font" {
            font_name = args.next();
            if font_name.is_none() {
                eprintln!("`--font` needs a name. Try `cargo run -- list`.");
                std::process::exit(1);
            }
        } else if arg == "--frames" {
            match args.next().and_then(|n| n.parse::<u32>().ok()) {
                Some(count) if count > 0 => max_frames = Some(count),
                _ => {
                    eprintln!("`--frames` needs a number of frames, 1 or more.");
                    std::process::exit(1);
                }
            }
        } else {
            words.push(arg);
        }
    }
    if let Some(first) = words.first() {
        name = first.clone();
    }
    let name = name.as_str();

    if name == "list" {
        println!("Effects (other common names work too, like \"marquee\" or \"fire\"):");
        for effect in Effect::ALL {
            println!("  {}", effect.name());
        }
        println!("  demo (a playlist of several)");
        println!();
        println!("Text (uses ledkit-text):");
        println!("  text \"message\" [--size WxH] [--font NAME] (a ticker; 32x8 unless --size)");
        println!(
            "  clock [--size WxH] (date and a 7-segment time, up to 30 rows tall, from the system clock, in UTC; 64x64)"
        );
        println!();
        println!("Fonts for --font:");
        for (font, what) in FONTS {
            println!("  {font:<13}{what}");
        }
        return;
    }

    // The demo: the matrix playlist from the ESP32 example.
    let swirl = Swirl::new();
    let rings = Rings::new().palette(&palettes::OCEAN);
    let pinwheel = Pinwheel::new().color(Rgb::ORANGE);
    let plasma = Plasma::new().palette(&palettes::PARTY);
    let flames = Flames::new();
    let rain = Rain::new();
    let second = Duration::from_secs(1);
    let entries = [
        Entry::new(&swirl, 4 * second),
        Entry::new(&rings, 4 * second).transition(Transition::fade(second)),
        Entry::new(&pinwheel, 4 * second).transition(Transition::iris(second)),
        Entry::new(&plasma, 4 * second).transition(Transition::dissolve(second)),
        Entry::new(&flames, 4 * second).transition(Transition::fade(second)),
        Entry::new(&rain, 4 * second).transition(Transition::wipe(Edge::Top, second)),
    ];
    let demo = Playlist::new(&entries);

    let chosen = Effect::from_name(name);
    let is_text = name == "text" || name == "clock";
    let animation: &dyn Animation = match &chosen {
        Some(effect) => effect,
        None if name == "demo" || is_text => &demo,
        None => {
            eprintln!("No effect called \"{name}\". Try `cargo run -- list`.");
            std::process::exit(1);
        }
    };

    let default_size = match name {
        "text" => TEXT_SIZE,
        "clock" => CLOCK_SIZE,
        _ => (WIDTH, HEIGHT),
    };
    let (width, height) = match (strip_len, size) {
        (Some(len), _) => (len, 1),
        (None, Some(width_height)) => width_height,
        (None, None) => default_size,
    };

    // The show: an effect (or the demo), a ticker, or the clock.
    let mut show = match name {
        "text" => {
            let font = match font_name.as_deref() {
                Some(font_name) => parse_font(font_name, height).unwrap_or_else(|| {
                    eprintln!("No font called \"{font_name}\". Try `cargo run -- list`.");
                    std::process::exit(1);
                }),
                None => default_font(height),
            };
            let message = if words.len() > 1 {
                words[1..].join(" ")
            } else {
                String::from("ledkit")
            };
            let mut ticker = Ticker::new(font).letters(Rainbow::new());
            ticker.set_messages(&[message.as_str()]);
            Show::Text(Box::new(ticker))
        }
        "clock" => Show::Clock(Box::new(Clock::new(width, height))),
        _ => Show::Effect(animation),
    };
    let mut pixels = vec![Rgb::BLACK; usize::from(width) * usize::from(height)];
    let layout = MatrixLayout::new(width, height);

    print!("\x1b[2J"); // clear the screen
    let start = Instant::now();
    let mut frames = 0u32;
    // The time spent working out the pictures (not printing them).
    let mut drawing = Duration::ZERO;
    while max_frames.map_or(start.elapsed() < RUN_FOR, |count| frames < count) {
        let now = Time::from(start.elapsed());
        let drawing_started = Instant::now();
        let mut canvas = Canvas::new(&mut pixels, layout).unwrap();
        match &mut show {
            Show::Effect(animation) => (&*animation).render(&mut canvas, now),
            Show::Text(ticker) => ticker.render(&mut canvas, now),
            Show::Clock(clock) => clock.render(&mut canvas, now),
        }
        drawing += drawing_started.elapsed();
        frames += 1;
        draw(&pixels, width, height, name, glyph, dots);
        std::thread::sleep(FRAME);
    }
    println!("\x1b[0m");
    if frames > 0 {
        println!(
            "Average time to draw a frame: {:.2} ms ({frames} frames, {width}x{height} LEDs).",
            drawing.as_secs_f64() * 1000.0 / f64::from(frames)
        );
    }
}

/// What `main` shows: an effect, a ticker or the clock.
enum Show<'a> {
    Effect(&'a dyn Animation),
    Text(Box<Ticker>),
    Clock(Box<Clock>),
}

/// A bitmap font that suits a panel this tall.
fn default_font(height: u16) -> Font {
    match height {
        0..=9 => Font::X11_5X7,
        10..=23 => Font::X11_6X10,
        _ => Font::X11_10X20,
    }
}

/// Finds a font by the names in `FONTS`. The segment fonts are drawn two rows
/// shorter than the panel.
fn parse_font(name: &str, panel_height: u16) -> Option<Font> {
    let segment_height = panel_height.saturating_sub(2).max(5);
    Some(match name {
        "4x6" => Font::X11_4X6,
        "5x7" => Font::X11_5X7,
        "5x8" => Font::X11_5X8,
        "6x10" => Font::X11_6X10,
        "6x13" => Font::X11_6X13_BOLD,
        "7x13" => Font::X11_7X13_BOLD,
        "7x14" => Font::X11_7X14_BOLD,
        "8x13" => Font::X11_8X13_BOLD,
        "9x15" => Font::X11_9X15_BOLD,
        "9x18" => Font::X11_9X18_BOLD,
        "10x20" => Font::X11_10X20,
        "tiny" => Font::TINY_3X5.spacing(1),
        "7seg" => Font::seven_segment(segment_height),
        "7seg-slanted" => Font::seven_segment_slanted(segment_height),
        "14seg" => Font::fourteen_segment(segment_height),
        _ => return None,
    })
}

/// The `clock` show: a small date on top, a big time below.
struct Clock {
    date: TextArea,
    time: TextArea,
}

impl Clock {
    /// A clock for a panel `width` x `height`: the date in the top 8 rows
    /// and the time in a 7-segment font (30 rows tall, or less if the panel is
    /// short or narrow) in the rest, with Plasma letters and a black outline.
    fn new(width: u16, height: u16) -> Clock {
        // A small font for the date, and a tiny one when the panel is narrow.
        let date_font = if width >= 60 {
            Font::X11_5X7
        } else {
            Font::TINY_3X5.spacing(1)
        };
        let date = TextArea::new(date_font)
            .at(0, 1, width, 7)
            .letters(Rgb::new(120, 120, 255));
        let below = height.saturating_sub(8);
        // As tall as 30 rows, but no taller than fits across the panel
        // (a 30-row "12:34" is 111 LEDs wide, wider than a 64-wide panel).
        let mut rows = 30.min(below).max(5);
        while rows > 5 && Font::seven_segment(rows).text_width(b"88:88") > width {
            rows -= 1;
        }
        let time = TextArea::new(Font::seven_segment(rows))
            .at(0, 8, width, below)
            .letters(Plasma::new())
            .outline(Rgb::BLACK);
        Clock { date, time }
    }

    /// Reads the system clock, updates both texts and draws them.
    fn render(&mut self, canvas: &mut Canvas<'_, Rgb, MatrixLayout>, now: Time) {
        let seconds = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let (date, time) = format_clock(seconds);
        self.date.set_text(&date);
        self.time.set_text(&time);
        self.date.render(canvas, now);
        self.time.render(canvas, now);
    }
}

/// The date ("MON 02 NOV") and the time ("12:34"; the colon blinks, so it is
/// a space every other second) for `seconds` since 1970, in UTC.
fn format_clock(seconds: u64) -> (String, String) {
    const DAYS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    const MONTHS: [&str; 12] = [
        "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
    ];
    let days = (seconds / 86_400) as i64;
    let (_year, month, day) = civil_from_days(days);
    // 1 January 1970 was a Thursday, day number 4 counting Sunday as 0.
    let weekday = (days + 4).rem_euclid(7) as usize;
    let of_day = seconds % 86_400;
    let (hours, minutes) = (of_day / 3600, of_day / 60 % 60);
    let colon = if of_day % 2 == 0 { ':' } else { ' ' };
    (
        format!(
            "{} {day:02} {}",
            DAYS[weekday],
            MONTHS[(month - 1) as usize]
        ),
        format!("{hours:02}{colon}{minutes:02}"),
    )
}

/// Turns a number of days since 1 January 1970 into (year, month, day).
/// This is Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_from_march = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_from_march + 2) / 5 + 1) as u32;
    let month = if month_from_march < 10 {
        month_from_march + 3
    } else {
        month_from_march - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Reads a size such as `32x16`: the width, then the height.
fn parse_size(text: &str) -> Option<(u16, u16)> {
    let (width, height) = text.split_once(['x', 'X'])?;
    let side = |s: &str| s.parse::<u16>().ok().filter(|n| (1..=MAX_SIDE).contains(n));
    Some((side(width)?, side(height)?))
}

/// Reads `--char`: a single character, or its code. Codes 128 to 255 are
/// taken from code page 437, as in DOS; others are Unicode code points, which
/// can also be written `U+25CF`.
fn parse_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        if !c.is_ascii_digit() {
            return Some(c);
        }
    }
    let code = match text.strip_prefix("U+").or_else(|| text.strip_prefix("u+")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => text.parse::<u32>().ok()?,
    };
    let c = match code {
        128..=255 => CP437_HIGH.chars().nth(usize::try_from(code - 128).ok()?)?,
        _ => char::from_u32(code)?,
    };
    (!c.is_control()).then_some(c)
}

/// Prints the LEDs, from the top-left corner of the terminal.
///
/// Normally each LED is a large dot, `⬤` (or the `--char` character), then a
/// space, so the LEDs sit on a square grid with gaps between them, and LEDs
/// that are off or nearly off show dark grey instead of disappearing.
///
/// With `--blocks`, up to 32x32, each LED is two spaces wide and one line tall. Bigger matrices
/// use half blocks (`▀`, its top in one LED's color and its bottom in the next
/// row's), so each LED is one column wide and half a line tall, and a 64x64
/// panel fits in 64 columns and 32 lines.
///
/// With `--blocks` and `--char`, each LED is that character in the LED's
/// color on black: twice up to 32x32, once for bigger matrices, one line per row.
fn draw(pixels: &[Rgb], width: u16, height: u16, name: &str, glyph: Option<char>, dots: bool) {
    let mut out = String::from("\x1b[H"); // cursor to the top-left
    let _ = write!(out, "ledkit-anim: {name} ({width}x{height})\x1b[K\n\n");
    let rows: Vec<&[Rgb]> = pixels
        .chunks(usize::from(width))
        .take(usize::from(height))
        .collect();
    let small = width <= 32 && height <= 32;
    if dots {
        let glyph = glyph.unwrap_or(BIG_DOT);
        for row in rows {
            out.push_str("[48;2;0;0;0m");
            for &color in row {
                let lit = color.r.max(color.g).max(color.b) > UNLIT.r;
                let shown = if lit { color } else { UNLIT };
                let _ = write!(out, "[38;2;{};{};{}m{glyph} ", shown.r, shown.g, shown.b);
            }
            out.push_str(
                "[0m
",
            );
        }
    } else if let Some(glyph) = glyph {
        let repeat = if small { 2 } else { 1 };
        for row in rows {
            out.push_str("\x1b[48;2;0;0;0m");
            for color in row {
                let _ = write!(out, "\x1b[38;2;{};{};{}m", color.r, color.g, color.b);
                for _ in 0..repeat {
                    out.push(glyph);
                }
            }
            out.push_str("\x1b[0m\n");
        }
    } else if small {
        for row in rows {
            for color in row {
                let _ = write!(out, "\x1b[48;2;{};{};{}m  ", color.r, color.g, color.b);
            }
            out.push_str("\x1b[0m\n");
        }
    } else {
        for pair in rows.chunks(2) {
            for (x, top) in pair[0].iter().enumerate() {
                let _ = write!(out, "\x1b[38;2;{};{};{}m", top.r, top.g, top.b);
                match pair.get(1) {
                    Some(bottom) => {
                        let color = bottom[x];
                        let _ = write!(out, "\x1b[48;2;{};{};{}m▀", color.r, color.g, color.b);
                    }
                    // An odd last row: no LED below, so the terminal's own background.
                    None => out.push_str("\x1b[49m▀"),
                }
            }
            out.push_str("\x1b[0m\n");
        }
    }
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_page_437() {
        assert_eq!(CP437_HIGH.chars().count(), 128);
        assert_eq!(parse_char("219"), Some('█'));
        assert_eq!(parse_char("176"), Some('░'));
        assert_eq!(parse_char("254"), Some('■'));
    }

    #[test]
    fn characters_and_codes() {
        assert_eq!(parse_char("#"), Some('#'));
        assert_eq!(parse_char("●"), Some('●'));
        assert_eq!(parse_char("35"), Some('#'));
        assert_eq!(parse_char("9679"), Some('●'));
        assert_eq!(parse_char("U+25CF"), Some('●'));
        assert_eq!(parse_char("7"), None); // a bell, not something to draw
        assert_eq!(parse_char("10"), None);
        assert_eq!(parse_char("ab"), None);
        assert_eq!(parse_char(""), None);
    }

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    #[test]
    fn clock_text() {
        // 2024-01-01 12:34:56 UTC was a Monday.
        let seconds = 19_723 * 86_400 + 12 * 3600 + 34 * 60 + 56;
        assert_eq!(format_clock(seconds), ("MON 01 JAN".into(), "12:34".into()));
        assert_eq!(format_clock(seconds + 1).1, "12 34");
    }

    #[test]
    fn fonts() {
        assert!(parse_font("nope", 8).is_none());
        for (name, _) in FONTS {
            assert!(parse_font(name, 16).is_some(), "{name}");
        }
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size("32x16"), Some((32, 16)));
        assert_eq!(parse_size("64X64"), Some((64, 64)));
        assert_eq!(parse_size("0x8"), None);
        assert_eq!(parse_size("64"), None);
        assert_eq!(parse_size("257x8"), None);
    }
}
