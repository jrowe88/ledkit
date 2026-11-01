# ledkit-uart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drive one-wire (NeoPixel-family) LEDs from an inverted hardware serial port, with
the bit pattern computed from each chip's timing, on the Teensy 4 and the ESP32-C6.

**Architecture:** `ledkit-core` gets a pure, `const` UART encoding next to the SPI one
(`encode/uart.rs`). A new workspace crate, `ledkit-uart`, wraps any `embedded-io` 0.7 serial
port plus a delay in `ClocklessUart`, which implements `Output` and `OutputAsync` the way
`ledkit-spi`'s `ClocklessSpi` does. Two examples set up the real ports: `esp32c6-uart` and
`teensy4-uart` (blocking and DMA).

**Tech Stack:** Rust 2021, `no_std`; `embedded-io` / `embedded-io-async` 0.7,
`embedded-hal` / `embedded-hal-async` 1.0 (`DelayNs`); examples on esp-hal 1.2 (`unstable`)
and `teensy4-bsp` 0.6 / `imxrt-hal` 0.6.

**Spec:** `docs/superpowers/specs/2026-11-01-ledkit-uart-design.md`

## Global Constraints

- No `unsafe` (`unsafe_code = "forbid"` in the workspace); `no_std`; no heap.
- Workspace MSRV 1.82; clippy pedantic clean with the workspace lint settings; `missing_docs`.
- Docs in the project's plain style (see `crates/ledkit-spi` for tone and layout).
- `embedded-io` and `embedded-io-async` **0.7** (what `imxrt-hal` 0.6 implements).
- Teensy: `teensy4-bsp` 0.6, `imxrt-hal` 0.6. C6: esp-hal 1.2 with `unstable`.
- Two LED bits per serial byte, 5 slots each, inverted 8N1 (start bit high, stop bit low,
  data least significant bit first). Never send a pattern outside the chip's windows.
- Board setup (pin, baud, inversion, DMA) lives in the examples, never in `ledkit-uart`.

## Review Focus

- **A baud the port really runs at that differs from the one asked for** (HAL rounding): the
  caller passes the real baud; one that can't time the chip must fail at `new` with the
  nearest working bauds, never send. Pinned in Task 1 (`error_names_working_neighbours`).
- **A buffer that's too small:** `write` returns `BufferTooSmall` and sends nothing to the
  port. Pinned in Task 2 (`too_small_buffer_sends_nothing`).
- **A port that accepts only part of a write:** the whole frame still goes out, in order.
  Pinned in Task 2 (`partial_writes_send_the_whole_frame`).
- **Back-to-back frames:** every frame is followed by its own full reset wait, so the second
  frame is never glued onto the first. Pinned in Task 2 (`every_frame_waits_the_reset_time`).
- **A port error mid-frame:** the error comes back, and no reset wait or flush follows.
  Pinned in Task 2 (`port_error_is_returned`).

---

### Task 1: The UART encoding in `ledkit-core`

**Files:**
- Create: `crates/ledkit-core/src/encode/uart.rs`
- Modify: `crates/ledkit-core/src/encode/mod.rs` (add `mod uart; pub use uart::UartEncoding;`)
- Modify: `crates/ledkit-core/src/encode/spi.rs` (move `window_ok` and `miss_ps` to
  `encode/mod.rs` as `pub(crate)` so both encodings share them; rename the `spi_hz`
  parameter to `hz`)
- Modify: `crates/ledkit-core/src/error.rs` (new `TimingError` variant and its message)
- Test: in-file `#[cfg(test)]` modules of `uart.rs` and `error.rs`

