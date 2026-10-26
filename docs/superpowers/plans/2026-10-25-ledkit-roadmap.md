# ledkit roadmap: plan of attack

**Spec:** `docs/superpowers/specs/2026-10-25-ledkit-design.md`

The spec covers about ten crates plus docs and tooling, so it is delivered as a series of
milestones. Each milestone gets its **own detailed plan**, written when the milestone
starts, so it can build on the real interfaces the previous milestone produced. Each one
ends with something that works and is tested on its own.

| # | Milestone | Delivers | Proves | Needs hardware? |
|---|---|---|---|---|
| **1** | **Foundation + `ledkit-color`** | Cargo workspace, CI, licenses, README/AGENTS/CONTRIBUTING stubs; the complete color crate | The crate conventions, `no_std` builds on a no-FPU Arm target and on RISC-V, and the readable-API style | No |
| **2** | **`ledkit-core`** | LED chip catalogue (all phase 1 chips), color orders, encoders for every technique (pulse pairs, SPI with clock validation, clocked frames), `Strip<Led, N>`, the output pipeline (brightness, gamma, power limit, dithering), blocking + async output traits, `smart-leds-trait` interop | The central "define a chip once, encode for any technique" idea, entirely in host-side unit tests | No |
| **3** | **First light: `ledkit-spi`** | The universal SPI backend (blocking + async) and its first examples | Real LEDs on a **Raspberry Pi (Linux)** and a **Teensy 4**: NeoPixel via SPI encoding, DotStar natively | Pi Zero / 3 A+, Teensy 4, WS2812B + APA102 strips |
| **4** | **ESP32 backends** | `ledkit-esp` (esp-hal RMT, `no_std`) and `ledkit-esp-idf` (port of the pulse project's RMT driver) | Both ESP32 worlds on the C6 DevKit, then the S3 (Xtensa) | ESP32-C6, ESP32-S3, strips |
| **5** | **`ledkit-rp`** | RP2040/RP2350 PIO backend | Pico 2 | Pico 2, strips |
| **6** | **`ledkit-layout`** | Segments, matrices (serpentine, rotation, tiling), `Canvas`, `embedded-graphics` support | Text and shapes on an 8×8 / 16×16 matrix | A matrix |
| **7** | **`ledkit-anim`** | The `Animation` trait, the phase 1 effects, and the combinators (sequence, crossfade, layer, per-segment) | Combined effects running on strips and matrices | Strips, matrix |
| **8** | **"Just works" packaging** | The `ledkit` facade + prelude, `cargo generate` board templates, the numbered example ladder, the mdBook guide with photos, `AGENTS.md` + `llms.txt` | A hobbyist going from nothing to a lit strip by following the guide | All first-class boards |
| **9** | **Release 0.1** | Publish all phase 1 crates to crates.io in dependency order, docs.rs config, guide on GitHub Pages, announcement | Others can depend on it | No |
| 10 | Phase 1.5 | `ledkit-sim` desktop simulator, Claude Code plugin, remaining effects | | No |

**Ordering rationale:**
- Milestones 1–2 need no hardware, so they can start before your new boards arrive.
- Milestone 3 uses the cheapest, most universal backend, on hardware you already own, to
  validate the core design before the chip-specific backends depend on it.
- Milestones 6–7 can run in parallel with 4–5 once milestone 2 is done, since they depend
  only on the core and color crates.

**Repository:** `C:\Dev\jrowe88\ledkit`, to be pushed to `github.com/jrowe88/ledkit` once
milestone 1 is done.
