# Pigeon build & lint commands
## Build

```sh
cargo build --workspace
```

## Lint (clippy with warnings as errors)

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

## Test

```sh
cargo test --workspace
```

## Format check

```sh
cargo fmt --all -- --check
```

## Format apply

```sh
cargo fmt --all
```

## Run server

```sh
cargo run -p pigeon
```

## Notes

- Rust toolchain: stable, edition 2021, MSRV 1.75.
- Target Minecraft Java: 1.21.11 (protocol TBD — see pigeon-protocol).
- Always run `cargo fmt`, then `cargo clippy -- -D warnings`, then `cargo build` before committing.
