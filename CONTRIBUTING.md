# Contributing to ROSQL

Thanks for your interest in contributing to ROSQL!

## Prerequisites

- **Rust** (stable) — [rustup.rs](https://rustup.rs)
- **protoc** — required for proto code generation (`apt-get install protobuf-compiler` on Debian/Ubuntu, `brew install protobuf` on macOS)
- **buf** (optional) — for proto linting (`buf lint proto/`). Install from [buf.build](https://buf.build/docs/installation)
- **just** (optional) — command runner. Install from [just.systems](https://just.systems)

## Getting started

```sh
git clone https://github.com/RobotOpsInc/rosql
cd rosql
just build       # or: cargo build
just test        # or: cargo test
```

## Build variants

```sh
# Default: parser + drivers (no networking, no WASM)
cargo build

# WASM package (for frontend editors)
cargo build --target wasm32-unknown-unknown --features wasm

# gRPC server + CLI binary
cargo build --features server --bin rosql
```

## Running checks

```sh
just check       # runs build + test + clippy + fmt + buf-lint
```

Or individually:

```sh
cargo test
cargo clippy -- -D warnings
cargo fmt --check
buf lint proto/
```

## Proto development

Proto files live in `proto/rosql/v1/`. When you modify a `.proto` file:

1. `cargo build` — the `build.rs` script auto-regenerates Rust types via `prost-build`
2. `buf lint proto/` — validate proto style compliance
3. `cargo test` — ensure generated types compile and tests pass

The generated Rust types (`src/proto/rosql.v1.rs`) are not committed — they're regenerated on every build.

## Submitting changes

1. Fork the repo and create a feature branch from `development`
2. Make your changes
3. Run `just check` (or the individual commands above) and ensure everything passes
4. Open a pull request against `development`

## Releasing

Releases are created via a manual GitHub Actions workflow. Only maintainers can trigger releases.

1. **Bump the version** in `Cargo.toml` (e.g. `version = "0.2.0"`)
2. **Add a CHANGELOG entry** under `## [0.2.0] - YYYY-MM-DD` in `CHANGELOG.md`
3. **Merge to `main`** via a PR through `development`
4. **Trigger the release** — go to Actions → Release → Run workflow (from `main`)

The workflow will:
- Create a git tag `v0.2.0`
- Create a GitHub Release with the CHANGELOG notes
- Build Linux binaries (x64 + arm64) and attach them to the release
- Publish the `rosql` crate to [crates.io](https://crates.io/crates/rosql)
- Publish the `@robotops/rosql` WASM package to [npm](https://www.npmjs.com/package/@robotops/rosql)

If the version hasn't been bumped (tag already exists), the workflow will fail — this is intentional.

## Reporting issues

File bugs and feature requests in the [issue tracker](https://github.com/RobotOpsInc/rosql/issues).

## Contact

Questions? Email [kristophm@robotops.com](mailto:kristophm@robotops.com).

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 license.
