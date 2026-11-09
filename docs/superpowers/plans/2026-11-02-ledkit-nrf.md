# ledkit-nrf Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** drive WS2812-type LEDs from the nRF52840's PWM sequencer through EasyDMA (blocking and async), with XIAO nRF52840 examples for PWM and SPIM.

**Architecture:** `ledkit-core::encode` gains an equal-period search and nRF52 PWM duty-word helpers (host-tested, `const`). A new crate `ledkit-nrf` (outside the root workspace, like `ledkit-esp`) wraps `embassy-nrf` 0.11's `SequencePwm` in `PwmOutput`, implementing `Output` and `OutputAsync`. Two standalone example projects exercise it and `ledkit-spi` / `ledkit-uart` on the same pin.

**Tech Stack:** Rust stable, `thumbv7em-none-eabihf`, `embassy-nrf` 0.11 (`nrf52840`), `embassy-time` 0.5, `embassy-executor`, `ledkit-core`.

**Spec:** `docs/superpowers/specs/2026-11-02-ledkit-nrf-design.md`

## Global Constraints

- `no_std`, no `alloc`, no `unsafe` in library code; every public item has docs and an example; tests in the module they test.
- Backends never re-implement colour order, brightness, gamma or timing math (`AGENTS.md`).
- PWM clock 16 MHz (`embassy_nrf::pwm::PWM_CLK_HZ`), prescaler `Div1`, `CounterMode::Up`, `SequenceLoad::Common`, idle level low, `OutputDrive::HighDrive`.
- Duty word bit 15 always set; reset word `0x8000`.
- One sequence per frame, at most 32 767 words.
- Shared-file edits (`AGENTS.md`, roadmap, `ci.yml`, `encode/mod.rs`, `error.rs`) stay small and appended, to merge cleanly with `ledkit-rp`.
- Before committing: `cargo test --workspace --all-features`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-features --all-targets -- -D warnings`, `cargo build -p ledkit-core -p ledkit-color --target thumbv6m-none-eabi` and the same for `riscv32imc-unknown-none-elf`, plus each example's `cargo build --release` and clippy.

## Review Focus

1. Second and later frames: `LOOPSDONE` must be cleared before each start, or `write` returns at once and frames are cut short. Owned by Task 3 (the code path) and checked on hardware by the back-to-back round.
2. Zero LEDs: a frame of only reset words must still play and return. Task 2 test `fill_pwm_words_with_no_wire_bytes_is_only_reset`.
3. Buffer too small and frame over 32 767 words: errors before touching the PWM. Task 2 test `buffer_too_small_reports_exact_need`; Task 3 maps `FrameTooLong`.
4. A chip/clock with no equal-period fit (e.g. 1 MHz): `TimingError::EqualPeriodUnusable`, not a panic. Task 1 test `too_slow_a_clock_has_no_equal_period`.
5. RGBW chips: 32 bits per LED. Task 2 test `buffer_len_counts_rgbw`.

---

### Task 1: Equal-period search in ledkit-core

**Files:** Create `crates/ledkit-core/src/encode/equal_period.rs`; modify `crates/ledkit-core/src/encode/mod.rs` (one `mod` + one `pub use`), `crates/ledkit-core/src/error.rs` (one variant + Display arm).

**Produces:** `pub struct EqualPeriodTicks { pub period: u32, pub zero_high: u32, pub one_high: u32 }` (Copy, Debug, PartialEq, Eq); `impl ClocklessTiming { pub const fn equal_period_ticks(&self, ticks_per_second: u32, max_period: u32) -> Result<EqualPeriodTicks, TimingError> }`; `TimingError::EqualPeriodUnusable { ticks_per_second: u32, max_period: u32 }` with Display "a clock of {MHz} can't give this LED's 0 and 1 bits the same length (up to {max_period} ticks); try a faster clock".

Search: for each period `P` in 2..=max_period, each `zh`, `oh` in 1..P with all four phases inside their windows (exact integer comparison, as `phase_ticks` does), minimise the key (sum of |zh_ns − typical| + |oh_ns − typical| in picoseconds, |P_ns − typical period| where typical period = (zh+zl+oh+ol typicals)/2, P). Bound each inner loop by the window so it stays cheap in `const`.

- [ ] Tests (in the new file): `ws2812b_at_16mhz` → `{22, 6, 13}`; `ws2812_ws2813_ws2811_at_16mhz` → `{20,6,11}`, `{20,6,14}`, `{20,4,10}`; `sk6812_at_16mhz` → `{19,5,10}`; `slow_ws2811_at_16mhz` → `{40,8,19}`; `apa106_at_16mhz` → `{27,6,22}`; `ws2812b_at_8mhz` → `{11,3,6}`; `too_slow_a_clock_has_no_equal_period` (1 MHz → `EqualPeriodUnusable`); `max_period_is_respected` (WS2812B, max 21 → a result with period ≤ 21 or the error); `works_at_compile_time` (a `const` binding); `zero_clock_is_an_error`; Display test in `error.rs`.
- [ ] Run `cargo test -p ledkit-core equal_period` → fail, implement, → pass.

### Task 2: nRF52 PWM duty-word helpers

**Files:** Create `crates/ledkit-core/src/encode/pwm.rs`; modify `encode/mod.rs`; one line in `AGENTS.md`'s `encode` bullet.

**Consumes:** `EqualPeriodTicks`. **Produces:** `pub const PWM_RESET_WORD: u16 = 0x8000`; `pub const fn pwm_bit_word(high_ticks: u32) -> u16` (`0x8000 | min(high, 0x7FFF)`); `pub const fn pwm_reset_words(reset_us: u32, period: u32, ticks_per_second: u32) -> usize` (ceil, at least 1); `pub const fn pwm_word_count(byte_count: usize, reset_words: usize) -> usize` (`byte_count*8 + reset_words`); `pub fn fill_pwm_words(wire: impl IntoIterator<Item = u8>, ticks: EqualPeriodTicks, reset_words: usize, out: &mut [u16]) -> Result<usize, BufferTooSmall>` (MSB first; counts all bytes on overflow so `needed` is exact, like `fill_rmt_words`).

- [ ] Tests: `bit_word_sets_polarity` (`pwm_bit_word(13) == 0x800D`); `reset_word_is_low_all_period`; `ws2812b_reset_is_204_words`; `sk6812_reset_is_68_words`; `fill_one_byte` (`0b1000_0000` → `[0x800D, 0x8006 ×7, reset…]`); `fill_pwm_words_with_no_wire_bytes_is_only_reset`; `buffer_too_small_reports_exact_need`.
- [ ] Fail → implement → pass; commit Tasks 1–2: `core: equal-period search and nRF52 PWM duty words`.

### Task 3: crate `ledkit-nrf`

**Files:** Create `crates/ledkit-nrf/{Cargo.toml,README.md,LICENSE-MIT,LICENSE-APACHE,NOTICE,src/lib.rs,src/error.rs,src/smart_leds.rs}`; root `Cargo.toml` `exclude` gains `"crates/ledkit-nrf"`.

**Consumes:** Tasks 1–2. **Produces:**
- `pub const fn buffer_len<C: Chip>(led_count: usize) -> usize` (words; clocked chip is a build error).
- `pub struct PwmOutput<'d, C: Chip, B>`; `pub fn new<T: embassy_nrf::pwm::Instance>(pwm: Peri<'d, T>, pin: Peri<'d, impl GpioPin>, buffer: B) -> Result<Self, Error>`.
- `impl<C, B: AsMut<[u16]>> Output<C>`, and `OutputAsync<C>` behind feature `async` (default; pulls `embassy-time`).
- `pub enum Error { Timing(TimingError), BufferTooSmall(BufferTooSmall), FrameTooLong { words: usize, max: usize }, Pwm(embassy_nrf::pwm::Error) }`, `#[non_exhaustive]`, `Display`.
- Features: `nrf52840` (default-off pass-through, required), `async`, `smart-leds`.

