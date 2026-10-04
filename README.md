# rstyle

Rust style checker built on rustc HIR (resolved names and types, not just syntax). It flags functions that are too long, take too many params, mutate too much, or have a high side-effect density.

## Layout

| Crate | Role |
| --- | --- |
| `rstyle-core` | Pure rules, config, suppression, reporting. No IO, no compiler types. |
| `rstyle-driver` | rustc driver (`rustc_private`). Extracts per-function facts to JSON. |
| `cargo-rstyle` | `cargo rstyle` CLI. Runs `cargo check` with the driver as workspace wrapper, then evaluates. |

See [`SIDE_EFFECTS.md`](SIDE_EFFECTS.md) for the rule spec.

Other top-level folders:

| Folder | What it is |
| --- | --- |
| [`eval/`](eval/README.md) | Result CSVs from the miner evaluation (leave-one-project-out, normalizer ablation, alignment, CodeNet). |
| [`scripts/`](scripts/README.md) | Shell and Python (stdlib only) drivers that produce `eval/` and `site/wl/data`; also `style.sh`. |
| [`site/wl/`](site/wl/README.md) | Static viewer for Weisfeiler-Lehman traces of function pairs. |
| `rust_refactoring_examples/` | Labeled example crates (`labels.csv`) the miner is scored against. |
| `.plans/` | Local planning notes and LLM logs. Gitignored, not part of the repo. |

## Requirements

- `rustup`. The pinned nightly (`nightly-2026-09-30`, with `rustc-dev`, `llvm-tools-preview`, `rust-src`, `clippy`, `rustfmt`) installs automatically from `rust-toolchain.toml`.
- The toolchain is pinned because rustc-internal APIs change between nightlies. Bump deliberately and re-run the tests.

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

Two layers:

1. **Signature groups.** Methods of different types with the same normalized signature (`fn(&Self) -> String`). Rare signatures score high, common getters score near zero. This is a blocking key, not a finding.
2. **Dependence-graph candidates.** The driver records a dependence graph per function (data, control and operand-position edges; `for`, `?` and macros are single nodes; call nodes are labeled by the callee's signature class, not its name). A Weisfeiler-Lehman kernel clusters functions with the same structure, an alignment step produces the *holes* (`T0 = Dog | Cat`, `M0 = Dog::bark | Cat::meow`), and method holes are joined back to the signature groups. `for` loops and `Iterator::for_each` closures are the same iteration; a `match` whose arms call analogous methods is reported as `enum_dispatch`.

Each candidate has a `kind` (`trait_method`, `capability_set`, `enum_dispatch`, `generic_fn`, `parameterize`), a score with its parts (`support`, `lift`, `holes`, `coverage`), an observation, an inference, a possible refactor sketch, every site with `path:line`, the functions that fill the holes, and counter-evidence (effect mismatch, common signature). Patterns whose implementations all already implement one trait method are listed under `suppressed`, not proposed. Output is deterministic.

Dependence graphs are about 10x bigger than the style facts, so only `patterns` asks the driver for them (`RSTYLE_PDG=1`, facts in `target/rstyle/facts-sem`; style runs keep `target/rstyle/facts` and a separate cargo fingerprint, so the modes never clobber each other).

`--hide-implements TEXT` forgets trait links whose trait method id contains `TEXT` (`*` hides all). Evaluation oracle: hide a trait that has several impls and check the miner proposes it back.

Opt a function out of mining with `// rstyle-allow(abstraction): reason` on the line above it.

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
scripts/style.sh            # dog-food: must report 0 violations
python3 scripts/test_codenet.py   # tests of the evaluation scripts (fake CodeNet package, needs cargo)
```

Evaluation scaffolding (all deterministic, dependency-free Python, outputs under gitignored `eval/codenet/`): see [`scripts/README.md`](scripts/README.md) for the scripts and the dev / test / holdout protocol, and [`eval/README.md`](eval/README.md) for the result files.

Fixtures live in `crates/cargo-rstyle/tests/fixtures/{clean,violations,broken}`. Golden output uses `insta`.
