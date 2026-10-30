# rstyle

Style, patterns and review commands now use the compiler pipeline exclusively.
See [breaking changes](BREAKING_CHANGES.md) for removed selectors and capture
interfaces. The pre-removal implementation is preserved on the `legacy` branch.

Rust style checker built on rustc HIR (resolved names and types, not just syntax). It flags functions that are too long, take too many params, mutate too much, or have a high side-effect density.

## Layout

| Crate | Role |
| --- | --- |
| `rstyle-core` | Pure rules, config, suppression, reporting. No IO, no compiler types. |
| `rstyle-driver` | rustc driver (`rustc_private`). Extracts per-function facts to JSON. |
| `cargo-rstyle` | `cargo rstyle` CLI. Runs `cargo check` with the driver as workspace wrapper, then evaluates. |

See [`SIDE_EFFECTS.md`](SIDE_EFFECTS.md) for the rule spec.

Opt-in strict source-to-pair analysis is available with `cargo rstyle clones`.
See [the Rust feature coverage matrix](docs/rust-feature-coverage.md) for current
support, examples, and the remaining real-world Rust roadmap.
See [the integration workflow](docs/source-to-pair.md) for build selection,
verified evidence, reports and exit codes. It does not activate abstraction mining.
Clones also offers opt-in [source and signature rules](docs/clones-source-rules.md)
whose evidence remains usable when a body is excluded from strict matching.
[Compiler advisory](docs/clones-compiler-advisory.md) reuses existing Rust
collectors and core policy. Add `--compiler-advisory --compiler-complexity-top 20`
to rank clean and flagged functions by completed COG/CC scores per unit;
missing scores and owners remain explicit, including on saved-facts replay.
Normal style checks use that compiler path by default:
`cargo rstyle --format json`. It reads workspace
`rstyle.toml` (or `--config`), checks all authored rules and returns 1 for findings.
The compiler path reuses private builds and completed owner observations when Cargo
reports the exact artifacts fresh; reports retain separate compilation contexts.
Root JSON exposes diagnostics, summary, fix groups and complexity rows, with
explicit scope and planning completeness. Root style saves its numeric compiler
inputs for replay; analyses requiring original HIR bodies use a full clones capture.
Text diagnostics and complexity rows carry compiler unit labels, followed by a
table of package, target, test mode and source for each unit. Library and test
builds of the same source remain distinguishable on saved-run replay.
Add `--changed-files changes.json` to compiler style to select findings for CI;
use `{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}` with paths
relative to the workspace root. Metrics and fix groups retain the full captured
scope. Invalid suppressions remain visible, and unobserved selected paths or
missing required inputs still return 2. This selection also works with an explicit
`--replay-run PATH`.
Compiler style also supports `--write-baseline baseline.json` to export a complete
inventory to a new file, and `--baseline baseline.json --fail-on-new` to fail CI
only for new/changed selected findings or policy/build-context drift. Raw findings
remain visible, and incomplete analysis still exits 2. Export retains the normal
finding exit status. Both workflows support explicit saved-run replay.
Density findings remain blocking, just like other violations. Dismiss individual
reviewed findings with the existing `rstyle-allow(side_effect_density)` source
comment and a reason, or use the existing separate review file without changing
source:

```sh
cargo rstyle --format json
cargo rstyle --replay-run RUN --review-file rstyle-reviews.json \
  --dismiss-compiler-diagnostic UNIT DIAGNOSTIC_INDEX --review-reason "Reviewed IO boundary"
cargo rstyle --review-file rstyle-reviews.json --format json
```

Use the `run_directory` and parallel `diagnostic_units`/`diagnostic_indices`
entries from the report (also shown in text). Reviews hide only the exact finding;
changed values, policy or build context can make it visible again. Raw per-unit
findings remain available. `--review-status` inspects the file without writing.
The review file is explicit and independent of the pattern ignore ledger.