**Interfaces:**
- Produces (in `ledkit_core::encode`, re-exported the way `SpiEncoding` is):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartEncoding {
    /// High slots (of 5) for a `0` LED bit.
    pub zero_high_slots: u8,
    /// High slots (of 5) for a `1` LED bit.
    pub one_high_slots: u8,
}
impl UartEncoding {
    pub const SLOTS_PER_LED_BIT: u8 = 5;
    /// The data byte (as the port is told to send it, before inversion) for two LED bits.
    pub const fn byte_for(&self, first: bool, second: bool) -> u8;
    /// Serial bytes for `wire_bytes` LED wire bytes: 4 per wire byte, no reset bytes.
    pub const fn buffer_len(&self, wire_bytes: usize) -> usize;
    pub fn encode(&self, wire: impl IntoIterator<Item = u8, IntoIter: ExactSizeIterator>, out: &mut [u8]) -> Result<usize, BufferTooSmall>;
    pub fn encode_from_iter(&self, wire: impl IntoIterator<Item = u8>, out: &mut [u8]) -> Result<usize, BufferTooSmall>;
}
impl ClocklessTiming {
    pub const fn uart_encoding(&self, baud: u32) -> Result<UartEncoding, TimingError>;
}
// in TimingError (#[non_exhaustive]):
UartBaudUnusable { baud: u32, nearest_lower_baud: Option<u32>, nearest_higher_baud: Option<u32> },
```

`byte_for` rule: slots 0..=9 on the wire are start, d0..d7, stop. LED bit A is slots 0–4 and
bit B slots 5–9; a bit with `k` high slots is high for its first `k` slots. Data bit `i` is
the **inverse** of wire slot `i + 1`. Encoding goes most significant LED bit first, two LED
bits per serial byte, so one wire byte becomes 4 serial bytes.

`uart_encoding` search: all pairs `1 <= k0 < k1 <= 4`; a pair qualifies when the four
pulses (`k0`, `5 - k0`, `k1`, `5 - k1` slots at `baud`) fit `zero_high`, `zero_low`,
`one_high`, `one_low` exactly (`window_ok`); the smallest total `miss_ps` wins, ties to the
smaller `k0` then `k1`. Errors carry the nearest qualifying bauds on a 100 kHz grid from
1.0 to 6.0 Mbaud. Baud 0 never qualifies.

- [ ] **Step 1: Write the failing tests** in `uart.rs`:

```rust
#[test]
fn ws2812b_at_4_mbaud() {
    let e = ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000).unwrap();
    assert_eq!((e.zero_high_slots, e.one_high_slots), (1, 3));
}
#[test]
fn byte_table_for_ws2812b() {
    let e = ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000).unwrap();
    assert_eq!(e.byte_for(false, false), 0xEF);
    assert_eq!(e.byte_for(false, true), 0x8F);
    assert_eq!(e.byte_for(true, false), 0xEC); // the spec's worked example
    assert_eq!(e.byte_for(true, true), 0x8C);
}
#[test]
fn encodes_msb_first_two_bits_per_byte() {
    let e = ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000).unwrap();
    let mut out = [0u8; 4];
    assert_eq!(e.encode([0b1010_0000u8], &mut out), Ok(4));
    assert_eq!(out, [0xEC, 0xEC, 0xEF, 0xEF]);
}
#[test]
fn rgbw_is_16_bytes_per_led() {
    let e = ClocklessTiming::of::<Sk6812Rgbw>().uart_encoding(4_000_000).unwrap();
    assert_eq!(e.buffer_len(4), 16);
}
#[test]
fn encode_reports_a_small_buffer() {
    let e = ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000).unwrap();
    let mut out = [0u8; 7];
    assert_eq!(e.encode([1u8, 2], &mut out), Err(BufferTooSmall { needed: 8 }));
    assert_eq!(e.encode_from_iter([1u8, 2], &mut out), Err(BufferTooSmall { needed: 8 }));
}
#[test]
fn encode_and_encode_from_iter_agree() { /* same bytes for [0x00, 0xFF, 0x5A] */ }
#[test]
fn every_pattern_fits_the_chip_windows() {
    // For Ws2812b, Sk6812Rgbw, Ws2811, Ws2811Slow, Ws2813, Apa106 and every grid baud
    // (1.0..=6.0 Mbaud, 100 kHz steps) where uart_encoding is Ok: each of the four pulse
    // lengths in ns (slots * 1e9 / baud) lies in its phase's [min_ns, max_ns].
}
#[test]
fn every_clockless_chip_has_a_working_baud() {
    // Every clockless chip in the catalogue: some grid baud returns Ok.
}
#[test]
fn error_names_working_neighbours() {
    let t = ClocklessTiming::of::<Ws2812b>();
    match t.uart_encoding(1_000_000) {
        Err(TimingError::UartBaudUnusable { baud: 1_000_000, nearest_higher_baud: Some(hi), .. }) => {
            assert!(t.uart_encoding(hi).is_ok());
            assert!(t.uart_encoding(hi - 100_000).is_err());
        }
        other => panic!("{other:?}"),
    }
    assert!(t.uart_encoding(0).is_err());
}
#[test]
fn works_at_compile_time() {
    const LEN: usize = match ClocklessTiming::of::<Ws2812b>().uart_encoding(4_000_000) {
        Ok(e) => e.buffer_len(30 * 3),
        Err(_) => panic!(),
    };
    assert_eq!(LEN, 360);
}
```

And in `error.rs`, a message test: `UartBaudUnusable { baud: 4_100_000, nearest_lower_baud: Some(4_000_000), nearest_higher_baud: None }` displays text containing `"4.1 Mbaud"` and `"4.0 Mbaud"` (the `Mhz` formatter's number style); with no neighbours it contains `"no serial baud works for this LED"`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p ledkit-core uart`
Expected: compile errors (no `uart_encoding`, no `UartEncoding`).

