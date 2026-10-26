# Contributing to ledkit

Thanks for helping. ledkit aims to be easy to use and easy to contribute to.

## Build and test

```sh
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo build -p ledkit-color --target thumbv6m-none-eabi
cargo build -p ledkit-color --target riscv32imc-unknown-none-elf
cargo build -p ledkit-core --all-features --target thumbv6m-none-eabi
cargo build -p ledkit-core --all-features --target riscv32imc-unknown-none-elf
```

The embedded targets come from `rustup target add thumbv6m-none-eabi riscv32imc-unknown-none-elf`.
See [AGENTS.md](AGENTS.md) for the code conventions.

## License of contributions

Unless you state otherwise, any contribution you submit is licensed under
MIT OR Apache-2.0, the same as the project, without additional terms.

## Add an LED type

LED chips live in `crates/ledkit-core/src/chips.rs`.

Every new chip gets a doc comment in plain words that says what products use it, and cites the datasheet (name, revision and the table the timings come from).

### A clockless chip (one data wire, NeoPixel-style)

1. Add a struct with `clockless_chip!`. Set the pixel type (`Rgb` or `Rgbw`), the channel order, the `[zero_high, zero_low, one_high, one_low]` phases as `ph(typical, min, max)` in nanoseconds, and the reset time in microseconds.
2. Add a row for it to the `clockless_catalogue_matches_the_table` test and add the type to the `every_typical_value_is_inside_its_window` test, both in `chips.rs`.
3. Add a test in `crates/ledkit-core/src/encode/spi.rs` that checks `ClocklessTiming::of::<YourChip>().spi_encoding(...)` at a common SPI clock (for example 3.2 MHz or 4 MHz), including the expected bit pattern.
4. Add a row to the "Which chip type do I use?" table (see below).
5. Run the commands under "Build and test".

### A clocked chip (data plus clock, DotStar-style)

1. Add a struct with `clocked_chip!`. It takes only the type name and the printed name: the macro fixes the APA102 frame format, `BGR` channel order and `Rgb` pixels, so there are no timings to enter. If your chip needs a different order, pixel type or frame format, `clocked_chip!` doesn't fit; open an issue first.
2. Add it to the `clocked_catalogue_matches_the_table` test in `chips.rs`.
3. Add a row to the "Which chip type do I use?" table (see below).
4. Run the commands under "Build and test".

### The "Which chip type do I use?" table

The same table appears in three places, and they must stay identical: the top of `crates/ledkit-core/src/chips.rs`, the crate docs in `crates/ledkit-core/src/lib.rs`, and `crates/ledkit-core/README.md` (which uses plain backticks instead of links).

## Add a backend

Coming in a later milestone.

## Add an effect

Coming in a later milestone.
