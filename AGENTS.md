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
- Data version: `DATA_VERSION = 4440` (Minecraft 1.21.11) — see `crates/pigeon-data/src/lib.rs`.

## Regenerating vanilla data reports

The Mojang data generator dumps JSON reports for blocks, items, registries,
packets, biomes, commands, etc. These are checked into `crates/pigeon-data/reports/`
and embedded at compile time via `include_str!`.

Workflow (run from the workspace root):

1. Download `server.jar` from the Minecraft launcher (e.g. to `.tools/server.jar`).
2. Extract the inner server jar:

   ```sh
   jar xf .tools/server.jar META-INF/versions/1.21.11/server-1.21.11.jar
   ```

3. Run the data generator against the inner jar and the unpacked libraries:

   ```sh
   java -cp ".tools/server-inner.jar;libraries/<all *.jar>" \
     net.minecraft.data.Main --reports --server --all \
     --output .tools/reports
   ```

   Pass `-cp` via a single quoted `cmd /c` command — Java argfiles mis-parse
   paths containing whitespace (e.g. `PigeonMC v2`).

4. Copy the produced JSON into the crate:

   ```sh
   cp .tools/reports/blocks.json     crates/pigeon-data/reports/
   cp .tools/reports/items.json      crates/pigeon-data/reports/
   cp .tools/reports/registries.json  crates/pigeon-data/reports/
   cp .tools/reports/commands.json   crates/pigeon-data/reports/
   cp .tools/reports/packets.json     crates/pigeon-data/reports/
   cp .tools/reports/datapack.json     crates/pigeon-data/reports/
   cp .tools/reports/json-rpc-api-schema.json crates/pigeon-data/reports/
   cp -r .tools/reports/biome_parameters crates/pigeon-data/reports/
   ```

5. Bump `DATA_VERSION` in `crates/pigeon-data/src/lib.rs` if the world data
   version changed, then run `cargo test -p pigeon-data` to confirm the
   new JSON still parses against the typed view in the crate.
