# Animation preview in the terminal

See `ledkit-anim` effects on your computer, without any LEDs: each LED is drawn as a
colored dot in the terminal. Handy for choosing an effect and its settings before
flashing a board.

```bash
cargo run -- list
```

```bash
cargo run -- swirl
```

```bash
cargo run -- comet --strip 30
```

```bash
cargo run -- demo
```

```bash
cargo run -- demo --size 64x64
```

- `list` prints the effect names. Common other names work too, such as `marquee`,
  `fire`, `concentric` or `rotating-x`.
- A name shows that effect, with its default settings, on an 8x8 matrix.
- `--size WxH` shows it on a bigger (or smaller) matrix, width first: for example
  `--size 32x16`, `32x32`, `64x32` or `64x64`, the common HUB75 panel sizes. A
  64x64 panel needs a window at least 128 columns wide and 66 lines tall (or 64
  by 34 with `--blocks`).
- `--strip N` shows it on a strip of N LEDs instead.
- Each LED is a round dot in its color, and unlit LEDs are dark grey dots, the
  way a real panel looks.
- `--char C` draws each LED as another character instead of `⬤`. If your font
  draws `⬤` badly, `--char 9679` gives a smaller dot, `●`, that nearly every font
  has. Give the character itself (`--char "#"`, `--char ●`) or its code: 32 to
  126 for ASCII, 128 to 255 for the DOS character set (code page 437, so
  `--char 219` is `█` and `--char 254` is `■`), or any Unicode code point
  (`--char 9679` or `--char U+25CF` for `●`).
- `--blocks` draws solid squares with no gaps, and unlit LEDs black. Matrices
  bigger than 32x32 are then drawn at half height (one character per two rows) so
  they fit in a smaller window. With `--char`, the character fills the square.
- `demo` (also what you get with no name) plays the matrix show from the
  `esp32c6-anim` example.

It runs for 20 seconds; press Ctrl-C to stop sooner. To try other settings, change
`src/main.rs`.

Your terminal needs "true color" (24-bit color) support: Windows Terminal, macOS
Terminal, iTerm2 and most Linux terminals have it. The colors are the raw animation
colors, with no brightness limit or gamma, so they look brighter and flatter than on
real LEDs.