Default `cargo rstyle patterns` also saves metadata and its advisory graphs,
avoiding full body trees. Pattern and review replay need no source or compiler
tools; body-dependent analysis requires a full clones capture.
Optional [native pair explanations](docs/clones-pair-differences.md) show ordered
scalar differences with separate coverage through `--explain-pairs`.
Bounded [helper proposals](docs/clones-helper-proposals.md) use that evidence
with `--helper-proposals` to suggest useful scalar-expression abstractions.
[Saved finding reviews](docs/clones-finding-review.md) retain explicit dismissals
for helpers, enum dispatch and witnessed capability sets across analysis runs.
`--review-status` inspects every retained entry with explicit scope and evidence gaps.
`--review-prune --review-prune-dry-run` previews removal of completed negative reviews.

Other top-level folders:

| Folder | What it is |
| --- | --- |
| [`eval/`](eval/README.md) | Result CSVs from the miner evaluation (leave-one-project-out, normalizer ablation, alignment, CodeNet). |
| [`scripts/`](scripts/README.md) | Shell and Python (stdlib only) drivers that produce `eval/` and `site/wl/data`; also `style.sh`. |
| [`site/wl/`](site/wl/README.md) | Static viewer for Weisfeiler-Lehman traces of function pairs. |
| `rust_refactoring_examples/` | Labeled example crates (`labels.csv`) the miner is scored against. |
| `.plans/` | Local planning notes and LLM logs. Gitignored, not part of the repo. |

## Requirements

- `rustup` and a native build toolchain (for example, Xcode Command Line Tools on macOS).
- When working in a checkout, the pinned nightly (`nightly-2026-09-30`, with `rustc-dev`, `llvm-tools-preview`, `rust-src`, `clippy`, `rustfmt`) installs automatically from `rust-toolchain.toml`. For installation directly from GitHub, install the toolchain explicitly as shown below.
- The toolchain is pinned because rustc-internal APIs change between nightlies. Bump deliberately and re-run the tests.

## Install from GitHub (alpha)

No manual clone or existing local checkout is needed. With `rustup` and a native
build toolchain installed, download and run the [installer](scripts/install.sh)
from any directory:

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/rstyled/rstyle/HEAD/scripts/install.sh | sh
```

This executes the downloaded script directly, so only use it with a repository
you trust. To inspect it first, download the script instead of piping it to `sh`.
The installer installs the pinned nightly and both binaries; it does not install
`rustup`, use `sudo`, or change your shell configuration.

To replace an existing installation:

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/rstyled/rstyle/HEAD/scripts/install.sh | sh -s -- --force
```

From an existing checkout, use `sh scripts/install.sh`. The installer always
installs from GitHub, not from local source changes. Add `--force` to replace an
existing installation, or use `--help` for details. `CARGO_INSTALL_ROOT` and
`CARGO_HOME` are respected when choosing the installation directory.

Alternatively, install manually:

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

Cargo automatically downloads the repository into its Git cache (normally under
`~/.cargo/git`), resolves dependencies, compiles both binaries on your machine,
and installs them side by side in Cargo's binary directory (normally
`~/.cargo/bin`). It does not create a project checkout in your current directory.
This is a source-based installation, not a prebuilt binary download; it needs
network access, disk space, and time to compile.

Ensure Cargo's binary directory is on `PATH`, then run from the Rust project you
want to analyze:

```sh
cargo rstyle
```

Both binaries are required. The driver links against the pinned toolchain's
compiler libraries, so keep that toolchain installed after installation. Building
locally lets the driver's runtime library path point to your own toolchain; no
release-library repackaging is needed for this installation method.

The command installs the repository's default branch. For a published alpha tag,
add `--tag TAG` to the `cargo install` command, replacing `TAG` with the actual
release tag, and use the nightly pinned by that revision. To update, rerun the
installation commands; add `--force` to `cargo install` if you need to replace an
existing installation.

## Fix groups