- [ ] **Step 3: Implement** `encode/uart.rs`, the shared helpers move, and the error variant
with its `Display` (reuse the `Mhz` formatter's number style with the unit `Mbaud`). Doc
comments with runnable examples on `UartEncoding` and `uart_encoding`, in the style of
`SpiEncoding`, including a "sizing a buffer at compile time" example.

- [ ] **Step 4: Run the tests and the workspace checks**

Run: `cargo test -p ledkit-core` and `cargo clippy --workspace --all-features --all-targets -- -D warnings`
Expected: all pass, no warnings. The existing SPI tests still pass after the helper move.

- [ ] **Step 5: Commit**

```bash
git add crates/ledkit-core
git commit -m "ledkit-core: UART encoding for one-wire LEDs (2 LED bits per serial byte)"
```

---

### Task 2: The `ledkit-uart` crate

**Files:**
- Create: `crates/ledkit-uart/Cargo.toml`, `src/lib.rs`, `src/clockless.rs`, `src/error.rs`,
  `src/smart_leds.rs`, `src/testing.rs`, `README.md`
- Modify: `.github/workflows/ci.yml` (add `ledkit-uart` to the no_std build lines for
  `thumbv6m-none-eabi` and `riscv32imc-unknown-none-elf`, next to `ledkit-spi`'s)
- Test: in-file `#[cfg(test)]` modules (with fakes in `testing.rs`, as in `ledkit-spi`)

**Interfaces:**
- Consumes: `ClocklessTiming::uart_encoding`, `UartEncoding::{encode_from_iter, buffer_len}`,
  `TimingError` (Task 1); `ledkit_core::{wire::clockless_bytes, Chip, Output, OutputAsync}`.
- Produces:

```rust
pub struct ClocklessUart<C: Chip, U, D, B> { /* uart, delay, buffer, encoding, reset_us */ }
impl<C: Chip, U, D, B: AsMut<[u8]>> ClocklessUart<C, U, D, B> {
    /// `baud` is the rate the port really runs at.
    pub fn new(uart: U, baud: u32, delay: D, buffer: B) -> Result<Self, TimingError>;
    pub fn encoding(&self) -> UartEncoding;
    pub fn port(&self) -> &U;
    pub fn release(self) -> (U, D, B);
}
impl Output<C> for ClocklessUart<C, U, D, B>      // U: embedded_io::Write, D: embedded_hal::delay::DelayNs
impl OutputAsync<C> for ClocklessUart<C, U, D, B> // U: embedded_io_async::Write, D: embedded_hal_async::delay::DelayNs
#[non_exhaustive] pub enum Error<E> { Uart(E), BufferTooSmall(BufferTooSmall) }
pub const fn buffer_len<C: Chip>(led_count: usize, baud: u32) -> usize; // panics (at compile time when const) on an unusable baud
// feature "smart-leds": SmartLedsWrite + SmartLedsWriteAsync for ClocklessUart
```

A frame: encode into the buffer, `write_all` the used bytes, `flush`, then
`delay.delay_us(reset_us)` with the chip's `ClocklessTiming::reset_us`. The wait is at the
end of each frame and the delay is required (my calls, per the spec's builder list). An
empty frame writes nothing but still flushes and waits.

`Cargo.toml` mirrors `ledkit-spi`'s: workspace package fields, `[lints] workspace = true`,
deps `ledkit-core`, `embedded-io = "0.7"`, `embedded-io-async = "0.7"`, `embedded-hal = "1.0"`,
`embedded-hal-async = "1.0"`, optional `smart-leds-trait`, dev-dep `embassy-futures`.

- [ ] **Step 1: Write the failing tests** (fakes: `FakePort` records written bytes, a
flush count, can accept at most `n` bytes per `write`, and can fail on the k-th write;
`FakeDelay` records each `delay_us` value; async twins of both):

```rust
#[test] fn sends_the_encoded_frame_then_flushes_and_waits() {
    // Strip<Ws2812b, 2, _> with one red LED at full brightness through ClocklessUart at 4 Mbaud:
    // port bytes == UartEncoding::encode of wire::clockless_bytes of the frame,
    // flushes == 1, delays == [280].
}
#[test] fn every_frame_waits_the_reset_time() { /* two show() calls: delays == [280, 280], bytes are two whole frames */ }
#[test] fn partial_writes_send_the_whole_frame() { /* port takes 3 bytes per write: same bytes as an unlimited port */ }
#[test] fn too_small_buffer_sends_nothing() {
    // buffer of buffer_len(2 LEDs) - 1 bytes: Err(Error::BufferTooSmall(_)), no bytes, no flush, no delay
}
#[test] fn port_error_is_returned() { /* write fails: Err(Error::Uart(_)), no flush, no delay */ }
#[test] fn unusable_baud_fails_at_new() { /* new(.., 1_000_000, ..) for Ws2812b: Err(TimingError::UartBaudUnusable { .. }) */ }
#[test] fn rgbw_frames() { /* Sk6812Rgbw, 2 LEDs: 32 bytes sent */ }
#[test] fn empty_frame_flushes_and_waits() { /* 0 LEDs: no bytes, flushes == 1, delays == [280] */ }
#[test] fn async_matches_blocking() { /* same frame through OutputAsync (embassy_futures::block_on): identical bytes, flush, delay */ }
#[test] fn buffer_len_is_const() { const N: usize = buffer_len::<Ws2812b>(30, 4_000_000); assert_eq!(N, 360); }
#[test] fn error_messages_are_readable() { /* as in ledkit-spi's error.rs: "serial port error: 7" */ }
// feature smart-leds: smart_leds_write_matches_output() for blocking and async
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p ledkit-uart --all-features`
Expected: compile errors (crate empty).

- [ ] **Step 3: Implement** the crate. Crate docs and `README.md` in `ledkit-spi`'s style:
what it is; wiring (serial transmit pin, inverted, through a 74AHCT125 to DIN, common
ground); the baud table per chip from the spec; why the port must be inverted; the reset
wait; the note that pauses between bytes only stretch the low part (harmless up to about
4.5 µs on a WS2812B); a "Tested on hardware" table with both boards marked untested for now.

- [ ] **Step 4: Run the tests and checks**

Run: `cargo test --workspace --all-features`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo build -p ledkit-uart --all-features --target thumbv6m-none-eabi`, `cargo doc -p ledkit-uart --no-deps` with `RUSTDOCFLAGS=-D warnings`
Expected: all pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/ledkit-uart .github/workflows/ci.yml Cargo.lock
git commit -m "ledkit-uart: drive one-wire LEDs from an inverted serial port"
```

---

### Task 3: `esp32c6-uart` example

**Files:**
- Create: `examples/esp32c6-uart/` (`Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`,
  `rust-toolchain.toml`, `.gitignore`, `src/main.rs`, `README.md`), modelled on
  `examples/esp32c6-spi`
- Modify: `.github/workflows/ci.yml` (job `example-esp32c6-uart`: `cargo build --release
  --locked` and `cargo clippy --release --locked -- -D warnings`)

**Interfaces:**
- Consumes: `ClocklessUart::new`, `buffer_len` (Task 2).

Settings at the top of `main.rs`: `LED_COUNT = 100`, `LedType = Ws2812b`,
`BAUD = 4_000_000`, `BRIGHTNESS = 20%`, `POWER_LIMIT = 500 mA`. Transmit on **GPIO4**,
inverted with `OutputSignal::with_output_inverter(true)` (esp-hal `unstable`), on `UART1`
with `Config::default().with_baudrate(BAUD)` (8N1 default). Delay: `esp_hal::delay::Delay`.
Two modes in one loop, as in the long-strip tests: blocking (`UartTx`), then async
(`UartTx::into_async`, driven with `embassy_futures::block_on`, with a small adapter
implementing `embedded_hal_async::delay::DelayNs` over the blocking `Delay`). Each mode
shows red, green, blue, white (1 s each), counting (LED 0 red, every 10th white, last blue,
5 s), and a fast rainbow (50 frames a second, 10 s), printing each step over serial.
If esp-hal reports the real baud (`Config`/driver), pass that to `ClocklessUart::new`.

- [ ] **Step 1: Write the example and its README** (wiring table: GPIO4 → 74AHCT125 1A, 1Y →
strip DIN, OE to GND, 5 V and common ground; one-time setup; run; expected serial output;
troubleshooting: wrong colors, nothing lights, a stray first LED at power-up).

- [ ] **Step 2: Build and lint**

Run (in `examples/esp32c6-uart`): `cargo build --release --locked` then `cargo clippy --release --locked -- -D warnings`
Expected: both succeed.

- [ ] **Step 3: Hardware check (owner)**

Flash with `espflash flash --port COM11 --chip esp32c6 target/riscv32imac-unknown-none-elf/release/esp32c6-uart`
on the C6 with the 100-LED strip on GPIO4. Pass: in both modes the colors are right, LED 99
is blue in counting, the rainbow is clean, and LED 0 shows no stray color at start-up. If
LED 0 flickers at start-up, route the pin after inversion is set and re-test.

- [ ] **Step 4: Record and commit**

Add the result to `crates/ledkit-uart/README.md` and its crate docs ("Tested on hardware").

```bash
git add examples/esp32c6-uart .github/workflows/ci.yml crates/ledkit-uart
git commit -m "examples: esp32c6-uart, a 100-LED strip from an inverted serial port"
```

---

### Task 4: `teensy4-uart` example

**Files:**
- Create: `examples/teensy4-uart/` (same set of files), modelled on `examples/teensy4-spi`
- Modify: `.github/workflows/ci.yml` (job `example-teensy4-uart`, as `example-teensy4-spi`
  does it, plus clippy)

**Interfaces:**
- Consumes: `ClocklessUart::new`, `buffer_len` (Task 2).

Settings: as in Task 3. Serial port 1: `LPUART6`, transmit on **pin 1** (receive pin 0
claimed, left unconnected), set up with the BSP's LPUART helper at `BAUD`; then
`lpuart.disable(|d| d.set_inversion(Direction::Tx, true))`. Pass the baud the port really
runs at (from `Lpuart::baud()` and the 24 MHz UART clock) to `ClocklessUart::new`. Delay:
`cortex_m::delay::Delay` (implements `DelayNs`). Two modes in one loop:

- **blocking:** `ClocklessUart` over the `Lpuart` (`embedded_io::Write`);
- **DMA:** a `DmaUart` adapter in the example implementing `embedded_io_async::Write`
  (`write` = `Lpuart::dma_write(channel, buf).await`, returns `buf.len()`; `flush` waits
  for transmit complete) using one DMA channel from the BSP's `Resources`, polled with
  `embassy_futures::block_on`, plus the same async-delay adapter as Task 3.

Same pattern sequence and serial-free status output (no serial console on the Teensy;
mirror `teensy4-spi`'s use of the orange LED if it has one). The README explains building
the `.hex` with `rust-objcopy` and loading it with the Teensy Loader, as `teensy4-spi`'s does.

- [ ] **Step 1: Write the example and its README** (wiring: pin 1 → 74AHCT125 1A, 1Y → DIN).

- [ ] **Step 2: Build and lint**

Run (in `examples/teensy4-uart`): `cargo build --release --locked` then `cargo clippy --release --locked -- -D warnings`
Expected: both succeed.

- [ ] **Step 3: Check the DMA buffer placement**

Confirm the frame buffer lands in DTCM (uncached) with `teensy4-rt`'s default layout (map
file or symbol address in `0x2000_0000..0x2008_0000`). If it doesn't, flush the cache
before each DMA write or move the buffer, and note why in a comment.

- [ ] **Step 4: Hardware check (owner)**

Make the hex with `rust-objcopy -O ihex target/thumbv7em-none-eabihf/release/teensy4-uart teensy4-uart.hex`,
load it with the Teensy Loader, 100-LED strip on pin 1. Pass: both modes show the right
colors, LED 99 blue in counting, a clean rainbow, no stray LED 0 at start-up. If the DMA
mode hangs, the DMA future needs its interrupt: wire up `DMA` interrupt handling and re-test.

- [ ] **Step 5: Record and commit**

```bash
git add examples/teensy4-uart .github/workflows/ci.yml crates/ledkit-uart
git commit -m "examples: teensy4-uart, blocking and DMA, on a 100-LED strip"
```

---

### Task 5: Project docs

**Files:**
- Modify: `README.md` (board table rows for both examples; crates table row for
  `ledkit-uart`; the "One API, many boards" text if it lists techniques)
- Modify: `crates/ledkit-core/README.md` and `src/lib.rs` (mention the UART encoding where
  the SPI encoding is described)
- Modify: `docs/superpowers/specs/2026-10-25-ledkit-design.md` (technique table: a row
  "Serial port (UART) | Teensy 4 LPUART, ESP32 UART, any inverted UART | 2 LED bits per
  serial byte, computed for the actual baud")
- Modify: `docs/superpowers/plans/2026-10-25-ledkit-roadmap.md` (a short "Serial-port
  technique" note: what shipped, the hardware results, parallel strips still paused)
- Modify: `CONTRIBUTING.md` only if its backend guide lists the techniques

- [ ] **Step 1: Update the docs** with the hardware results from Tasks 3 and 4.

- [ ] **Step 2: Check**

Run: `cargo test --workspace --all-features --doc` and `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS=-D warnings`
Expected: pass.

- [ ] **Step 3: Commit**

```bash
git add README.md crates/ledkit-core docs CONTRIBUTING.md
git commit -m "docs: the serial-port technique in the READMEs, design and roadmap"
```
