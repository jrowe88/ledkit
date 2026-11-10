# Hardware findings

What testing on real boards taught us: the board quirks, the bugs real hardware found,
and why some settings are what they are. The pass/fail record for each path lives in
each crate README's "Tested on hardware" table and each example README's status line;
this file keeps the *why*. Add to it when a test turns up something a future reader
would otherwise rediscover.

## Bench setup

- Strips: a 100-LED WS2812B-type strip, a 300-LED APA102-type 5050 DotStar strip, a
  144-LED SK9822 strip. Panels: an Adafruit NeoPixel 8×8 (WS2812B) and an Adafruit
  DotStar HD 8×8 (APA102-2020, #3444).
- Off-board data goes through a 74AHCT125: data pin → 1A, 1Y → 330 Ω → DIN, 1OE to GND,
  VCC 5 V, the strip on its own 5 V supply, grounds joined. If a test flickers with no
  color pattern, suspect the wiring first.
- At 4.3 V supply, a WS2812B reads 3.0 V and up as high, so 3.3 V data works without the
  level shifter (used for the ESP32-S3 run).

## All chips

- **Dithering at low frame rates** (2026-10-30): at 20% brightness each channel reaches
  the LED as only about 0 to 9, and at about 45 fps the 8-frame dither cycle repeats at
  5–6 Hz, which the eye sees as flicker. The owner kept dithering on by default; the docs
  and every slow-rainbow example turn it off and say why.
- **Perceptual brightness below about 6% is off**, documented on `Brightness` (found
  porting the pulse project's minimum brightness).
- **DotStar end frame:** the APA102 needs `ceil(n / 16)` bytes of `0xFF` (a zero end
  frame left LEDs 60–63 of the 8×8 unlit); verified on the 8×8 and the 300-LED strip.
  The SK9822 keeps a 4-byte zero reset frame plus `ceil(n / 16)` zero bytes; verified on
  the 144-LED strip.
- **DotStar HD brightness** (on by default): lowers the 5-bit level only below raw level
  64 (about 52%), because on the APA102-2020 a lowered 5-bit level gives less light than
  the math says. Side by side, HD lights from 10%, plain only from 20%.
- **APA102-2020s apply their own gamma** (about 2.6) to the 8-bit color, while the 5-bit
  level is close to linear. So ledkit's perceptual curve is applied twice on them, and
  the power estimate is very conservative. See [power-measurements.md](power-measurements.md).
- **WS2812B power:** about 11 mA per channel at full, not the 20 mA ledkit assumes, so
  the power limit holds the strip to about half its budget.

## ESP32-C6

- `ledkit-esp-idf`: esp-idf-hal 0.47's RMT encoder bridge panics on the state "complete +
  memory full" (`Unknown rmt_encode_state_t value: 3`), which happens whenever a frame
  ends on a refill block: every WS2812B frame of 4+ LEDs whose bit count is a multiple
  of 48. `RmtOutput` pads even-length frames with one `0x00` byte. Fixed upstream in
  esp-rs/esp-idf#600; drop the padding after the next esp-idf-hal release.
- `ClocklessSpi` with esp-hal blocking SPI refills its 64-byte FIFO mid-frame without a
  gap the LEDs notice (100 LEDs at 3.2 MHz, `new` and `new_packed`). One board only, so
  the `new_packed` warning stays for other HALs.
- DotStar clock sweep 1 to 40 MHz requested ran clean through a 74AHCT125 on a breadboard.

## ESP32-S3 (Adafruit Feather #5477)

- The onboard NeoPixel's power switch, GPIO21, must be driven high first.
- The S3's SPI holds MOSI high from set-up until its first transfer, so the first frame
  arrived one bit late. `ClocklessSpi` now starts its first frame with a reset gap.
- Native USB: if `espflash` can't connect, hold BOOT and tap RESET.
- The ESP-IDF examples set a short `target-dir` because `esp-idf-sys` rejects output
  paths over 88 characters on Windows.

## Teensy 4

- LPSPI needs two settings for NeoPixels: tristate the data pin between transfers, and no
  PCS-to-SCK delay. Without them the first LED of each transfer reads a stray "1".
- Three 512-byte SPI writes per 100-LED frame: the pauses between them do no harm.
- The BSP clocks its UARTs at 8 MHz (max 2 Mbaud), so `teensy4-uart` raises the UART
  clock to 24 MHz for 4 Mbaud.

## Raspberry Pi Zero W

- `linux-spidev` needs `dtparam=spi=on` and `core_freq=250` (32-bit Pi OS trixie; building
  on the Pi takes about 22 minutes).

## RP2040 (Feather SCORPIO) and RP2350 (Pico 2 W)

- rp-hal, not embassy-rp: embassy-rp 0.10 depends on `cortex-m` unconditionally, so it
  can't build for the RP2350's RISC-V cores.
- The SCORPIO needs the GD25Q64C boot code (`BOOT_LOADER_GD25Q64CS`). Its RESET button was
  unreliable (holding it usually worked; replugging always did).
- The SCORPIO shifts GPIO16–23 to 5 V, so strips plug into a NeoPixel channel with no
  74AHCT125.
- Flashing on the owner's PC: `cargo run --release` with the board in BOOTSEL (picotool
  with a Zadig driver); copying the UF2 is the fallback.

## nRF52840 (Seeed XIAO)

- `memory.x` must start RAM at `0x20000008`: from `0x20000000` the program overwrote the
  MBR's interrupt-forwarding address and reset in a loop after one frame.
- The WS2812B runs at 22 PWM ticks (1.375 µs) per bit, not nrf-smartled's 20, to keep its
  `1` low at or above 450 ns.
- The SPIM needs a clock pin even for NeoPixels: with `new_txonly_nosck` it never started,
  so the example puts SCK on an unconnected pin (D8).
- The UARTE tops out at 1 Mbaud with no inversion, so `ledkit-uart` refuses it with a clear
  error (a tested negative case).

## Teensy 4 WS2812 techniques (research, 2026-11-01)

Background for parallel strips, which wait on a multi-strip output in `ledkit-core`.

- **WS2812Serial** (the model for `ledkit-uart`): an LPUART at 4 Mbaud with inverted
  output and DMA; ten 250 ns slots per byte carry 2 LED bits. Its "1" (1 µs high, 0.25 µs
  low) is outside ledkit's WS2812B window, which is why `ledkit-uart` picks the high-slot
  counts from each chip's timing.
- **OctoWS2811** (MIT): QuadTimer 4 channels mark the bit start, the "0" point and the
  "1" point; XBAR turns each into a DMA request, and three DMA channels write the GPIO
  set/clear registers. Pixels are transposed so each word holds one bit per pin,
  double-buffered in about 10 KB.
- **Rust:** `imxrt-hal` 0.6 has no FlexIO, QuadTimer or XBAR drivers.
  [`ws2812-flexio`](https://github.com/finomnis/ws2812-flexio) (MIT/Apache) drives up to 3
  parallel strips per FlexIO block, but it is on `imxrt-ral` 0.5 / `imxrt-dma` 0.1, so it
  can't take the peripherals from `teensy4-bsp` 0.6 without a port. Look at wrapping or
  contributing to it before writing FlexIO support from scratch.
