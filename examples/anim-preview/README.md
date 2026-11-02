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

- `list` prints the effect names. Common other names work too, such as `marquee`,
  `fire`, `concentric` or `rotating-x`.
- A name shows that effect, with its default settings, on an 8x8 matrix.
- `--strip N` shows it on a strip of N LEDs instead.
- `demo` (also what you get with no name) plays the matrix show from the
  `esp32c6-anim` example.

It runs for 20 seconds; press Ctrl-C to stop sooner. To try other settings, change
`src/main.rs`.

Your terminal needs "true color" (24-bit color) support: Windows Terminal, macOS
Terminal, iTerm2 and most Linux terminals have it. The colors are the raw animation
colors, with no brightness limit or gamma, so they look brighter and flatter than on
real LEDs.
