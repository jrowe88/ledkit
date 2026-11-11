# ledkit (the facade crate): design

*Date: 2026-11-10. Status: built (first version), for the owner's review. Worked out with
the owner on 2026-11-10; the owner asked for development to go ahead on the designer's
judgment. Marked "my call" where the owner left a choice to the designer, and "changed
while building" where building found a reason to differ from the first draft.*

## Intent

`ledkit` is the one crate a hobbyist adds. Today a program names four or five crates
(`ledkit-core`, `ledkit-layout`, `ledkit-anim`, `ledkit-text`, `ledkit-esp`) and repeats
the same 20–30 lines of setup in every example: the RMT clock and its divider, sizing a
pulse buffer with `buffer_len`, a hand-written `millis()` closure and a fixed
`delay_ms(16)` loop. The facade gets a first program down to "add `ledkit` and your HAL,
`use ledkit::prelude::*`, ask for a strip, draw something every frame", without hiding
anything: every shortcut is made of public pieces a user can assemble by hand.

This is milestone 8 in [the roadmap](../roadmap.md). It comes before the first crates.io
release, so the first thing people find there is the easy way in.

## Goals

- **One dependency** for ledkit itself, with Cargo features to pick the backend.
- **A prelude** with what a first program needs; everything else reachable by path.
- **Board helpers, ESP first:** ESP32-C6 and ESP32-S3, on both esp-hal (`no_std`) and
  ESP-IDF (`std`). One call gives a ready-to-use `Strip`.
- **A frame loop and clock:** "give me the time, about N times a second", blocking and
  async.
