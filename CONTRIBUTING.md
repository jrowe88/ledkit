# Contributing to ledkit

Thanks for helping. ledkit aims to be easy to use and easy to contribute to.

## Build and test

```sh
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo build -p ledkit-color --target thumbv6m-none-eabi
cargo build -p ledkit-color --target riscv32imc-unknown-none-elf
cargo build -p ledkit-core --all-features --target thumbv6m-none-eabi
cargo build -p ledkit-core --all-features --target riscv32imc-unknown-none-elf
```

`ledkit-esp` and `ledkit-esp-idf` are outside the workspace and only build for an ESP32 target. Check them through their examples (`examples/esp32c6-rmt`, `examples/esp32c6-esp-idf`; see AGENTS.md and CI) with `cargo build --release` and `cargo clippy -- -D warnings` in the example's folder.

The embedded targets come from `rustup target add thumbv6m-none-eabi riscv32imc-unknown-none-elf`.
See [AGENTS.md](AGENTS.md) for the code conventions.

## License of contributions

Unless you state otherwise, any contribution you submit is licensed under
MIT OR Apache-2.0, the same as the project, without additional terms.

## Add an LED type

LED chips live in `crates/ledkit-core/src/chips.rs`.

Every new chip gets a doc comment in plain words that says what products use it, and cites the datasheet (name, revision and the table the timings come from).

### A clockless chip (one data wire, NeoPixel-style)

1. Add a struct with `clockless_chip!`. Set the pixel type (`Rgb` or `Rgbw`), the channel order, the `[zero_high, zero_low, one_high, one_low]` phases as `ph(typical, min, max)` in nanoseconds, and the reset time in microseconds.
2. Add a row for it to the `clockless_catalogue_matches_the_table` test and add the type to the `every_typical_value_is_inside_its_window` test, both in `chips.rs`.
3. Add a test in `crates/ledkit-core/src/encode/spi.rs` that checks `ClocklessTiming::of::<YourChip>().spi_encoding(...)` at a common SPI clock (for example 3.2 MHz or 4 MHz), including the expected bit pattern.
4. Add a row to the "Which chip type do I use?" table (see below).
5. Run the commands under "Build and test".

### A clocked chip (data plus clock, DotStar-style)

1. Add a struct with `clocked_chip!`. It takes only the type name and the printed name: the macro fixes the APA102 frame format, `BGR` channel order and `Rgb` pixels, so there are no timings to enter. If your chip needs a different order, pixel type or frame format, `clocked_chip!` doesn't fit; open an issue first.
2. Add it to the `clocked_catalogue_matches_the_table` test in `chips.rs`.
3. Add a row to the "Which chip type do I use?" table (see below).
4. Run the commands under "Build and test".

### The "Which chip type do I use?" table

The same table appears in three places, and they must stay identical: the top of `crates/ledkit-core/src/chips.rs`, the crate docs in `crates/ledkit-core/src/lib.rs`, and `crates/ledkit-core/README.md` (which uses plain backticks instead of links).

## Add a backend

A backend turns a `Strip`'s finished frame into signals on a real peripheral. `crates/ledkit-spi` is the model to copy for a bus backend (SPI). Pulse peripherals (the ESP32 RMT, the RP PIO) are a second model: copy `crates/ledkit-esp-idf` (std) or `crates/ledkit-esp` (`no_std`), which turn the chip's timing into pulses with `PulseTicks` / `BitPulse` and the RMT word helpers in `ledkit_core::encode`. `crates/ledkit-rp` is the model for a program that sends every bit in the same time (`PioTiming`, built on `EqualPeriodTicks`) from a FIFO, fed by the processor or by DMA. A peripheral whose bit length can't change per bit (a PWM sequencer) is the nRF's version of that: copy `crates/ledkit-nrf`, which uses `ClocklessTiming::equal_period_ticks` and the PWM duty-word helpers. The steps:

