# Animation preview in the terminal

See `ledkit-anim` effects on your computer, without any LEDs: each LED is drawn as a
colored block in the terminal. Handy for choosing an effect and its settings before
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
  `--size 32x16`, `32x32`, `64x32` or `64x64`, the common HUB75 panel sizes. Up to
  32x32 each LED is a square block; bigger matrices are drawn at half height (one
  character per two rows) so they fit in the window. A 64x64 panel needs a window
  at least 64 columns wide and 34 lines tall.
- `--strip N` shows it on a strip of N LEDs instead.
- `--char C` draws each LED as a character in the LED's color, instead of a
  solid block. Give the character itself (`--char "#"`, `--char ●`) or its code:
  32 to 126 for ASCII, 128 to 255 for the DOS character set (code page 437, so
  `--char 219` is `█` and `--char 254` is `■`), or any Unicode code point
  (`--char 9679` or `--char U+25CF` for `●`).
- `--dots` draws round LEDs with gaps between them, and shows unlit LEDs as dark
  grey dots, the way a real panel looks. Add `--char` to use another shape, such
  as `--dots --char 254` for square LEDs. A 64-wide panel needs 128 columns and
  one line per row.
- `demo` (also what you get with no name) plays the matrix show from the
  `esp32c6-anim` example.

It runs for 20 seconds; press Ctrl-C to stop sooner. To try other settings, change
`src/main.rs`.

Your terminal needs "true color" (24-bit color) support: Windows Terminal, macOS
Terminal, iTerm2 and most Linux terminals have it. The colors are the raw animation
colors, with no brightness limit or gamma, so they look brighter and flatter than on
real LEDs.
