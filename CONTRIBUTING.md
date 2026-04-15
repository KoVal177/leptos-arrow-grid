# Contributing to leptos-arrow-grid

Thank you for your interest in contributing!

## Development Setup

```bash
# Rust WASM target (one-time)
rustup target add wasm32-unknown-unknown

# Trunk web bundler (one-time)
cargo install trunk
```

## Running Tests

```bash
# Library unit tests (native)
cargo test -p leptos-arrow-grid --all-features

# Playground build check (validates WASM compilation)
cd examples/playground && cargo build --target wasm32-unknown-unknown
```

## Code Style

```bash
cargo fmt                                                  # format
cargo clippy --all-features -- -D warnings                 # lint
```

This project follows `rustfmt.toml` (edition 2024, max_width 100).
`clippy::unwrap_used` and `clippy::panic` are denied — use `.expect("reason")` instead.

## Running the Playground

```bash
cd examples/playground
trunk serve
# open http://localhost:8080
```

## Pull Request Checklist

- [ ] `cargo fmt` passes with no changes
- [ ] `cargo clippy --all-features -- -D warnings` is clean
- [ ] `cargo test -p leptos-arrow-grid --all-features` passes
- [ ] `cargo build --target wasm32-unknown-unknown` succeeds
- [ ] Public API changes are documented in `CHANGELOG.md` under `[Unreleased]`
- [ ] New public items have doc comments

## Reporting Bugs

Open an issue on [GitHub](https://github.com/KoVal177/leptos-arrow-grid/issues)
and include: Rust toolchain version, browser + OS, a minimal reproducible example,
and the full error or unexpected behaviour.