`cargo rstyle` also tells you which violations can be fixed independently:

```
fix groups: 693 (92 stacked, 601 independent)
  group 4 (2 functions; land together or stack in this order):
    src/a.rs:10 a::callee [fn_params]
    src/b.rs:42 b::caller [fn_length]
```

- A group is a set of violations whose fixes overlap. Put them in one PR, or stack the PRs in the listed order (callees first). Different groups never conflict, so they can go in parallel.
- The overlap model: a fix edits the function itself, and for `grouping.caller_rules` (default `fn_params`) also its callers, up to `grouping.caller_hops` (default 1). The call graph resolves trait-method calls to every impl.
- `--format json` has the full `fix_groups` array. Groups carry soft `hints` (`shared_type`, `shared_caller`): other groups that touch the same mutated type or the same caller. They are not reasons to stack. Regroup after each merge; fixes change the graph.
- Why not widen the footprint? On rust-lang/cargo, counting density callers and mutated types turned 519 of 908 violating functions into one group. `fn_params`-only gives 693 groups, largest 89.

### Parameter refactor plans

Groups with a `fn_params` member carry a `plan` (JSON `fix_groups[].plan`, and under the group in text output):

```
  group 172 (...)
      1. signature Dependency::from_toml (src/.../dependency.rs:224; 5 incoming, 0 outgoing sites)
      2. call sites find_workspace_dep (src/.../mod.rs:1372; 3 incoming, 1 outgoing sites; pass-through: unstable_features)
      bundle (1 sites): key: &str, unstable_features: &Features
```

- Steps: re-signed functions callee-first, then the callers whose call sites must follow. `incoming` = call sites into the function, `outgoing` = its call sites into re-signed functions.
- `pass-through`: parameters whose every mention is a forward into a re-signed function. The caller doesn't need them; fix in the callee or an upper layer and drop them from the caller.
- `bundle`: parameters forwarded together (same callee `name: type`, 2+) at N call sites: a params-struct candidate. Only closed sets are reported (a subset with the same support as a superset is dropped).
- Argument provenance (`param(i)`, `const`, `field`, `other`) and each parameter's name/type/use count are in the facts files (`param_list`, `edges[].args`).

## Usage

Run on this workspace:

```sh
scripts/style.sh            # builds driver + CLI, then runs rstyle
cargo style                 # alias; needs `cargo build --workspace` once first
```

The same command reports style findings, fix groups, and cyclomatic (CC) and
cognitive (COG) complexity for functions with findings:

```sh
cargo style
cargo style --format json
```

Both complexity limits default to 12; scores above either limit are findings.
Clean functions are omitted. Text ranks reported functions by COG, then CC. JSON
includes source evidence, all findings, and counts for every analyzed function. See
[the shared scoring policy and Rust mapping](docs/complexity.md) for the counting
rules, coverage, frontend contract, and configuration.

Run on another project:

```sh
cargo install --path crates/cargo-rstyle
cargo install --path crates/rstyle-driver   # driver must sit next to the CLI binary
cd your-project && cargo rstyle
```

Options:

```
cargo rstyle [-p PKG] [--all-targets] [--format text|json]
             [--config PATH] [--manifest-path PATH] [--skill]
```

`--skill` prints an agent skill (`SKILL.md`: how to run, read JSON, fix by group, suppress) and exits 0 without analyzing. Install it for Cursor:

```sh
mkdir -p .cursor/skills/rstyle && cargo rstyle --skill > .cursor/skills/rstyle/SKILL.md
```

Exit codes: `0` clean, `1` violations, `2` tool error.

## Abstraction mining (`patterns`)

`cargo rstyle patterns` looks for *latent shared abstractions*: code that is written as parallel copies but would be one trait, generic fn or parameter.

```
cargo rstyle patterns [--format text|json] [--all-targets] [--min-score N] [--top N]
                      [--hide-implements TEXT]...
```