- **Blocking and async (Embassy) as equals**, wherever the backend supports both.
- **Rust-like:** typed values and units, `Result` and `?`, builders and `const fn`
  where they help. The spirit of the [design spec's API sketch](ledkit-design.md#api-feel-illustrative-not-final),
  not its letter (no `Seconds(5.0)` float units in `no_std` code).

## Non-goals (this round)

- Board helpers for RP2040/RP2350, nRF52840, Teensy 4 or Linux. The re-exports cover those
  boards now (`ledkit::nrf`, `ledkit::spi` and so on); helpers come later, following the ESP
  pattern.
- `cargo generate` board templates, the mdBook guide, `llms.txt`.
- Re-exporting HALs. The user adds `esp-hal` (or `esp-idf-hal`/`esp-idf-svc`) to their own
  `Cargo.toml`; they need it for start-up anyway, and it keeps HAL upgrades out of ledkit's
  version number.

## Anti-goals

- **A second API.** Helpers return the same `Strip`, outputs and types as the crates
  underneath. Dropping a helper for the hand-written version is a local change, never a
  rewrite.
- **Hiding the hardware.** Helpers take the peripherals the user already has (a pin, the
  RMT); they never call `esp_hal::init` or take all of `Peripherals`.
- **Breaking the rules in AGENTS.md:** no `alloc` in the `no_std` path, integer math per
  pixel, and no backend logic (timing, colour order) in the facade.

## Shape

### Modules and features

`ledkit` re-exports each crate as a module, under its short name:

| Path | Crate | Feature |
|---|---|---|
| `ledkit::color` | `ledkit-color` | always |
| `ledkit::core` | `ledkit-core` | always |
| `ledkit::layout` | `ledkit-layout` | `layout` (default) |
| `ledkit::anim` | `ledkit-anim` | `anim` (default; turns on `layout`) |
| `ledkit::text` | `ledkit-text` | `text` (default; turns on `anim`) |
| `ledkit::spi` | `ledkit-spi` | `spi` |
| `ledkit::uart` | `ledkit-uart` | `uart` |
| `ledkit::esp` | `ledkit-esp` + helpers | `esp32c6` or `esp32s3` (also `esp32`, `esp32c3`, `esp32s2`, re-exports only) |
| `ledkit::esp_idf` | `ledkit-esp-idf` + helpers | `esp-idf` |
| `ledkit::nrf` | `ledkit-nrf` | `nrf52840` |

Pass-through features: `smart-leds`, `embedded-graphics` (to `ledkit-layout`), and `async`.

*Changed while building:*

- **No `ledkit::rp` yet.** rp235x-hal 0.4 needs `riscv-rt` 0.12 and esp-hal 1.2 needs
  0.16. `riscv-rt` declares `links`, so Cargo allows one version per dependency graph,
  and it resolves optional dependencies too, so even behind a feature the two can't share
  the facade. Pico projects add `ledkit-rp` next to `ledkit`; the README says so.
- **`async` only turns on `ledkit-nrf/async`.** The ESP, SPI and serial backends have
  async built in, and the esp-hal `strip!(.., async)` needs no feature.
- **No extra `compile_error!`** for two chip features: esp-hal already refuses two chips,
  with its own message.

### The prelude

`ledkit::prelude` is the union of the crate preludes for whatever features are on: the chip
types, `Strip`, `Rgb`/`Rgbw`/`Hsv`/`Hue`, `Brightness`, `Milliamps`, the layout types
(`MatrixLayout`, `Corner`, `Canvas`), the animation prelude (effects, `AnimationExt`,
`Time`), the text prelude (`Font`, `Size`, `Edges`, `TextArea`, `Ticker`), and the frame
loop types. Backend output types (`RmtOutput`, `ClockedSpi`, …) stay in their modules, not
in the prelude: a program names its backend once, at setup.

Where two crate preludes export the same name, the facade's prelude lists items explicitly
so there is exactly one of each.

### ESP board helpers

Both flavours give back an ordinary `Strip<C, N, RmtOutput<…>>`. The long form is what
the helper does, and each helper's docs show it.

**esp-hal (`ledkit::esp`).** The awkward part today is the pulse buffer: its length,
`buffer_len::<C>(N)`, can't be computed from `N` inside a generic function on stable Rust.
So the helper is a macro, which can write that constant expression in place (my call):

```rust
use ledkit::prelude::*;

let peripherals = esp_hal::init(esp_hal::Config::default());
let mut strip = ledkit::esp::strip!(Ws2812b, 30, peripherals.RMT, peripherals.GPIO8)?;
```

It creates the RMT at 80 MHz, takes channel 0, sizes the buffer and returns
`Result<Strip<Ws2812b, 30, _>, ledkit::esp::Error>`. Variants:

- `strip!(.., async)` makes the RMT async (`Rmt::new(..).into_async()`) for
  `show_async().await`.
- A function form for users who already have an `Rmt` (to use its other channels):
  `ledkit::esp::strip_on(rmt.channel1, pin, rmt_hz, buffer)`, where `buffer` comes from a
  second macro, `ledkit::esp::pulse_buffer!(Ws2812b, 30)`. This keeps the RMT in the
  user's hands.

**ESP-IDF (`ledkit::esp_idf`).** `RmtOutput::new(pin)` already needs nothing else (ESP-IDF
picks the channel and allocates the buffer), so these are plain functions:

```rust
let mut strip = ledkit::esp_idf::strip::<Ws2812b, 30>(peripherals.pins.gpio8)?;
let mut strip = ledkit::esp_idf::strip_with_dma::<Ws2812b, 30>(peripherals.pins.gpio5)?; // S3
```

The ESP-IDF backend is blocking only today; async for it is out of scope here.

**DotStars on ESP.** DotStars need SPI, not the RMT. The SPI setup is plain esp-hal and
the user writes it, as now; the helper for it is `ledkit::spi::ClockedSpi::new(spi)` (no
change). A board helper for ESP SPI is a candidate for later.

### The frame loop and clock

A **`Clock`** is anything that can say how long it has been running, as a `Time`. ledkit
provides it for the platforms in this round:

- `ledkit::esp::Clock` from `esp_hal::time::Instant`,
- `ledkit::esp_idf::Clock` from `std::time::Instant`,
- *(changed while building)* no separate Embassy clock: the esp-hal `Clock` works in
  async code too, and the async delay can be an esp-hal `OneShotTimer` made async, or
  `embassy_time::Delay` under Embassy.

**`Frames`** turns a clock and a delay into a steady loop: each step waits until the next
frame is due, then yields the time.

```rust
let mut frames = Frames::new(clock, delay).per_second(60);  // 60 is the default
loop {
    let now = frames.next_frame();          // blocks until the next frame is due
    rainbow.render(&mut strip, now);
    strip.show()?;
}
```

`FramesAsync` is the same, with `frames.next_frame().await` and an
`embedded-hal-async` delay. Waiting is to the next *deadline*, not a fixed sleep, so the
frame rate holds while drawing takes longer or shorter.

The first frame comes at once, at time 0. If drawing falls more than a whole frame
behind, the missed frames are skipped, not rushed. `restart()` starts the time from 0 again
(for scenes), and `delay()` lends out the delay (for a pause). Frame rates are clamped to
1–1000; 60 fps averages exactly (16, 17, 17 ms ...).

Hand-written stays first-class: `clock.now()` gives a `Time` on its own, any closure
returning a `Time` is a `Clock`, and the existing pattern (`Time::from_millis(..)` and a
delay) keeps working.

Where it lives (my call): `Clock`, `Frames` and `FramesAsync` go in `ledkit-anim`'s `time`
module next to `Time`, using `embedded-hal`/`embedded-hal-async` `DelayNs`, so users of
the separate crates get them too. The per-platform `Clock`s live in the facade's `esp` /
`esp_idf` modules.

### Errors

Helpers return the backend's own error type (`ledkit::esp::Error`,
`ledkit::esp_idf::Error`), so `?` works in a `main` that returns that error. The facade
adds no error type of its own.

