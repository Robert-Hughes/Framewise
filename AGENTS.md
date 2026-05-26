# Agents

## Workspace structure

Two crates in the Cargo workspace:

- `framewise/` — the library crate (`framewise`). A Rust GUI library where the app is always in control.
- `sample/` — sample application binary that depends on `framewise` and exercises it end-to-end.

## Commands

Run from the workspace root. Commands apply to both crates unless scoped.

```sh
# Tests
cargo test

# Lints
cargo clippy

# Formatting
cargo fmt
```

## Code quality expectations

Both `framewise` and `sample` must stay clean:

- No `cargo clippy` warnings in either crate.
- `cargo fmt` applied — no formatting diffs.
- `cargo test` passes.

When making changes to `framewise`, keep `sample` compiling and lint-clean. The sample app is not throwaway — treat it with the same care as the library.
