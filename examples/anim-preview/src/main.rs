//! See ledkit animations in your terminal, without any LEDs.
//!
//! ```text
//! cargo run -- swirl            # one effect on an 8x8 matrix
//! cargo run -- comet --strip 30 # one effect on a 30-LED strip
//! cargo run -- demo             # the playlist from the ESP32 example
//! cargo run -- list             # the names you can use
//! ```
//!
//! Each LED is drawn as a colored block. Your terminal needs "true color"
//! (24-bit) support; Windows Terminal, macOS Terminal, iTerm2 and most Linux
//! terminals have it.

use std::io::Write;
use std::time::{Duration, Instant};

use ledkit_anim::prelude::*;
use ledkit_layout::{Canvas, MatrixLayout};

/// The matrix size shown unless `--strip` is given.
const WIDTH: u16 = 8;
const HEIGHT: u16 = 8;

/// How long to run before stopping (press Ctrl-C to stop sooner).
const RUN_FOR: Duration = Duration::from_secs(20);

/// About 30 frames a second.
const FRAME: Duration = Duration::from_millis(33);

fn main() {
    // The arguments: an effect name (or `list` or `demo`), and `--strip N`.
    let mut name = String::from("demo");
    let mut strip_len = None;
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

    let (width, height) = match strip_len {
        Some(len) => (len, 1),
        None => (WIDTH, HEIGHT),
    };
    let mut pixels = vec![Rgb::BLACK; usize::from(width) * usize::from(height)];
    let layout = MatrixLayout::new(width, height);

    print!("\x1b[2J"); // clear the screen
    let start = Instant::now();
    while start.elapsed() < RUN_FOR {
        let now = Time::from(start.elapsed());
        (&animation).render(&mut Canvas::new(&mut pixels, layout).unwrap(), now);
        draw(&pixels, width, height, name);
        std::thread::sleep(FRAME);
    }
    println!("\x1b[0m");
}

/// Prints the LEDs as colored blocks, from the top-left corner of the terminal.
fn draw(pixels: &[Rgb], width: u16, height: u16, name: &str) {
    let mut out = String::from("\x1b[H"); // cursor to the top-left
    out.push_str(&format!("ledkit-anim: {name}\x1b[K\n\n"));
    for row in pixels.chunks(usize::from(width)).take(usize::from(height)) {
        for color in row {
            out.push_str(&format!("\x1b[48;2;{};{};{}m  ", color.r, color.g, color.b));
        }
        out.push_str("\x1b[0m\n");
    }
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(out.as_bytes());
    let _ = stdout.flush();
}
