# Contributing to ledkit

Thanks for helping. ledkit aims to be easy to use and easy to contribute to.

## Build and test

```sh
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all -- --check
cargo clippy --workspace --all-features -- -D warnings
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

1. Add a struct with `clockless_chip!` (one-wire, NeoPixel-style) or `clocked_chip!` (data plus clock). Give it a doc comment in plain words that says what products use it, and cite the datasheet (name, revision and the table the timings come from). Set the channel order and the `[zero_high, zero_low, one_high, one_low]` phases as `ph(typical, min, max)` in nanoseconds, plus the reset time in microseconds.
2. Add the type to the `every_typical_value_is_inside_its_window` test in `chips.rs` (clockless chips only).
3. Add a test in `crates/ledkit-core/src/encode/spi.rs` that checks `ClocklessTiming::of::<YourChip>().spi_encoding(...)` at a common SPI clock (for example 3.2 MHz or 4 MHz), including the expected bit pattern.
4. Add a row to the "Which chip type do I use?" table in `crates/ledkit-core/src/lib.rs` (and the crate README) and to the table at the top of `chips.rs`.
5. Run the commands under "Build and test".

## Add a backend

Coming in a later milestone.

## Add an effect

Coming in a later milestone.