Send path: compute words with `fill_pwm_words`; if `> 32_767` → `FrameTooLong`; take `loops_done = pwm.event_loops_done()` once in `new` and store it; per frame: `loops_done.clear()`, `SingleSequencer::new(&mut pwm, &buf[..used], SequenceConfig::default())`, `start(SingleSequenceMode::Times(1))`, wait for `loops_done.is_triggered()` (spin, or `Timer::after(micros(used * period / 16))` then `yield_now` loop), drop the sequencer (stops the PWM).

- [ ] Docs in the style of `ledkit-esp` (quick tour, sizing, reset gap, pins for the XIAO, async note, don't-cancel note, empty "Tested on hardware" table); README same content.
- [ ] Verify via Task 4's build (the crate only builds for an nRF target): `cargo build` and clippy in the example.

### Task 4: example `nrf52840-pwm`

**Files:** `examples/nrf52840-pwm/{Cargo.toml,.cargo/config.toml,memory.x,build.rs,rust-toolchain.toml,.gitignore,README.md,src/main.rs}`.

`memory.x`: FLASH `0x27000` length `0xED000 - 0x27000`; RAM `0x20000000` length 256K. Embassy executor; HFXO on; data pin D10 = `P1_15`; onboard LED red `P0_26`, green `P0_30`, blue `P0_06` (active low). Rounds per the spec (blocking; async with a 10 kHz pin-toggling task; 1–150 sweep; rainbow forever). `defmt` not used (no probe assumed); `panic-halt`.

- [ ] `cargo build --release` and `cargo clippy --release -- -D warnings` clean in the example dir.

### Task 5: example `nrf52840-spi`

Same layout. SPIM3 on D10 (MOSI) with SCK on an unused pin (D8 = `P1_13`); `ClocklessSpi::new_packed` at 4 MHz blocking and async (embassy-nrf `Spim` implements `embedded-hal` 1.0 and `-async`); `new` at 4 MHz; UARTE0 at 1 Mbaud: `ClocklessUart::new` must return `Err(TimingError::UartBaudUnusable{..})` → magenta. README as Task 4.

- [ ] Build and clippy clean.

### Task 5b: example `nrf52840-dotstar` (owner request)

Same layout as Task 5. `ClockedSpi::<LedType, _>::new(spim)` on SPIM3, MOSI D10 (`P1_15`), SCK D8 (`P1_13`). Settings: `type LedType = Apa102; const LED_COUNT: usize = 64;`. Rounds: for each of `M1, M2, M4, M8, M16, M32` (embassy-nrf `spim::Frequency`; recreate the `Spim` per step) show red/green/blue/white and counting; HD brightness steps 5, 10, 15, 20, 30, 40, 60 %; rainbow at 8 MHz forever. Onboard LED blinks the step number before each clock step; red = error. README: two-channel 74AHCT125 wiring, what you'll see, pending hardware test.

- [ ] Build and clippy clean.

### Task 6: CI, crate map, roadmap; final checks; commit

- `ci.yml`: jobs `example-nrf52840-pwm`, `example-nrf52840-spi`, `example-nrf52840-dotstar` appended (target add, fmt, clippy, build `--release --locked`).
- `AGENTS.md` crate-map row, "exist so far", "outside the workspace" paragraph; `CONTRIBUTING.md` one line; roadmap row 5b and notes.
- [ ] Run every command in Global Constraints; commit on branch `nrf`.