## Where the crate lives and how it's checked

- `crates/ledkit/`, like the others: `Cargo.toml`, README (which is also the crates.io
  landing page and the `lib.rs` docs), `LICENSE-*`, `NOTICE`.
- It depends on crates outside the root workspace (`ledkit-esp`, `-esp-idf`, `-rp`,
  `-nrf`), which only build for their targets. Like them, it is excluded from the root
  workspace (my call), so `cargo test --workspace --all-features` stays host-only. CI
  tests its host features (`layout`, `anim`, `text`, `spi`, `uart`, `smart-leds`,
  `embedded-graphics`) on the host and builds and lints the ESP features through
  examples.
- docs.rs: built for `riscv32imac-unknown-none-elf` with `esp32c6` and `async`, so the
  ESP helpers show.

## Examples and testing

- New examples `examples/esp32c6-start` (esp-hal; blocking, or async with
  `--features async`) and `examples/esp32s3-start-idf` (ESP-IDF) written against `ledkit`
  and the HAL only: a strip helper, `Frames`, and one effect.
- `esp32s3-text` moves to the facade, as the proof that a real program shrinks.
- The owner checks the new examples by eye on the C6 DevKit and the S3 Feather, recorded as
  in [hardware/test-plan.md](../hardware/test-plan.md).
- Host tests: the `Frames` deadline logic, with a fake clock and delay.

## Success

- A first program needs `ledkit` plus the HAL in `Cargo.toml` and one `use` line.
- `esp32s3-text`'s setup shrinks to a few lines, and the rest of the file is the scenes.
- Each helper's docs show its long form, and swapping in that long form compiles.
- The new examples light up on the C6 and S3, blocking and async.

## Calls made while building

- The prelude is the union of the crate preludes (glob re-exports of the same item don't
  clash), plus `Pixel` and the layout types; `ledkit-layout` has no prelude of its own.
- The macros are defined as `__ledkit_esp_strip!` / `__ledkit_esp_pulse_buffer!` (hidden)
  and re-exported as `ledkit::esp::strip!` / `pulse_buffer!`, so they appear only there.
- `Frames::next_frame` returns a plain `Time`.
- `strip!` keeps the pulse buffer inside the strip (on the stack, ~100 bytes per RGB LED);
  `strip_on` takes any buffer, including a `&'static mut` one, for long strips.
- CI: a `ledkit` job (host tests and clippy, C6 clippy and docs, an nRF build) and an
  `example-esp32c6-start` job; `esp32s3-start-idf` joins the ESP-IDF S3 job.
- `esp32s3-text` now uses `ledkit` (features `esp32s3`, `spi`) and `Frames`; it ran on the
  S3 Feather after the change (scenes advanced on time over serial, 2026-11-10).
- On hardware (2026-11-11): the `esp32c6-start` program, built for the S3 Feather's
  onboard NeoPixel, passes blocking and async (owner, by eye). Results are kept in
  `crates/ledkit/README.md`'s "Tested on hardware" table.
