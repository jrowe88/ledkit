# ledkit-nrf: one-wire LEDs from the nRF52840's PWM and EasyDMA

Status: design, waiting for the owner's approval (2026-11-02). Breadth first: get the
nRF52840 covered with a working native path and the portable one, refine later.

## What and why

The nRF52's native way to drive NeoPixel-type LEDs is the PWM peripheral's sequencer: it
reads 16-bit duty words from RAM through EasyDMA and plays one per PWM period. With the
period set to one LED bit, the frame goes out with no CPU involvement, so interrupts and the
radio can't disturb it (the test plan's T8 and T11). `ledkit-spi` already runs on the
nRF's SPIM; this adds the PWM path and examples for both on the owner's XIAO nRF52840.

**Builds:** a crate `ledkit-nrf` with `PwmOutput` (blocking `Output` and async `OutputAsync`,
RGB and RGBW, any of the four PWM instances, any pin); two `ledkit-core::encode` helpers it
uses; examples `nrf52840-pwm` and `nrf52840-spi`; CI jobs, crate README, crate map, roadmap
entry. `no_std`, no heap, no `unsafe`, `smart-leds` traits behind a feature.

## Decisions

Each is my recommendation, with the reason and what I set aside.

1. **HAL: `embassy-nrf` 0.11 only.** Its `SequencePwm` has a blocking API that needs no
   executor and fits async apps too, and it does the pin, register and "buffer is in RAM"
   work, so ledkit needs no register code or `unsafe`. *Set aside:* `nrf52840-hal` (its PWM
   DMA wants `'static` buffers and has no async); can be added behind a feature later.

2. **Timing from a shared equal-period search in `ledkit-core`.** PWM's period (`COUNTERTOP`)
   is the same for every bit, so `0` and `1` need equal periods, which `PulseTicks` doesn't
   promise. New `ClocklessTiming::equal_period_ticks(hz, max_period) -> EqualPeriodTicks
   { period, zero_high, one_high }` (`const fn`, in `encode/equal_period.rs`) picks the high
   times closest to the chip's typical highs (they decide `0` from `1`), then the period
   closest to its typical bit period (keeps the lows clear of their minimums), with all four
   phases inside the datasheet windows. The RP PIO backend needs the same thing; I've told
   the `ledkit-rp` session so only one copy lands. *Set aside:* nrf-smartled's fixed 20 ticks.
   At 16 MHz the search gives 20 ticks (1.25 µs) for the WS2812, WS2813 and WS2811, 19 for the
   SK6812 and **22 (1.375 µs) for the WS2812B**, because at 20 ticks its `1` bit would be low
   for 437.5 ns, under the catalogue's 450 ns minimum.

3. **Duty words: bit 15 (polarity) always set.** With it set the line is high from the start
   of the period for `compare` ticks, so a bit is `0x8000 | high_ticks` and the reset word is
   `0x8000` (low all period). With it clear, a compare of 0 would hold the line *high*. New
   helpers in `encode::pwm` (named like the RMT ones): `pwm_bit_word`, `PWM_RESET_WORD`,
   `pwm_reset_words`, `pwm_word_count`, `fill_pwm_words`. Clock: 16 MHz, prescaler 1,
   up-counting, one channel.

4. **Reset gap: trailing zero-duty words in the frame**, `ceil(reset_us / period)` of them
   (plus one, as the PWM reports the end when the last word starts: 205 words, 410 bytes, for the WS2812B). `write` returns only after they've played, so
   back-to-back `show()` calls each latch (T2) with no timer. *Set aside:* the `ENDDELAY`
   register (no RAM, but its event timing is easy to get wrong) and a timestamp-and-wait.

5. **RAM: 2 bytes per LED bit**, 48 bytes per RGB LED, 64 per RGBW, plus the reset words.
   `const fn buffer_len::<C>(led_count)` in words for a `[u16; N]` buffer: 100 WS2812B LEDs
   = 5.2 KB, 300 = 14.8 KB (of 256 KB).

6. **Long frames: one sequence per frame**, up to 32 767 words (EasyDMA's 15-bit `CNT`):
   about 1350 RGB or 1020 RGBW LEDs. Longer is a clear `Error::FrameTooLong`. *Follow-up:*
   use both sequences to double it, or ping-pong refill for unlimited length.

7. **Blocking:** fill words, clear the PWM's `LOOPSDONE` event (embassy-nrf clears it only
   once, at setup), start the sequence once (`SingleSequencer`, `Times(1)`), spin on
   `LOOPSDONE`, stop, return. The event handles are taken from `SequencePwm` before the
   sequencer borrows it; the sequencer stays alive until the frame ends, since dropping it
   stops the PWM. **Async:** start, `embassy_time::Timer::after(exact frame time).await`
   (other tasks run, CPU can sleep), then yield until `LOOPSDONE`. Needs a time driver, which
   every Embassy app has; behind a default `async` feature. *Set aside:* our own interrupt
   handler (needs `unsafe` to enable the interrupt, and a `bind_interrupts!` line for the
   user); pure yield polling (keeps the CPU busy). Like `ledkit-esp`, the docs say not to
   cancel `show_async` mid-frame. *Follow-up:* recover from a cancelled frame.

