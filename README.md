# rstyle

rstyle finds the code worth refactoring. It works from what the compiler knows, resolved names and
types rather than just syntax, and it analyzes Rust and Go.

- **Style checks** flag functions that are too long, too complex, take too many parameters, mutate
  too much, or have a high density of side effects.
- **Fix groups** tell you which of those findings can be fixed independently, and which must land
  together.
- **Pattern mining** finds code written as parallel copies that could be one trait, generic function
  or parameter.
- **Runtime bounds** give symbolic upper bounds on how often loops and calls run, such as
  `len(xs) * len(ys)`.
- **Control flow** reports dominators and data and control dependence for each function.
- **Defects** point out what a function's own flow shows is probably wrong, such as a computed value
  nothing ever reads.

> rstyle is in alpha and is installed from source. It depends on rustc internals, so it needs a
> specific nightly toolchain, which the installer sets up for you.

## Install

You need [`rustup`](https://rustup.rs) and a native build toolchain, such as Xcode Command Line Tools
on macOS. Then run:

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/rstyled/rstyle/HEAD/scripts/install.sh | sh
```

This installs the pinned nightly, then compiles `rstyle`, `cargo-rstyle` and `rstyle-driver` into
Cargo's binary directory (normally `~/.cargo/bin`, which must be on your `PATH`). Compiling takes a
while. The installer doesn't install `rustup`, use `sudo` or change your shell configuration.
To read [the script](scripts/install.sh) first, download it instead of piping it to `sh`.

To update, run the same command with `sh -s -- --force` in place of the final `sh`.

Keep the three binaries in the same directory, and keep the nightly installed: the driver links
against its compiler libraries.

<details>
<summary>Install without the script</summary>

```sh
rustup toolchain install nightly-2026-09-30 --profile minimal \
  --component rustc-dev \
  --component llvm-tools-preview \
  --component rust-src

cargo +nightly-2026-09-30 install \
  --git https://github.com/rstyled/rstyle.git \
  --locked \
  cargo-rstyle rstyle-driver
```

Add `--force` to replace an existing installation. `CARGO_INSTALL_ROOT` and `CARGO_HOME` choose the
install directory, for the script too.

</details>

## Quick start

From the Rust project you want to analyze:

```sh
cargo rstyle
```

```text
src/lib.rs:5:1: fn_params: function `total` has too many parameters (actual 5, limit 4) [diagnostic 0] [compiler unit 0]
src/lib.rs:9:1: fn_spacing: function `checkout` has no blank line between its phases (actual 5, limit 4) [diagnostic 1] [compiler unit 0]
src/lib.rs:9:1: mutation_per_target: function `checkout` mutates one target too many times (actual 4, limit 3) [diagnostic 2] [compiler unit 0]
fix groups: 1 (1 stacked, 0 independent)
  group 1 (2 functions; land together or stack in this order):
    src/lib.rs:5 total [fn_params]
    src/lib.rs:9 checkout [fn_spacing, mutation_per_target]
      1. signature total (src/lib.rs:5; 1 incoming, 0 outgoing sites)
      2. call sites checkout (src/lib.rs:9; 0 incoming, 1 outgoing sites)
...
rstyle: 3 functions, 3 violations, mean density 66, max density 200, complexity 3 measured, 0 unavailable
```

Fix one group at a time, callees first, then run it again, since fixes change the call graph.

## Usage

| Command | What it does |
| --- | --- |
| `cargo rstyle` | Style findings, fix groups and complexity scores for a Rust workspace |
| `cargo rstyle patterns` | Candidates for a shared trait, generic function or parameter |
| `cargo rstyle patterns --complexity` | The same, plus runtime bounds |
| `cargo rstyle ignore <id>` | Dismiss a pattern candidate you reviewed |
| `rstyle rust` | Every analysis over a Rust workspace, saved with SARIF reports |
| `rstyle go` | Every analysis over a Go module, saved with SARIF reports |
| `rstyle cxx` | Every analysis over a C or C++ project, saved with SARIF reports |

`rstyle` and `cargo rstyle` are the same program, so either name works for any command. `rstyle go`,
`rstyle cxx` and `rstyle rust` also take `--check` or `--patterns` to run just that analysis.

Common options:

| Option | Meaning |
| --- | --- |
| `--format json` | Machine-readable output on stdout |
| `-p PKG` | Only analyze this package |
| `--all-targets` | Include tests, examples and benches |
| `--config PATH` | Use this file instead of `<workspace root>/rstyle.toml` |
| `--manifest-path PATH` | Analyze the workspace of this `Cargo.toml` |

To teach a coding agent how to use rstyle, `--skill` prints a `SKILL.md` for it. For Cursor:

```sh
mkdir -p .cursor/skills/rstyle && cargo rstyle --skill > .cursor/skills/rstyle/SKILL.md
```

### Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Clean |
| `1` | Findings |
| `2` | Tool error or incomplete analysis |

### Go

`rstyle go` uses `$RSTYLE_GO` if set, then `rstyle-go` from Go's binary directory or your `PATH`, and
otherwise runs `go install`. `rstyle-go` is private for now, so set `GOPRIVATE=github.com/rstyled` if
you have access to it, or point `$RSTYLE_GO` at a local build.

### C and C++

`rstyle cxx` uses `$RSTYLE_CXX` if set, then `rstyle-cxx` on your `PATH`. It's built against LLVM 19, so
rstyle doesn't install it: build it from [rstyle-cxx](https://github.com/rstyled/rstyle-cxx), which is
private for now. It reads your project through Clang, so it needs your build's `compile_commands.json`. It looks in
the project and its build folders, and otherwise asks your CMake, Meson or Make build for one, which runs its build
scripts: pass `--no-generate-database` on code you wouldn't build. `--compilation-database` names one elsewhere. If it
ends up with none it warns, says how to specify one, and goes on guessing flags, which loses what it can't parse.

## Rules

| Rule | Flags | Setting | Default |
| --- | --- | --- | --- |
| `fn_length` | Code lines in a function, not counting blanks and comments | `fn_length.max_lines` | 50 |
| `fn_params` | Parameters, not counting `self` | `fn_params.max_params` | 4 |
| `fn_spacing` | A body of this many statements with no blank line | `fn_spacing.min_statements` | 5 |
| `cyclomatic_complexity` | Cyclomatic score | `complexity.max_cyclomatic` | 12 |
| `cognitive_complexity` | Cognitive score, which also charges nesting | `complexity.max_cognitive` | 12 |
| `mutation_per_target` | Writes or `&mut` borrows of one root | `mutation_budget.max_mutations_per_target` | 3 |
| `mutated_targets` | Distinct roots mutated in one function | `mutation_budget.max_mutated_targets` | 3 |
| `side_effect_density` | Weighted effects per statement, times 1000 | `side_effects.max_density_milli` | 500 |

[Style checks](docs/style-checks.md) describes each rule and every setting.

## Configuration

Put an `rstyle.toml` in the project root. Every key is optional, and an unknown key is an error, so a
typo can't silently turn a rule off.

```toml
exclude = ["target/**", "src/generated/**"]

[fn_length]
max_lines = 80

[complexity]
max_cognitive = 20
```

## Suppressing a finding

Put a comment on the line directly above the function. The reason is required:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
fn shim(a: u32, b: u32, c: u32, d: u32, e: u32) { /* ... */ }
```

A missing reason or an unknown rule id is itself reported, as `invalid_suppression`. Use the rule id
`abstraction` to keep a function out of pattern mining.

## Using it in CI

To adopt rstyle on an existing codebase, record today's findings once and commit the file. That run
exits 1 if there are findings, and the baseline is still written.

```sh
cargo rstyle --write-baseline rstyle-baseline.json
```

Then fail only on what's new:

```sh
cargo rstyle --baseline rstyle-baseline.json --fail-on-new
```

To gate on pattern candidates too, fix each one or dismiss it into `rstyle-ignore.toml`, and commit
that file:

```sh
cargo rstyle patterns --fail-on-candidates
```

For code scanning tools, `rstyle rust`, `rstyle go` and `rstyle cxx` write SARIF 2.1.0 to the run's `report.sarif`
under `.rstyle/runs/`. [Style checks](docs/style-checks.md#using-it-in-ci) also covers checking only
the files a change touches.

## Documentation

| Guide | Covers |
| --- | --- |
| [Style checks](docs/style-checks.md) | Rules, every setting, fix groups, parameter refactor plans, baselines and reviews |
| [Pattern mining](docs/patterns.md) | How candidates are found and scored, and the ignore ledger |
| [Runtime bounds](docs/runtime-bounds.md) | What `--complexity` models, and how to read its results |
| [Runner and frontends](docs/frontends.md) | `rstyle go`, `rstyle cxx` and `rstyle rust`, saved runs, the `cpg` commands, and writing a frontend for another language |
| [CPG storage and transport](docs/cpg-transport.md) | Document storage, sealed captures and the local pipe protocol |
| [Scripts](scripts/README.md) | Build, test, profiling and evaluation scripts |

## How it works

Each language has a frontend that writes `compat.cpg` documents, a code property graph with declared
coverage. Everything else happens in one place: `rstyle-core` derives rule facts, dependence graphs
and runtime-bound programs from those documents, so no analysis is written per language and a saved
run can be analyzed again without a compiler or the sources.

| Crate | Role |
| --- | --- |
| [`rstyle-core`](crates/rstyle-core) | Rules, analyses, config, suppression and reporting. No compiler types. |
| [`rstyle-driver`](crates/rstyle-driver) | The Rust frontend: a rustc driver that writes each crate's documents. |
| [`cargo-rstyle`](crates/cargo-rstyle) | The `cargo rstyle` and `rstyle` commands, which run frontends and report results. |
| [`rstyle-capture`](crates/rstyle-capture) | What the CLI and the driver agree on about one captured unit. |

## Platforms

CI runs the full suite on Linux (x86-64 and arm64) and macOS. On Windows it runs a subset: some
`rstyle-core` modules, the pipe a frontend inherits, and `rstyle rust` and `cargo rstyle` through the
driver.

## Development

In a checkout, `rust-toolchain.toml` installs the pinned nightly automatically. It's pinned because
rustc's internal APIs change between nightlies, so bump it deliberately and re-run the tests.

```sh
scripts/install-hooks.sh     # once per clone: fmt, clippy and rstyle before each commit
scripts/style.sh             # build, then run rstyle on this workspace; must report 0 violations
scripts/test.sh              # cargo test
scripts/test.sh --nextest    # what CI runs; needs cargo-nextest
```

Once `cargo build --workspace` has run, `cargo style` is a shorthand for the second command. The
install script always builds from GitHub, even when run from a checkout, so to try local changes on
another project, use:

```sh
cargo install --locked --path crates/cargo-rstyle
cargo install --locked --path crates/rstyle-driver
```

Test fixtures live in [`crates/cargo-rstyle/tests/fixtures`](crates/cargo-rstyle/tests/fixtures), and
the tests have [their own guide](crates/cargo-rstyle/tests/README.md).
[`rust_refactoring_examples`](rust_refactoring_examples) holds the labeled crates the pattern miner is
scored against.
