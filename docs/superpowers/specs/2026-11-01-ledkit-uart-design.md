# ledkit-uart: driving one-wire LEDs from a serial port

Status: design, waiting for the owner's approval (2026-11-01).

## Intent and why

ledkit drives one-wire ("clockless") LEDs, the NeoPixel family, in two ways today: SPI
(`ledkit-spi`, any board) and the ESP32's RMT (`ledkit-esp`, `ledkit-esp-idf`). This adds a
third: a hardware serial port (UART). The owner's reason is breadth: "variety and options,
I want to be maximally compatible and flexible." More pins and boards can drive LEDs, and the
same `Strip` code works on top of all of them.

The idea comes from PJRC's WS2812Serial for the Teensy: a serial port with its output
inverted, run fast enough that each serial byte carries two LED bits. ledkit does what it
already does for SPI: it works the bit pattern out from each chip's datasheet timing and the
port's real speed, and checks it at compile time.

## Goals

- A new crate, `ledkit-uart`, with one output, `ClocklessUart`, that drives any one-wire chip
  in ledkit's catalogue from any serial port that can run at the needed speed with its
  transmit line inverted. Checked against the catalogue: WS2812, WS2812B, WS2811 and the
  SK6812 (RGB and RGBW) all fit at 4 Mbaud; WS2813/WS2815 at about 2.2 to 2.67 or 3.0 to
  3.33 Mbaud; APA106 at about 2.65 to 3.3; the slow 400 kHz WS2811 at about 1.54 to 1.9.
- **Blocking and async**, like `ledkit-spi`: blocking over `embedded_io::Write`, async over
  `embedded_io_async::Write`, both from embedded-io **0.7** (what `imxrt-hal` 0.6
  implements; esp-hal 1.2 implements 0.6 and 0.7). DMA, where a board has it, comes from the board's own driver
  underneath the async write.
- **The encoding in `ledkit-core`**, next to the SPI one: pure, `const`, tested on the host.
  It picks the pattern from the chip's timing and the real baud, and returns a clear error
  (with the nearest bauds that work) when none fits, so a wrong speed fails the build, not the
  LEDs.
- **Two boards, tested on hardware:** the Teensy 4 and the ESP32-C6, each on the owner's
  100-LED WS2812-type strip through the 74AHCT125. Other boards are not ruled out.
- `smart-leds` write traits behind a feature, like `ledkit-spi`.
- No `unsafe` (the workspace forbids it), `no_std`, no heap.

## Non-goals

- Parallel strips (OctoWS2811-style or FlexIO). Paused by the owner; it needs a multi-strip
  output in `ledkit-core` first, designed separately.
- Clocked LEDs (DotStars) over a serial port: they need a clock line, which a serial port
  doesn't have.
- Other boards (the Pi, STM32, RP2040) in this round. Nothing should block them later.
- Boards whose serial port can't invert its output. They would need an inverter chip on the
  wire; not supported or documented in this round.

## Anti-goals

- Copying WS2812Serial's fixed timing. Its "1" bit is 1000 ns high and 250 ns low, which is
  outside ledkit's WS2812B window (the low part must be at least 450 ns). ledkit must send
  only patterns that fit the chip's datasheet, like everywhere else.
- Board-specific code in `ledkit-uart`. Setting up the port (pin, speed, inversion, DMA)
  stays the user's code, shown in the examples, as it is for SPI ("who sets up the hardware"
  in the main design).

## How the signal works

A serial byte on the wire is 10 slots of equal length: a start bit, 8 data bits (least
significant first) and a stop bit. With the transmit line **inverted**, the line idles low,
the start bit is high and the stop bit is low, which suits one-wire LEDs: idle low is also
the LEDs' reset (latch) state.

Each byte carries two LED bits of 5 slots each:

- LED bit A uses the start bit and data bits 0 to 3. It always starts high (the start bit).
- LED bit B uses data bits 4 to 7 and the stop bit. It always ends low (the stop bit).

Each LED bit is sent as `k` high slots then `5 - k` low slots, with `k` from 1 to 4, chosen
separately for a "0" and a "1". The baud sets the slot length: at 4 Mbaud a slot is 250 ns
and an LED bit is 1.25 µs (800 kHz).

Example, WS2812B at 4 Mbaud: a "0" is 1 slot high (250 ns) and 4 low (1000 ns); a "1" is 3
high (750 ns) and 2 low (500 ns). All four fit ledkit's WS2812B windows (0-high 220 to
450 ns, 0-low 580 to 5000, 1-high 580 to 1000, 1-low 450 to 5000). That pair works from
3.0 to about 4.4 Mbaud. Data byte for "1 then 0" (slots after inversion H H H L L | H L L L L,
data bits are the inverse of slots 1 to 8): `0xEC`.

**The reset gap.** Unlike SPI, a serial port can't send "all low" bytes (every byte has a
high start bit). So the latch is time: after a frame has fully left the port (`flush`), the
output waits the chip's reset time (280 µs for the WS2812B) before the next frame can start.
`ClocklessUart` takes a delay (`embedded_hal::delay::DelayNs`, or the async one) for this.