8. **Pin and instance for beginners:** any PWM instance and any GPIO; docs say "`PWM0`
   unless it's taken", give the XIAO D-pin map, and say to avoid P0.09/P0.10 (NFC) and the
   onboard RGB LED pins. Examples use **D10 (P1.15)**, which is also SPIM MOSI in the SPI
   example, so one wire serves both. High drive on the pin; the HF crystal is started in the
   examples for exact timing.

9. **Crate outside the root workspace**, like `ledkit-esp`, because `embassy-nrf` only builds
   for an nRF target; built and linted through its examples in CI. The logic worth testing
   (search, encoder) lives in `ledkit-core` and is host-tested there.

## Alternatives on the nRF52840

| Path | Verdict |
|---|---|
| **PWM + EasyDMA** | this crate: exact timing per chip, zero CPU, any pin; 48 B/LED |
| **SPIM + EasyDMA** (`ledkit-spi`) | works today: `new_packed` at 4 MHz (15 B/LED) or `new` at 4 MHz. The SPIM has no 2.4/3.2 MHz. Example `nrf52840-spi` |
| I2S + EasyDMA | smallest RAM (12 B/LED), two strips per instance, but needs extra clock pins and a third encoder. *Follow-up* |
| UARTE | can't work (1 Mbaud max, no inversion). The SPI example shows `ledkit-uart` refusing it |

## Examples (XIAO nRF52840)

Standalone Embassy projects (`thumbv7em-none-eabihf`, `memory.x`, pinned stable toolchain).
The XIAO has no serial chip, so the onboard RGB LED shows progress: blue = blocking round,
green = async round, red = a `show` failed (stops). Shared pattern per round, as in
`esp32s3-check`: red/green/blue/white 1 s each; back-to-back red then blue (ends blue);
counting (LED 0 red, every 10th white, last blue) 5 s; 50 fps rainbow 10 s.

- **`nrf52840-pwm`:** PWM blocking; PWM async with a second task busy-toggling a pin
  (T8/T9); a 1–150 LED length sweep (T4); then a rainbow forever.
- **`nrf52840-spi`:** SPIM2 (SPIM3 has erratum 198) `new_packed` at 4 MHz blocking and async; `new` at 4 MHz; then
  `ClocklessUart` at 1 Mbaud on UARTE0, which must be refused (`UartBaudUnusable`): onboard
  LED magenta = refused as expected.

- **`nrf52840-dotstar`** (added at the owner's request, 2026-11-02; test plan T7): DotStars
  (APA102 / SK9822) need a clock wire, so they go over SPIM with `ledkit-spi`'s
  `ClockedSpi`, not the PWM. Data on D10 (MOSI), clock on D8 (SCK), each through its own
  74AHCT125 channel. SPIM3 (the only nRF52840 SPIM above 8 MHz). Rounds: a clock sweep at
  1, 2, 4, 8, 16 and 32 MHz (colors and counting at each; the onboard LED blinks the step
  number); HD brightness steps 5–60 %; a rainbow forever at 8 MHz. Chip and LED count are
  settings at the top (APA102 × 64 for the HD 8×8; change to `Sk9822` × 144 or `Apa102` ×
  300 for the long strips). No new library code.

READMEs: wiring (D10 → 74AHCT125 → 330 Ω → DIN, as on the Teensy); flashing with the UF2
bootloader (double-tap RESET, `uf2conv.py -f 0xADA52840 -b 0x27000`, copy to the drive),
`memory.x` with flash at `0x27000` (after the bootloader's S140 SoftDevice) and RAM at
`0x20000008` while BLE is unused; `probe-rs` on the SWD pads as the alternative; what you'll
see; "Status: pending hardware test".

My calls on the builder's open questions: `equal_period_ticks` is written here, with the
signature above; the RP session can take it as-is. The XIAO's Adafruit-style bootloader
jumps to the app with the SoftDevice disabled, but the MBR still forwards every interrupt
through the address in RAM's first 8 bytes, so RAM starts at `0x20000008` (found on hardware,
2026-11-08: from `0x20000000` the first timer interrupt reset the board). BLE will need RAM
after the SoftDevice's share. Starting the HF crystal is harmless if the bootloader already has it running.

## Repo edits (small, for the merge with `ledkit-rp`)

Two CI jobs appended to `ci.yml`; one crate-map row plus the "exist so far" and "outside the
workspace" lines in `AGENTS.md`; a roadmap row **5b `ledkit-nrf`** under milestone 5 with a
short notes section; one line in `CONTRIBUTING.md` naming PWM as a third pulse model.

## Follow-ups (refine pass)

I2S output; both sequences / refill for frames over ~1350 LEDs; recovery from a cancelled
async frame; an interrupt-driven async wait; several strips per PWM instance; BLE running
during a frame (T11); `nrf52840-hal` support; other nRF52 chips.
