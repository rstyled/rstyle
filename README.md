# rstyle

Rust style checker built on rustc HIR (resolved names and types, not just syntax). It flags functions that are too long, take too many params, mutate too much, or have a high side-effect density.

## Layout

| Crate | Role |
| --- | --- |
| `rstyle-core` | Pure rules, config, suppression, reporting. No IO, no compiler types. |
| `rstyle-driver` | rustc driver (`rustc_private`). Extracts per-function facts to JSON. |
| `cargo-rstyle` | `cargo rstyle` CLI. Runs `cargo check` with the driver as workspace wrapper, then evaluates. |

See [`SIDE_EFFECTS.md`](SIDE_EFFECTS.md) for the rule spec.

## Requirements

- `rustup`. The pinned nightly (`nightly-2026-09-30`, with `rustc-dev`, `llvm-tools-preview`, `rust-src`, `clippy`, `rustfmt`) installs automatically from `rust-toolchain.toml`.
- The toolchain is pinned because rustc-internal APIs change between nightlies. Bump deliberately and re-run the tests.

## Usage

Run on this workspace:

```sh
scripts/style.sh            # builds driver + CLI, then runs rstyle
cargo style                 # alias; needs `cargo build --workspace` once first
```

Run on another project:

```sh
cargo install --path crates/cargo-rstyle
cargo install --path crates/rstyle-driver   # driver must sit next to the CLI binary
cd your-project && cargo rstyle
```

Options:

```
cargo rstyle [-p PKG] [--all-targets] [--format text|json]
             [--config PATH] [--manifest-path PATH]
```

Exit codes: `0` clean, `1` violations, `2` tool error.

## Configuration

Drop an `rstyle.toml` in the project root (see the one in this repo for every key). All keys are optional, unknown keys are errors.

| Rule | Default | Meaning |
| --- | --- | --- |
| `fn_length.max_lines` | 50 | Code lines per fn (blank and comment lines excluded) |
| `fn_params.max_params` | 4 | Params per fn (`self` not counted by default) |
| `mutation_budget.max_mutations_per_target` | 3 | Writes / `&mut` borrows of one root |
| `mutation_budget.max_mutated_targets` | 3 | Distinct roots mutated in one fn |
| `side_effects.max_density_milli` | 500 | `1000 * sum(effect weights) / statements` |
| `exclude` | `["target/**"]` | Globs skipped entirely |

Callee classification (`[side_effects.paths]`) and effect weights (`[side_effects.weights]`) are configurable. Changing config never triggers a recompile.

## Suppression

On the line directly above the function, with a mandatory reason:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
fn shim(a: u32, b: u32, c: u32, d: u32, e: u32) { /* ... */ }
```

A missing reason or unknown rule id is reported as `invalid_suppression`.

## Development

```sh
cargo test --workspace      # core + CLI unit tests, e2e fixtures, self-check
scripts/style.sh            # dog-food: must report 0 violations
```

Fixtures live in `crates/cargo-rstyle/tests/fixtures/{clean,violations,broken}`. Golden output uses `insta`.