The default uses compiler observations across the selected workspace and reuses the core proposer. Authored functions are selected by default; generated signatures remain visible in coverage. Reports retain compilation contexts, unavailable inputs and the explicit saved run directory; incomplete input exits 2. Strict clone matching is independent of these advisory proposals.

Two layers:

1. **Signature groups.** Methods of different types with the same normalized signature (`fn(&Self) -> String`). Rare signatures score high, common getters score near zero. This is a blocking key, not a finding.
2. **Dependence-graph candidates.** The driver records a dependence graph per function (data, control and operand-position edges; `for`, `?` and macros are single nodes; call nodes are labeled by the callee's signature class, not its name). A Weisfeiler-Lehman kernel clusters functions with the same structure, an alignment step produces the *holes* (`T0 = Dog | Cat`, `M0 = Dog::bark | Cat::meow`), and method holes are joined back to the signature groups. `for` loops and `Iterator::for_each` closures are the same iteration; a `match` whose arms call analogous methods is reported as `enum_dispatch`.

Each candidate has a `kind` (`trait_method`, `capability_set`, `enum_dispatch`, `generic_fn`, `parameterize`), a score with its parts (`support`, `lift`, `holes`, `coverage`), an observation, an inference, a possible refactor sketch, every site with `path:line`, the functions that fill the holes, and counter-evidence (effect mismatch, common signature). Patterns whose implementations all already implement one trait method are listed under `suppressed`, not proposed. Output is deterministic.

Compiler patterns captures graph observations under `target/rstyle/clones/`. Use `--replay-run PATH` to reevaluate an explicit saved run without source files or compiler tools. Style and patterns both consume saved compiler unit snapshots. See [compiler advisory coverage](docs/clones-compiler-advisory.md) for scope and replay details.

`--hide-implements TEXT` forgets trait links whose trait method id contains `TEXT` (`*` hides all). Evaluation oracle: hide a trait that has several impls and check the miner proposes it back.

Opt a function out of mining with `// rstyle-allow(abstraction): reason` on the line above it.

**Reviewed candidates (the ignore ledger).** Every candidate prints an `id`. After you review one and decide not to merge it, dismiss it:

```
cargo rstyle ignore <id> [--allow-growth] [--reason TEXT]   # writes rstyle-ignore.toml
cargo rstyle patterns --show-ignored                        # list what the ledger hid
cargo rstyle patterns --no-ignore                           # one-off full view
cargo rstyle patterns --fail-on-candidates                  # CI gate: exit 1 if any candidate is left
cargo rstyle ignore --status [<id>]                         # where each entry's functions are now
cargo rstyle ignore --all-targets --prune [--dry-run]       # drop entries that match nothing
```

`rstyle-ignore.toml` (workspace root, or `--ignore-file`) is meant to be committed, like a lockfile. It is a final filter on the reported candidates; mining never reads it, and `--top` / `--min-score` apply after it. Delete the file, or an entry, to re-review. Comments directly above an entry, or inside it, are kept when `ignore` rewrites the file (other comments are dropped), so notes like "discussed in PR 12" survive; `--reason` is the structured version.

Listing and `ignore` share compiler discovery and mining options. Pruning requires complete full-workspace inputs captured with `--all-targets`; saved replay uses that recorded selection. Status reports absent members as unobserved within each compilation population rather than assuming deletion.

A dismissal holds while the candidate is the same: same kind, same functions, same dependence-graph shape of each site. Moving a function (new line numbers) or adding an impl block above it does not matter; editing its structure does. Growth resurfaces it by default: if two more functions join a dismissed candidate, it is a new candidate. `--allow-growth` records the members instead, and the candidate stays hidden as long as they are all still in it. Pruning is manual; stale entries are harmless, they just never match. Pass the same clustering flags to `ignore` as to `patterns`, since they decide which candidates (and ids) exist. Limits: shape comes from the dependence graph, so edits inside closures or that only swap one callee for another of the same signature are not seen.

**Reading the score.** `score = (ln(support) + lift) * coverage`, divided by `holes / 3` when there are more than 3 holes, halved on effect mismatch (natural-log units; a ranking, not a probability). `support` is the number of functions in the cluster, `lift` is how rare the shared signature is (`ln(methods in the workspace / methods sharing it)`; about 0 for getters, with up to +50% when the types share the method name), `coverage` is the mean share of each body that aligns, `holes` is the number of differing columns. `generic_fn` is halved and `parameterize` divided by 3 and both have lift 0, so compare scores within a kind. For the trait kinds a score under about 0.7, or the counter-evidence "signature is common in this codebase", is noise. These cutoffs come from the formula and from reading output, they are not calibrated on labeled data.

**Experiments (off by default).** `--normalize all|ctor,pred,case,exit,counter,cmp,try` canonicalizes graphs before comparing, `--pair-coverage N` clusters pairs by alignment coverage even below the WL threshold, `--max-holes N` raises the `parameterize` hole cap, `--single-call-guard` admits single-call bodies that do more than forward parameters, `--threshold N` and `--min-coverage N` move the clustering thresholds (defaults 600 and 600). On a held-out CodeNet sample none of them is clearly better than the defaults: the rules shrink graphs by about one node per pair and `case` does nearly all of it; `--pair-coverage 650 --max-holes 4` flags more pairs of strangers and more cross-problem pairs (see [`eval/README.md`](eval/README.md)).

Inspiration: comparing functions by approximate matching of their program dependence graphs (PDGs), as in [CCGraph: A PDG-based Code Clone Detector with Approximate Graph Matching](https://yinxingxue.github.io/papers/ase2020_CCGraph%20A%20PDG%20based%20Code%20Clone%20Detector%20With%20Approximate%20Graph%20Matching.pdf) (ASE 2020), with a Weisfeiler-Lehman (WL) graph kernel for the similarity. This is not a reimplementation.

Known limits: function-level only; closure bodies are opaque (except `for_each`); a cluster mixing already-trait and inherent impls is reported as a candidate rather than as "implement the existing trait"; the size floor is 4 nodes and 1 call, so small functions never cluster (on competitive-programming code that is more than half of edited-copy pairs), and the defaults were picked on a few real crates and a labeled example set, not tuned on a large corpus.

## Configuration

Drop an `rstyle.toml` in the project root (see the one in this repo for every key). All keys are optional, unknown keys are errors.

| Rule | Default | Meaning |
| --- | --- | --- |
| `fn_length.max_lines` | 50 | Code lines per fn (blank and comment lines excluded) |
| `fn_params.max_params` | 4 | Params per fn (`self` not counted by default) |
| `complexity.max_cyclomatic` | 12 | Cyclomatic score; report values above this limit |
| `complexity.max_cognitive` | 12 | Cognitive score; report values above this limit |
| `fn_spacing.min_statements` | 5 | Bodies with this many top-level statements need a blank line somewhere (0 disables) |
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

`abstraction` (for `patterns`) is also a valid rule id. A missing reason or unknown rule id is reported as `invalid_suppression`.

## Development

```sh
cargo test --workspace      # core + CLI unit tests, e2e fixtures, self-check
./scripts/test.sh --nextest # CI runner: four parallel tests, then all doctests
scripts/style.sh            # dog-food: must report 0 violations
python3 scripts/test_codenet.py   # tests of the evaluation scripts (fake CodeNet package, needs cargo)
```

Evaluation scaffolding (all deterministic, dependency-free Python, outputs under gitignored `eval/codenet/`): see [`scripts/README.md`](scripts/README.md) for the scripts and the dev / test / holdout protocol, and [`eval/README.md`](eval/README.md) for the result files.

Fixtures live in `crates/cargo-rstyle/tests/fixtures/{clean,violations,broken}`. Golden output uses `insta`.
