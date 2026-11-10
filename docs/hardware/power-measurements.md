# Power and brightness measurements

Bench measurements of what LEDs really draw, compared with ledkit's `PowerModel` and
brightness curve. The planned LED response model starts from these numbers (see
"Brightness and power" in the [roadmap](../roadmap.md)). Add new runs as new sections,
with the date, board, supply voltage and ledkit settings.

## WS2812B power calibration, ESP32-S3 Feather, 2026-11-09

100-LED WS2812B strip, bench supply at 5.00 V, data straight from GPIO5 (3.3 V, no
74AHCT125). ledkit power limit 1800 mA, dithering off, `Brightness::exact`.
Readings were the same over 3 loops.

| Step | Frame | ledkit estimate | Measured |
|---|---|---|---|
| 1 | all off | 100 mA | 88 mA |
| 2 | 10 full white | 700 mA | 405 mA |
| 3 | 10 full red | 300 mA | 198 mA |
| 4 | 10 full green | 300 mA | 198 mA |
| 5 | 10 full blue | 300 mA | 198 mA |
| 6 | 100 white, level 32 | 853 mA | 494 mA |
| 7 | 100 white, level 128 | capped at 1800 | 792 mA |
| 8 | 100 white, level 255 | capped at 1800 | 792 mA |

Derived: idle 0.88 mA per LED (model 1.0); about 11 mA per channel at full (model 20),
the same for R, G and B; linear with level (step 6 predicts 502 mA from the step 3
number, measured 494). The limiter caps steps 7 and 8 identically (it picks level about
72), but this strip then draws 792 mA, 44% of the 1800 mA budget. The 11 mA/channel
figure predicts about 1000 mA at level 72; the shortfall is probably voltage drop along
the strip at that current (not measured).

So for this strip, `PowerModel::TYPICAL_5V` reads about 1.8x high on channel current.

## DotStar HD 8x8 (Adafruit #3444, APA102-2020) calibration, same day

ClockedSpi at 4 MHz on GPIO5/6, limit 1500 mA, dithering off, Brightness::exact.

| Step | Frame | Estimate | Measured |
|---|---|---|---|
| 1 | 64 off | 64 | 97 |
| 2 | 8 full white | 544 | 533 |
| 3/4/5 | 8 full R / G / B | 224 | 252 / 248 / 249 |
| 6 | 64 white, level 32, plain | 546 | 109 |
| 7 | 64 white, level 32, HD (global 16, colors 63) | ~546 | 117 |
| 8 | 64 white, limiter picks level 95 | 1500 cap | 330 |

Idle 1.5 mA/LED (model 1.0); full channel about 19 mA (model 20, good). But partial
color levels draw far less than linear: level 32 -> 0.33% of full, level 95 -> 6.4%.
That fits a curve of about (level/255)^2.8 at both points (predicts 0.30% and 6.3%):
these LEDs seem to apply their own gamma to the 8-bit color. It explains the C6
"plain row off at 10-15%" result better than "short pulses don't light", and means the
limiter is very conservative on these (330 mA real vs 1500 budget). HD at level 32
drew more than plain (20 vs 12 mA above idle), as HD should, but less than the 2.8
curve predicts for global 16 x color 63 (meter resolution is coarse here).

### Curve test, DotStar HD 8x8, 5.0 V (and the panel works at 4.3 V too)

Raw frames from `wire::clocked_frame::<Apa102>().with_global_brightness(g)`, first 16
LEDs white, no ledkit brightness. (A first try with a hand-built frame ending in zero
bytes gave garbage: the APA102 reads 32 zero bits as a start frame; the wire.rs end
frame of 0xFF is right.)

| Global | Color | Measured mA | Above idle | Linear would be | Exponent fit |
|---|---|---|---|---|---|
| - | 0 | 96 | 0 | | |
| 31 | 16 | 97 | 1 | 54 | (too small) |
| 31 | 32 | 99 | 3 | 107 | 2.7 |
| 31 | 64 | 115 | 19 | 214 | 2.75 |
| 31 | 128 | 233 | 137 | 429 | 2.65 |
| 31 | 192 | 515 | 419 | 643 | 2.5 |
| 31 | 255 | 950 | 854 | 854 | |
| 16 | 255 | 560 | 464 | 441 | |
| 8 | 255 | 312 | 216 | 220 | |
| 4 | 255 | 186 | 90 | 110 | |
| 1 | 255 | 93 | ~0 | 28 | |

Color byte: current follows about (color/255)^2.6 (fits 2.5 to 2.75 from 32 to 192):
these APA102-2020 LEDs apply their own gamma. Global level: about linear from 4 to 31
(within 20%), next to nothing at 1. Owner: the low-global steps (16, 8, 4, 1) looked
bluish. Full channel about 18.7 mA (854 / 16 / 3 at 5.0 V), idle 1.5 mA per LED.
