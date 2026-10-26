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
```

The embedded targets come from `rustup target add thumbv6m-none-eabi riscv32imc-unknown-none-elf`.
See [AGENTS.md](AGENTS.md) for the code conventions.

## License of contributions

Unless you state otherwise, any contribution you submit is licensed under
MIT OR Apache-2.0, the same as the project, without additional terms.

## Add an LED type

Coming in a later milestone.

## Add a backend

Coming in a later milestone.

## Add an effect

Coming in a later milestone.