**Gaps inside a frame.** If the port pauses between bytes, the line sits low, which only
lengthens the low part of LED bit B. A pause is harmless while that low part stays inside
the chip's window (up to 5000 ns for the WS2812B, and a `0` bit's low part is already 1000 ns
at 4 Mbaud, so pauses up to about 4 µs). With DMA or
a hardware FIFO there are no pauses; a slow blocking driver could cause them. Document it.

## The parts

### In `ledkit-core` (encoding, host-tested)

- `ClocklessTiming::uart_encoding(baud) -> Result<UartEncoding, TimingError>`, a `const fn`
  like `spi_encoding`: tries every (`k0`, `k1`) pair, keeps those whose four pulses fit the
  chip's windows exactly at this baud, and picks the one closest to the chip's typical
  timing.
- `UartEncoding` (zero and one high-slot counts) with `const fn buffer_len(wire_bytes)` (4
  serial bytes per wire byte, so 12 per RGB LED and 16 per RGBW LED) and `encode` /
  `encode_from_iter` like `SpiEncoding`.
- A new `TimingError` variant for an unusable baud, carrying the nearest working bauds above
  and below (the SPI variant's pattern), so the build error names a fix.

### `ledkit-uart` (new workspace crate)

- `ClocklessUart<C, U, D, B>`: chip `C`, serial port `U`, delay `D`, buffer `B`, mirroring
  `ClocklessSpi`. `new(uart, baud, delay, buffer) -> Result<Self, TimingError>` takes the
  baud the port **really** runs at.
- `Output` over `embedded_io::Write` + `DelayNs`; `OutputAsync` over
  `embedded_io_async::Write` + the async `DelayNs`. A frame is encoded into the buffer, then
  written in one call (`write_all`), flushed, then the reset time is waited out.
- `const fn buffer_len::<C>(led_count, baud)`, so examples size their buffer at compile
  time, as with `ledkit-spi`.
- `smart-leds` feature, as in `ledkit-spi`.
- README and crate docs in the style of `ledkit-spi`: wiring, the baud table per chip, the
  inversion requirement, the reset gap, a hardware results table.

### Examples (outside the workspace, built in CI like the others)

- **`teensy4-uart`**: Teensy 4 serial port 1 (`LPUART6`, transmit on pin 1) through the
  74AHCT125 to the strip. `imxrt-hal` 0.6 sets the baud (24 MHz clock, 4 Mbaud exactly) and
  the inversion (`Disabled::set_inversion`). Two paths:
  - blocking, using `Lpuart`'s `embedded_io::Write`;
  - DMA, through a small adapter in the example that implements `embedded_io_async::Write`
    with `Lpuart::dma_write`, polled with `embassy_futures::block_on`.
- **`esp32c6-uart`**: an ESP32-C6 serial port with transmit on GPIO4 (the pin the other C6
  examples use), at 4 Mbaud, inverted through the GPIO matrix
  (`OutputSignal::with_output_inverter`, esp-hal `unstable`, which the C6 examples already
  enable). Blocking and async (`UartTx::into_async`), both of which implement the
  `embedded-io` traits.
- Each shows red, green, blue and white, a counting pattern (LED 0 red, every 10th white,
  the last blue) and a fast rainbow, like the long-strip tests.

## Constraints

- Workspace rules: no `unsafe`, `no_std`, no heap, MSRV as the workspace sets it, clippy
  pedantic clean, docs in the project's plain style.
- Teensy stack as already used: `teensy4-bsp` 0.6, `imxrt-hal` 0.6.
- C6 stack as already used: esp-hal 1.2 with `unstable`.

## Risks to check early

- **Start-up glitch.** Until inversion is switched on, a port idles high. The LEDs may read
  that as a stray bit. Check on hardware; if it shows, the examples set inversion before
  routing the pin, or send one reset gap first.
- **Teensy DMA without an interrupt.** The DMA future should complete when polled; if it
  needs the DMA interrupt to wake, the example wires up the interrupt. The DMA buffer must
  not sit in cached memory without a cache flush.
- **`flush` timing.** Checked: `imxrt-hal`'s `flush` waits for transmit complete, and
  esp-hal's waits until the FIFO is empty and the transmitter idle. Even a driver whose
  `flush` returned early would be covered, since 280 µs is far longer than one 2.5 µs byte.
- **Pauses in blocking mode** (above).

## Left to the builder

(A builder check found nothing important missing; these are the builder's calls.)

- Exact names, the `TimingError` variant's fields, and where the encoding file lives
  (`encode/uart.rs` is the natural spot).
- The baud grid for the "nearest working baud" suggestions.
- Whether the delay is required or optional (for users who pace frames themselves).
- Test details, and how the CI jobs for the two examples are set up.
- Whether the reset wait sits at the end of a frame or the start of the next (CONTRIBUTING's
  "remember when the last frame ended" version also fits), and whether to offer chunked
  writes like `ClocklessSpi::with_max_write_len`.
- Hardware passes: the owner flashes both boards and reports, as for earlier results.
