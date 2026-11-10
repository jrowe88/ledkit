# Hardware test plan

**Goal:** repeat on four new boards the by-eye checks that passed on the ESP32-C6
DevKit, the Teensy 4.0 and the Raspberry Pi Zero W (see each crate's "Tested on
hardware" table on `main`). For one-wire LEDs, test **every way each chip can make the
pulses**: its purpose-built peripheral (RMT, PIO, PWM sequencer), the DMA paths, and the
portable SPI and UART backends. Record results the same way as before.

**New boards (2026-11-02):**

| Board | Chip | Core / target | Onboard addressable LED | 5 V level shifting |
|---|---|---|---|---|
| Adafruit ESP32-S3 Feather 4 MB flash / 2 MB PSRAM (#5477) | ESP32-S3 | Xtensa LX7, `xtensa-esp32s3-none-elf` / `xtensa-esp32s3-espidf` | NeoPixel on GPIO33; its power switch GPIO21 must be driven high first | no, use the 74AHCT125 |
| Adafruit #5650 Feather RP2040 SCORPIO (8-channel NeoPixel driver) | RP2040 | Cortex-M0+, `thumbv6m-none-eabi` | NeoPixel on GPIO4 | **yes**: GPIO16–23 come out level shifted to 5 V on the NeoPixel header |
| Raspberry Pi Pico 2 W | RP2350 | Cortex-M33 `thumbv8m.main-none-eabihf` **and** Hazard3 RISC-V `riscv32imac-unknown-none-elf` | none (the "LED" sits on the CYW43 Wi-Fi chip) | no, use the 74AHCT125 |
| Seeed XIAO nRF52840 | nRF52840 | Cortex-M4F, `thumbv7em-none-eabihf` | none (plain RGB LED) | no, use the 74AHCT125 |

Pins in this plan come from memory of the vendor pinouts. Check each one against the
board's pinout card before wiring.

## What already passed (the baseline)

| # | Check | How it was judged | Passed on |
|---|---|---|---|
| T1 | **First light**: red, green, blue, white at 20%, then a slow rainbow (dithering off) | colors in order, smooth | C6 onboard LED: `ledkit-spi`, `ledkit-esp`, `ledkit-esp-idf` |
| T2 | **Back-to-back frames**: red then blue with no delay | LED ends on blue (the reset gap is honoured) | C6 (`ledkit-esp`, `ledkit-esp-idf`) |
| T3 | **100-LED strip**: solid colors, counting (LED 0 red, every 10th white, last blue), fast rainbow at 50 fps | clean through to LED 99, no glitches | C6 (SPI `new` + `new_packed`, RMT blocking + async, UART blocking + async, esp-idf), Teensy (SPI with `with_max_write_len`, UART blocking + DMA), Pi Zero W (spidev, `core_freq=250`) |
| T4 | **Frame-length sweep** 1 to 150 LEDs | no crash, every length lights | C6 `ledkit-esp-idf` (found the multiple-of-48 padding bug) |
| T5 | **8×8 matrix** (`ledkit-layout`): wiring finder, shapes, scrolling text | finder matches `.columns_first().starting_at(Corner::BottomLeft)` | C6 (RMT), Teensy (SPI) |
| T6 | **Animation show** (`ledkit-anim`) on the 8×8 and with `--features strip` | every animation and transition looks right | C6, Teensy |
| T7 | **DotStars** (`ClockedSpi`): HD 8×8 brightness sweep, 1 to 40 MHz clock sweep, SK9822 144 (zero end frame), APA102 300 (`0xFF` end frame) | every frame latches to the last LED; HD lights at 10% | C6, Teensy (HD 8×8) |

New checks for the DMA and native-peripheral paths:

| # | Check | Why |
|---|---|---|
| T8 | **Busy CPU**: T3 while another task hammers the CPU and interrupts (a busy loop on the other core, or a timer interrupt at 10 kHz) | DMA, PIO and RMT should be unaffected; CPU-fed paths (blocking SPI/UART, RMT refill) may glitch. This is the real reason to prefer the native paths |
| T9 | **Non-blocking frame**: start a frame, do work, then wait; check `show` returns before the frame ends (where the API allows) and the next `show` still waits out the reset gap | proves DMA really runs in the background and T2 still holds |
| T10 | **Long frame**: as many LEDs as the buffer allows (chain the 8×8 after the strip for 164), or the longest frame RAM allows | buffer sizing, DMA length limits, refill paths |
| T11 | **Radio on**: T3 with Wi-Fi or BLE active (S3, Pico 2 W, nRF) | radio interrupts and flash/PSRAM cache stalls |

## Every way to make one-wire pulses, per board

**Status (2026-11-10):** every board passes T1–T4 and T7 on its native backend and on SPI
(see each crate README's "Tested on hardware" table); the matrix and animation checks (T5,
T6) have run on the ESP32-C6, Teensy 4 and ESP32-S3 only. T8–T11 haven't been run on any board yet;
they are the open rounds in the [roadmap](../roadmap.md).

| Path | ESP32-S3 Feather | SCORPIO (RP2040) | Pico 2 W (RP2350) | XIAO nRF52840 |
|---|---|---|---|---|
| **Native pulse peripheral** | RMT (`ledkit-esp`, `ledkit-esp-idf`) ✅ exists | PIO + DMA (`ledkit-rp`, M5) | PIO + DMA (`ledkit-rp`, M5) | PWM sequencer + EasyDMA (`ledkit-nrf`, new); the usual nRF approach (`nrf-smartled`) |
| Native, CPU-fed variant | RMT blocking refill (48-word channel) and async refill | PIO fed from the CPU FIFO (no DMA) | as RP2040 | – (PWM is always DMA) |
| Native, DMA variant | RMT with DMA (on the S3, one TX channel): `ledkit-esp-idf` `RmtOutput::new_with_dma` ✅ added; esp-hal 1.2 has no RMT DMA | PIO + DMA | PIO + DMA | PWM + EasyDMA |
| SPI, CPU-fed (`new`) | ✅ | ✅ (PL022 FIFO) | ✅ | – (SPIM is always EasyDMA) |
| SPI, DMA (`new_packed` is safe) | esp-hal `SpiDma` | rp-hal SPI + DMA | as RP2040 | ✅ SPIM EasyDMA, 4 MHz only (no 2.4 / 3.2 MHz) |
| I2S, DMA | possible, not planned | – | – | I2S + EasyDMA: a second nRF option, later |
| UART, CPU-fed | ✅ (inverted in the GPIO matrix) | ✅ UART0 TX GPIO16 / UART1 TX GPIO20; invert with the pad's output override | ✅ same | ❌ UARTE max 1 Mbaud, no inversion: **negative test** (ledkit must refuse with a clear error) |
| UART, DMA | esp-hal 1.2 has no UART DMA; async only | rp-hal UART + DMA | as RP2040 | ❌ |
| Parallel strips | I2S/LCD-CAM (FastLED style), later | PIO, 8 shifted channels: the board for it | PIO | – |

## Backend and example work before testing

1. **ESP32-S3** (done 2026-11-02, builds; passed on hardware 2026-11-09): `esp32s3-rmt` (onboard
   NeoPixel), `esp32s3-check` (one strip, seven rounds: RMT blocking and async, SPI `new`,
   SPI DMA `new_packed` blocking and async, UART blocking and async, then a 1–150 LED
   sweep), `esp32s3-esp-idf` (RMT and RMT DMA, Wi-Fi off and scanning, each with the
   sweep), and ports of `matrix`, `anim` and `dotstar-hd`. New API: `Strip::release`
   (ledkit-core) and `RmtOutput::new_with_dma` (ledkit-esp-idf). CI jobs
   `example-esp32s3` and `example-esp32s3-esp-idf`. Still open: a busy-CPU round (T8),
   and RMT channel memory size (`with_memsize`) for `ledkit-esp` if T8 shows refill
   glitches.
2. **`ledkit-rp` (milestone 5)**: PIO with CPU feed and with DMA, the equal-period timing
   search noted in the roadmap, the reset gap. One crate covering `rp2040-hal` and
   `rp235x-hal`. Examples `rp2040-pio`, `rp2350-pio` (Arm and RISC-V).
3. **`ledkit-nrf` (new milestone, needs a design)**: PWM sequencer + EasyDMA, blocking and
   async (`embassy-nrf`), the reset gap. Its 16-bit duty words cost 2 bytes per LED bit
   (48 bytes per RGB LED), so check RAM for long strips. I2S later.
4. **Portable examples** for the new boards: `rp2040-spi`, `rp2040-uart`, `rp2350-spi`,
   `rp2350-uart`, `nrf52840-spi`, each with blocking/CPU and DMA rounds like `teensy4-uart`,
   then `-anim` / `-matrix` variants on the native backend.
5. **A shared test program**: T1–T4 and T8–T10 are the same on every board. A
   `ledkit-core`-level `hardware_check` sequence (colors, counting, rainbow, length sweep,
   back-to-back) that each example calls keeps the boards comparable.
6. CI builds for each, like the existing examples.

## Per-board test sheet

Wiring for the off-board boards matches the Teensy: data pin → 74AHCT125 1A, 1Y → 330 Ω →
DIN, 1OE to GND, VCC 5 V, strip on its own 5 V supply, grounds joined. Avoid the melted
breadboard; if a test flickers with no color pattern, suspect wiring first.

"All paths" below means every row of the board's column in the table above that exists.

### 1. ESP32-S3 Feather (first: the native backend exists, only Xtensa is new)

Setup: `espup install --targets esp32s3`, `espflash`. Native USB: if `espflash` can't
connect, hold BOOT, tap RESET. The onboard NeoPixel needs GPIO21 high (`esp32s3-rmt`
does it). External strip on pin "5" (GPIO5), DotStar clock on "6" (GPIO6).

Run order: `esp32s3-rmt` (T1, T2), `esp32s3-check` (T2, T3, T4 on every esp-hal path),
`esp32s3-esp-idf` (T2–T4 and T11, with and without RMT DMA), `esp32s3-matrix` (T5),
`esp32s3-anim` and `--features strip` (T6), `esp32s3-dotstar-hd` (T7).

| Check | Paths | LEDs |
|---|---|---|
| T1, T2 | RMT (`ledkit-esp` blocking + async, `ledkit-esp-idf`), SPI | onboard NeoPixel (GPIO33) |
| T3, T10 | RMT blocking, RMT async, RMT DMA (if exposed), `ledkit-esp-idf`, SPI `new`, SPI DMA `new_packed`, UART blocking, UART async/DMA | 100-LED strip, then strip + 8×8 chained |
| T4 | `ledkit-esp-idf`, `ledkit-esp` | 100-LED strip (the padding bug was chip-sensitive) |
| T5, T6 | RMT | 8×8, strip |
| T7 | SPI, SPI DMA | DotStar HD 8×8 sweep, 300-LED APA102, SK9822 144 |
| T8 | each path from T3, with the second core busy and a 10 kHz timer interrupt | 100-LED strip; expect RMT refill to be the weak spot, RMT DMA / SPI DMA to stay clean |
| T9 | RMT async, RMT DMA, SPI DMA | 100-LED strip |
| T11 | `ledkit-esp-idf` with Wi-Fi up; `ledkit-esp` with `esp-wifi` if practical | 100-LED strip |
| PSRAM | `ledkit-esp-idf` with the frame buffer in PSRAM (DMA from PSRAM needs care on the S3) | 100-LED strip |

### 2. Raspberry Pi Pico 2 W (milestone 5's named target)

Setup: `thumbv8m.main-none-eabihf` and `riscv32imac-unknown-none-elf` targets, `picotool`
(RP2350 UF2s need the right family ID) or `probe-rs`. External strip only. VBUS (pin 40)
powers the 74AHCT125, not the strip.

| Check | Paths | Cores |
|---|---|---|
| T2, T3, T10 | PIO CPU-fed, PIO DMA, SPI `new`, SPI DMA `new_packed`, UART CPU, UART DMA | Arm, then RISC-V for each |
| T4 | PIO DMA | Arm |
| T5, T6 | PIO DMA | Arm and RISC-V |
| T7 | SPI, SPI DMA | DotStar HD 8×8, 300-LED APA102 |
| T8 | all paths, core 1 busy, timer interrupt on core 0 | Arm |
| T9 | PIO DMA, SPI DMA | Arm |
| T11 | PIO DMA with CYW43 Wi-Fi running (it uses PIO and DMA too: check channel and state-machine sharing) | Arm |

### 3. Feather RP2040 SCORPIO

Setup: `rustup target add thumbv6m-none-eabi`, `elf2uf2-rs` / `picotool`; hold BOOT, tap
RESET, copy the UF2. The board shifts GPIO16–23 to 5 V, so plug the strip into a NeoPixel
channel and skip the 74AHCT125. Power the strip from the terminal block or its own supply.

| Check | Paths | LEDs |
|---|---|---|
| T1, T2 | PIO (GPIO4); UART1 TX on GPIO4 | onboard NeoPixel |
| T3, T10 | PIO CPU-fed, PIO DMA (GPIO16), SPI `new` and SPI DMA (SPI0 TX on GPIO19), UART CPU and DMA (UART0 TX GPIO16) | 100-LED strip on a shifted channel |
| T4 | PIO DMA | 100-LED strip |
| T5, T6 | PIO DMA | 8×8, strip |
| T7 | SPI, SPI DMA (data and clock both through shifted channels: GPIO19 TX, GPIO18 SCK) | DotStar HD 8×8 |
| T8 | all paths, core 1 busy | 100-LED strip |
| T9 | PIO DMA | 100-LED strip |
| Real SPI clock | SPI | 125 MHz has no exact 3.2 or 4 MHz divider: pass the HAL's real rate and check it's accepted and works |
| Power-limit cost | any | time `show()` with a limit on 300 LEDs (the roadmap asks for a Cortex-M0 measurement) |
| Later | parallel PIO, 8 strips | the roadmap's paused multi-strip output |

### 4. XIAO nRF52840

Setup: `thumbv7em-none-eabihf`. Flash with the UF2 bootloader (double-tap RESET; check
`memory.x` starts the app after the bootloader/SoftDevice) or with `probe-rs` on the SWD
pads. MOSI is D10. Avoid P0.09 / P0.10 (NFC by default).

| Check | Paths | LEDs |
|---|---|---|
| T2, T3, T10 | PWM + EasyDMA blocking and async (`ledkit-nrf`), SPIM `new` and `new_packed` at 4 MHz, SPIM async (`embassy-nrf`) | 100-LED strip, then chained |
| T4 | PWM | 100-LED strip |
| T5, T6 | PWM | 8×8, strip |
| T7 | SPIM at 4 and 8 MHz, SPIM3 up to 32 MHz | DotStar HD 8×8, 300-LED APA102 |
| T8 | all paths, timer interrupt at 10 kHz | 100-LED strip; all should be clean (every path is EasyDMA) |
| T9 | PWM async, SPIM async | 100-LED strip |
| T11 | PWM and SPIM with BLE advertising (SoftDevice or `trouble`) | 100-LED strip |
| 2.4 / 3.2 MHz requested | SPIM | should be a build error or a clear refusal, not a wrong clock |
| UART | `ledkit-uart` | negative test, see above |

## Recording results

After each pass, add a row to the crate README's "Tested on hardware" table (board,
peripheral and path such as "PIO + DMA", settings, LEDs, date, what was seen), set the
example README's status line, and update the [roadmap](../roadmap.md)'s open follow-ups.
Anything surprising (a board quirk, a bug the hardware found) also goes in
[findings.md](findings.md). Use the same wording as the existing rows. A path that can't work (nRF UART) gets a row too, saying what error
ledkit gave.