1. **Implement `Output` (blocking) and `OutputAsync` (if the HAL has async)** from `ledkit-core`. `write` receives the pixels after `Strip` has applied brightness, gamma, dithering and the power limit; never do any of that yourself.
2. **Check the chip kind at compile time**, with `const { ClocklessTiming::of::<C>() }` (or, for a clocked chip, `const { assert!(matches!(C::PROTOCOL, Protocol::Clocked(_))) }`, as `ClockedSpi::new` does), so a DotStar chip on a clockless backend fails to build instead of misbehaving.
3. **Use the `wire` and `encode` helpers** (`wire::clockless_bytes`, `wire::clocked_frame`, `SpiEncoding`, `PulseTicks`) instead of re-deriving color order, timing or bit patterns.
4. **Flush before returning**, so `show()` means the data has really left the chip and the LEDs can't be handed a half-sent frame.
5. **Own the reset gap.** A clockless chip needs a pause with the line low (its `reset_us`) between frames to latch the colors, and `Output`'s docs make that the backend's job. SPI satisfies it by sending trailing reset bytes and then flushing. A serial port can't send all-low bytes, so `ledkit-uart` flushes and then waits `reset_us` with an `embedded-hal` `DelayNs`. RMT and PIO backends must time it themselves: get the gap from `ResetGap::of::<C>()`, remember when the last frame ended and wait out the rest before the next one (`ledkit-esp-idf` sleeps, and counts a failed send as ended too; `ledkit-esp` instead appends the gap to the frame as a long low pulse).
6. **Test with a fake bus** (see `crates/ledkit-spi/src/testing.rs`): check the exact bytes for a known frame, error propagation, and that flush happens.
7. **Add a `smart-leds` impl** behind a `smart-leds` feature, so code written for the existing ecosystem needs only a new constructor.
8. **Write docs and an example** in the style of `crates/ledkit-spi/src/lib.rs`, and keep the crate's README identical in content (`crates/ledkit-core` does the same; the README uses plain backticks instead of links).
9. Run the commands under "Build and test", including the `no_std` builds for your crate.

## Add a panel preset

`ledkit-layout` has no panel presets yet. Add one only after the panel's wiring is confirmed on hardware with the wiring finder (a product page that doesn't state the wiring isn't enough), and say how it was checked.

## Add an effect

Effects live in `crates/ledkit-anim/src/effects/`, one file each. `rainbow.rs`
and `comet.rs` are good models.

1. **Make the struct** with its settings as private fields, `#[derive(Copy, Clone, Debug)]`
   and `#[must_use]`, a `pub const fn new()` with defaults that look good on an 8x8
   and a 100-LED strip, and `const fn` setters with plain names (`tail_length`, not
   `len`). Times are `core::time::Duration` (store milliseconds with `time::millis`),
   speeds are LEDs per second, and colors use a `colors: Colors` field plus the
   `color_setters!()` macro.
2. **Implement `Animation`.** `color_at` gets the LED's `Position` and the `Frame`
   (time and size) and returns its color. Keep it a pure function of those: no state
   between frames, integer math only, and no panics for any size (including 0 and
   1 LED) or time (including `u32::MAX`). For randomness, use `math::hash` of the LED
   and a time slot. Strip-like effects use `at.x` and `frame.width`; round ones use
   `effects::geometry::polar`.
3. **Register it**: `effects/mod.rs`, the prelude in `lib.rs`, a variant, name and
   aliases in `effect.rs` (and `Effect::ALL`), and the table in the README.
4. **Test it** in the same file: a few exact pixels at chosen times, how it moves,
   and `test_support::exercise` with the defaults and with every setting at 0.
5. **Look at it**: `cargo run -- <name>` in `examples/anim-preview` shows it in the
   terminal.

If you port an effect from an MIT-licensed project (FastLED, Adafruit's
CircuitPython LED_Animation, WS2812FX), add a line to `NOTICE`. Ideas from
copyleft projects (WLED, blinksy) are fine; their code is not.
