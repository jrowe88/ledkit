//! See ledkit animations in your terminal, without any LEDs.
//!
//! ```text
//! cargo run -- swirl            # one effect on an 8x8 matrix
//! cargo run -- comet --strip 30 # one effect on a 30-LED strip
//! cargo run -- demo --size 64x32 # a bigger matrix, width x height
//! cargo run -- demo --char 219  # draw each LED with a character (here █)
//! cargo run -- demo --dots      # round LEDs, unlit ones dark grey, like a real panel
//! cargo run -- demo             # the playlist from the ESP32 example
//! cargo run -- list             # the names you can use
//! ```
//!
//! Each LED is drawn as a colored block. Your terminal needs "true color"
//! (24-bit) support; Windows Terminal, macOS Terminal, iTerm2 and most Linux
//! terminals have it.

use std::fmt::Write as _;
use std::io::Write as _;
use std::time::{Duration, Instant};

use ledkit_anim::prelude::*;
use ledkit_layout::{Canvas, MatrixLayout};

/// The matrix size shown unless `--size` or `--strip` is given.
const WIDTH: u16 = 8;
const HEIGHT: u16 = 8;

/// The longest side `--size` accepts.
const MAX_SIDE: u16 = 128;

/// Code page 437 (the old IBM PC character set) from 128 to 255, so that
/// `--char 219` is `█` as it was in DOS.
const CP437_HIGH: &str = "ÇüéâäàåçêëèïîìÄÅÉæÆôöòûùÿÖÜ¢£¥₧ƒáíóúñÑªº¿⌐¬½¼¡«»\
    ░▒▓│┤╡╢╖╕╣║╗╝╜╛┐└┴┬├─┼╞╟╚╔╩╦╠═╬╧╨╤╥╙╘╒╓╫╪┘┌█▄▌▐▀\
    αßΓπΣσµτΦΘΩδ∞φε∩≡±≥≤⌠⌡÷≈°∙·√ⁿ²■\u{a0}";

/// The color of an unlit LED with `--dots`: dark grey, as a real panel's
/// LEDs look when they're off.
const UNLIT: Rgb = Rgb::new(40, 40, 40);

/// How long to run before stopping (press Ctrl-C to stop sooner).
const RUN_FOR: Duration = Duration::from_secs(20);

/// About 30 frames a second.
const FRAME: Duration = Duration::from_millis(33);

fn main() {
    // The arguments: an effect name (or `list` or `demo`), `--size WxH`,
    // `--strip N`, `--char C` and `--dots`.
    let mut name = String::from("demo");
    let mut strip_len = None;
    let mut size = None;
    let mut glyph = None;
    let mut dots = false;
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
        } else if arg == "--dots" {
            dots = true;
        } else {
            name = arg;
        }
    }
    let name = name.as_str();

    if name == "list" {
        println!("Effects (other common names work too, like \"marquee\" or \"fire\"):");
        for effect in Effect::ALL {
            println!("  {}", effect.name());
        }
        println!("  demo (a playlist of several)");
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
    let animation: &dyn Animation = match &chosen {
        Some(effect) => effect,
        None if name == "demo" => &demo,
        None => {
            eprintln!("No effect called \"{name}\". Try `cargo run -- list`.");
            std::process::exit(1);
        }
    };

    let (width, height) = match (strip_len, size) {
        (Some(len), _) => (len, 1),
        (None, Some(width_height)) => width_height,
        (None, None) => (WIDTH, HEIGHT),
    };
    let mut pixels = vec![Rgb::BLACK; usize::from(width) * usize::from(height)];
    let layout = MatrixLayout::new(width, height);

    print!("\x1b[2J"); // clear the screen
    let start = Instant::now();
    while start.elapsed() < RUN_FOR {
        let now = Time::from(start.elapsed());
        (&animation).render(&mut Canvas::new(&mut pixels, layout).unwrap(), now);
        draw(&pixels, width, height, name, glyph, dots);
        std::thread::sleep(FRAME);
    }
    println!("\x1b[0m");
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

/// Prints the LEDs as colored blocks, from the top-left corner of the terminal.
///
/// Up to 32x32, each LED is two spaces wide and one line tall. Bigger matrices
/// use half blocks (`▀`, its top in one LED's color and its bottom in the next
/// row's), so each LED is one column wide and half a line tall, and a 64x64
/// panel fits in 64 columns and 32 lines.
///
/// With `--char`, each LED is that character in the LED's color on black:
/// twice up to 32x32, once for bigger matrices, one line per row.
///
/// With `--dots`, each LED is a `●` (or the `--char` character) followed by a
/// space, so the LEDs sit on a square grid with gaps between them, and LEDs
/// that are off or nearly off show dark grey instead of disappearing.
fn draw(pixels: &[Rgb], width: u16, height: u16, name: &str, glyph: Option<char>, dots: bool) {
    let mut out = String::from("\x1b[H"); // cursor to the top-left
    let _ = write!(out, "ledkit-anim: {name} ({width}x{height})\x1b[K\n\n");
    let rows: Vec<&[Rgb]> = pixels
        .chunks(usize::from(width))
        .take(usize::from(height))
        .collect();
    let small = width <= 32 && height <= 32;
    if dots {
        let glyph = glyph.unwrap_or('●');
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
    fn sizes() {
        assert_eq!(parse_size("32x16"), Some((32, 16)));
        assert_eq!(parse_size("64X64"), Some((64, 64)));
        assert_eq!(parse_size("0x8"), None);
        assert_eq!(parse_size("64"), None);
        assert_eq!(parse_size("129x8"), None);
    }
}
