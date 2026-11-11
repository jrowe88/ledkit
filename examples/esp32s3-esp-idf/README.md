# ESP32-S3 hardware check on ESP-IDF: RMT ± DMA, SPI and serial, with Wi-Fi

Drives a WS2812B ("NeoPixel") strip from an Adafruit ESP32-S3 Feather with
`ledkit-esp-idf`, the `std` flavour of Rust on ESP32. It checks the RMT with ESP-IDF
refilling it from an interrupt, and with the RMT reading the frame through DMA
(`RmtOutput::new_with_dma`, which only the S3 and P4 have). Then it drives the same strip
through esp-idf-hal's SPI and serial drivers with `ledkit-spi` and `ledkit-uart`, and
repeats both RMT rounds with the Wi-Fi radio busy.

> **Passes on hardware** (Adafruit ESP32-S3 Feather, a 100-LED WS2812B strip): the RMT
> rounds with and without DMA, Wi-Fi off and scanning nonstop (2026-11-09), and the SPI
> and serial rounds (2026-11-11), each with every sweep from 1 to 150 LEDs and no panic.
> If you try it on other hardware, please report what you see, round by round.

## What you'll see

| Round | Output | Wi-Fi |
|---|---|---|
| 1 | `RmtOutput::new` | off |
| 2 | `RmtOutput::new_with_dma` | off |
| 3 | `ClocklessSpi::new` on `SpiBusDriver` (SPI2 with DMA, 3.2 MHz) | off |
| 4 | `ClocklessUart` on `UartDriver` (UART1, 4 Mbaud, the pin inverted) | off |
| 5 | `RmtOutput::new` | scanning nonstop |
| 6 | `RmtOutput::new_with_dma` | scanning nonstop |

Each round:

1. The whole strip red, green, blue, then white, one second each (20% brightness).
2. Two frames back to back, red then blue. The strip must end **blue**.
3. Counting, 5 seconds: LED 0 red, every 10th LED white, the last LED (99) blue.
4. A fast moving rainbow, 50 frames a second, for 10 seconds.
5. The sweep: frames of every length from 1 to 150 LEDs, a dim blue bar growing along
   the strip. On the ESP32-C6, some lengths once crashed ESP-IDF's RMT driver (see
   `ledkit-esp-idf`'s README), so watch for a panic in the serial output.

Then the rainbow runs forever. Press **RESET** to run the check again.

The Wi-Fi rounds don't join a network. The radio scans for networks over and over,
which keeps it and its interrupts busy, with no password needed. The serial output
shows a line after each scan.

## Wiring

The same as [`esp32s3-check`](../esp32s3-check#wiring): Feather pin **5** (GPIO5)
through a 74AHCT125 to the strip's DIN, the strip on its own 5 V supply, grounds joined.
Leave pin **6** (GPIO6) unconnected: round 3 puts the SPI clock on it, and round 4 the
serial port's unused receive side.

## One-time setup

```sh
cargo install espup espflash ldproxy --locked
espup install --targets esp32s3
```

ESP-IDF's build uses C tools that need the environment `espup` wrote. Before building,
load it in your shell:

- PowerShell: `. $HOME\export-esp.ps1`
- macOS/Linux: `. $HOME/export-esp.sh`

The first build downloads ESP-IDF and its S3 tools into `~/.espressif` (10+ minutes).
Later builds are quick.

On Windows the build goes to `C:/t/ledkit-s3` (set in `.cargo/config.toml`), because
`esp-idf-sys` rejects output paths longer than 88 characters. On macOS/Linux, set
`CARGO_TARGET_DIR=target` to build here instead.

## Run it

```sh
cargo run --release
```

If espflash can't connect, hold **BOOT**, tap **RESET**, let go of BOOT and run again;
afterwards press RESET to start the program.

## Expected serial output

```text
ledkit ESP32-S3 ESP-IDF check: 100 LEDs on GPIO5
ROUND 1: RMT, Wi-Fi off
  red
  ...
  sweep: frames of 1 to 150 LEDs
    25 LEDs
    ...
ROUND 2: RMT with DMA, Wi-Fi off
...
ROUND 3: SPI with DMA (3200000 Hz)
...
ROUND 4: serial port (4000000 baud, pin inverted)
  the port runs at 4000000Hz baud
...
Starting Wi-Fi: scanning nonstop from now on
ROUND 5: RMT, Wi-Fi scanning
  (Wi-Fi scan: 7 networks)
...
DONE. Rainbow forever; press RESET to run the check again.
```

ESP-IDF prints its own start-up and Wi-Fi log lines in between.

## Troubleshooting

- **Round 2 fails with `ESP_ERR_NOT_SUPPORTED`**: the chip has no RMT DMA. On the S3
  it does, so check the build is for `esp32s3` (`MCU` in `.cargo/config.toml`).
- **Rounds 5 and 6 glitch but 1 and 2 don't**: that's a finding about Wi-Fi. Note which
  rounds and what it looked like.
- **`Unknown rmt_encode_state_t value: 3` panic**: the esp-idf-hal bug `ledkit-esp-idf`
  works around. Note the length the sweep had reached.
- **Round 4 fails with `uart rx buffer length error`**: ESP-IDF wants a receive buffer
  even when only sending, which esp-idf-hal's transmit-only `UartTxDriver` doesn't give
  it. Use `UartDriver` with a spare pin for receive, as `main.rs` does.
- **`Too long output directory`**: see the Windows note under One-time setup.
