# Style checks

`cargo rstyle` checks a Rust workspace against the rules below and reports findings, fix groups and
complexity scores. It reads `rstyle.toml` from the workspace root, or the file named by `--config`,
and exits 1 if it finds anything.

```sh
cargo rstyle
cargo rstyle --format json
```

The text report ranks functions by cognitive complexity, then cyclomatic. Functions without findings
are left out. JSON includes source evidence for each score, all findings, and counts for every
analyzed function.

## Rules

| Rule id | Flags |
| --- | --- |
| `fn_length` | Functions with too many code lines. Blank and comment lines don't count. |
| `fn_params` | Functions with too many parameters. `self` doesn't count unless `fn_params.count_self` is set. |
| `fn_spacing` | Bodies with at least `fn_spacing.min_statements` top-level statements and no blank line anywhere. |
| `cyclomatic_complexity` | Functions whose cyclomatic score, which counts source decisions, is above the limit. |
| `cognitive_complexity` | Functions whose cognitive score, which also charges nesting, is above the limit. |
| `mutation_per_target` | Too many writes or `&mut` borrows of one root: a local, parameter or `self`. |
| `mutated_targets` | Too many distinct roots mutated in one function. |
| `side_effect_density` | Too many effects for the number of statements. See below. |
| `invalid_suppression` | An `rstyle-allow` comment with no reason or an unknown rule id. |

A missing complexity score is reported as unavailable, never as zero, and makes the run exit 2.
Closures count toward the function that owns them. Compiler lowering and a macro's own code add no
branches, while code you pass to a macro or that a macro wraps counts like any other.

### Side-effect density

`density_milli = 1000 * sum(effect weights) / statements`, flagged above
`side_effects.max_density_milli`.

Effects are mutations that escape the function through `&mut`, IO, network, globals, `unsafe`, time,
randomness, panics, and calls through `dyn`, function pointers or generic callables. Effects travel up
the call graph for `side_effects.transitive_depth` hops, so a function inherits them from what it
calls.

Functions under `side_effects.min_statements` statements or `side_effects.min_lines` code lines are
exempt.

Findings from this rule block like any other. To dismiss one you've reviewed, use an
[`rstyle-allow(side_effect_density)` comment](#suppressing-a-finding) with a reason, or a
[review file](#reviewing-findings-without-changing-source) if you'd rather not change the source.

## Configuration

Put an `rstyle.toml` in the project root. Every key is optional, and an unknown key is an error.
Changing the config never triggers a recompile. The [`rstyle.toml` in this repository](../rstyle.toml)
is a commented example, though it raises the complexity limits above their defaults.

| Key | Default | Meaning |
| --- | --- | --- |
| `exclude` | `["target/**"]` | Globs of original source to skip entirely. A callee they remove keeps its callers' effects incomplete. A build script's output is generated code, which they don't select. |
| `fn_length.max_lines` | 50 | Code lines per function |
| `fn_params.max_params` | 4 | Parameters per function |
| `fn_params.count_self` | `false` | Count `self` as a parameter |
| `fn_spacing.min_statements` | 5 | Bodies with this many top-level statements need a blank line somewhere; 0 disables the rule |
| `complexity.max_cyclomatic` | 12 | Report cyclomatic scores above this |
| `complexity.max_cognitive` | 12 | Report cognitive scores above this |
| `mutation_budget.max_mutations_per_target` | 3 | Writes or `&mut` borrows of one root |
| `mutation_budget.max_mutated_targets` | 3 | Distinct roots mutated in one function |
| `mutation_budget.granularity` | `"root"` | `"field"` keys targets as `self.a` and `self.b` instead of `self` |
| `side_effects.max_density_milli` | 500 | Density limit |
| `side_effects.min_statements` | 3 | Functions with fewer statements skip the density rule |
| `side_effects.min_lines` | 10 | Functions with fewer code lines skip the density rule |
| `side_effects.transitive_depth` | 3 | Call hops an effect travels up; 0 means direct effects only |
| `side_effects.weights.*` | see below | Weight of each effect kind |
| `side_effects.paths.*` | see below | Callee path prefixes that count as each effect kind |
| `grouping.caller_rules` | `["fn_params"]` | Rules whose fix also edits the function's callers |
| `grouping.caller_hops` | 1 | How many caller hops those fixes reach |

The default weights are `mutation = 1`, `io = 3`, `network = 3`, `global = 2`, `unsafe = 4`, `time = 1`,
`random = 1`, `panic = 0` and `unknown = 1`.

`[side_effects.paths]` classifies callees by path prefix for `io`, `network`, `time`, `random` and
`panic`. The longest match wins, and setting a list replaces its default:

```toml
[side_effects.paths]
network = ["std::net::", "reqwest::"]
```

## Suppressing a finding

Put a comment on the line directly above the function, with a reason:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
fn shim(a: u32, b: u32, c: u32, d: u32, e: u32) { /* ... */ }
```

`abstraction` is also a valid rule id, and keeps a function out of [pattern mining](patterns.md).

## Fix groups

`cargo rstyle` also tells you which findings can be fixed independently:

```text
fix groups: 693 (92 stacked, 601 independent)
  group 4 (2 functions; land together or stack in this order):
    src/a.rs:10 a::callee [fn_params]
    src/b.rs:42 b::caller [fn_length]
```

- A group is a set of findings whose fixes overlap. Put them in one PR, or stack the PRs in the listed
  order, callees first. Different groups never conflict, so they can go in parallel.
- The overlap model: a fix edits the function itself and, for `grouping.caller_rules`, its callers, up
  to `grouping.caller_hops`. The call graph resolves trait-method calls to every impl.
- `--format json` has the full `fix_groups` array. Groups carry soft `hints` (`shared_type`,
  `shared_caller`), which name other groups touching the same mutated type or caller. They aren't
  reasons to stack.
- Regroup after each merge, since fixes change the graph.

The footprint is deliberately narrow. On rust-lang/cargo, counting density callers and mutated types
turned 519 of 908 violating functions into one group, while `fn_params` alone gives 693 groups, the
largest with 89 functions.

### Parameter refactor plans

Groups with a `fn_params` member carry a `plan`, in JSON at `fix_groups[].plan` and under the group in
text output:

```text
  group 172 (...)
      1. signature Dependency::from_toml (src/.../dependency.rs:224; 5 incoming, 0 outgoing sites)
      2. call sites find_workspace_dep (src/.../mod.rs:1372; 3 incoming, 1 outgoing sites; pass-through: unstable_features)
      bundle (1 sites): key: &str, unstable_features: &Features
```

- **Steps** list the functions to re-sign, callee first, then the callers whose call sites must
  follow. `incoming` counts call sites into the function, and `outgoing` counts its call sites into
  re-signed functions.
- **`pass-through`** names parameters whose every mention is a forward into a re-signed function. The
  caller doesn't need them, so fix it in the callee or an upper layer and drop them from the caller.
- **`bundle`** names parameters forwarded together (the same callee `name: type`, two or more) at N
  call sites, which makes them a candidate for a params struct. Only closed sets are reported: a subset
  with the same support as a superset is dropped.

## Using it in CI

### Checking only what changed

```sh
cargo rstyle --changed-files changes.json --format json
```

```json
{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}
```

Paths are relative to the workspace root. This selects which findings count, while metrics and fix
groups keep the full captured scope. Invalid suppressions stay visible, and a selected path that wasn't
observed or a missing required input still exits 2.

### Baselines

```sh
cargo rstyle --write-baseline baseline.json          # export every current finding to a new file
cargo rstyle --baseline baseline.json --fail-on-new  # fail only on what's new
```

Exporting needs a complete analysis, and otherwise exits 2 without writing a file. It keeps the normal
exit status, so it exits 1 when there are findings.

Comparing exits 1 for new or changed selected findings, and for policy or build-context drift. Raw
findings stay visible, and incomplete analysis still exits 2. Comparing can be combined with
`--changed-files`; exporting can't.

### Reviewing findings without changing source

A review file dismisses individual findings from a saved run:

```sh
cargo rstyle --format json
cargo rstyle --replay-run RUN --review-file rstyle-reviews.json \
  --dismiss-compiler-diagnostic UNIT DIAGNOSTIC_INDEX --review-reason "Reviewed IO boundary"
cargo rstyle --review-file rstyle-reviews.json --format json
```

Take `RUN` from the report's `run_directory`, and `UNIT` and `DIAGNOSTIC_INDEX` from the parallel
`diagnostic_units` and `diagnostic_indices` arrays. The text report shows them too, as
`[compiler unit 0]` and `[diagnostic 2]`.

A review hides only that exact finding, so a changed value, policy or build context can make it visible
again. Raw per-unit findings stay available. `--review-status` inspects the file without writing, and
the file is independent of the [pattern ignore ledger](patterns.md#the-ignore-ledger).

## Saved runs

Every `cargo rstyle` and `cargo rstyle patterns` run saves its documents under `target/rstyle/runs/`,
and the report names the directory. `--replay-run PATH` evaluates a saved run again without source
files or compiler tools, and works with `--changed-files`, baselines and reviews. A run saved by one
route answers the others, so replaying a style run with `patterns --complexity` needs no new capture.

Those commands and `rstyle rust` share one capture and reuse private builds that Cargo reports as
fresh, so switching between them recompiles nothing that hasn't changed.

A new capture removes all but the workspace's 5 most recently used runs, since one run can hold
gigabytes. It never removes a run another command is using, or the run the next build reuses compiled
crates from. Set `RSTYLE_KEEP_RUNS` to another count, or to `all` to keep every run. To keep a run
you're working through, set it before capturing again.

`rstyle rust` and `rstyle go` save their finished runs elsewhere. See
[saved runs](frontends.md#saved-runs) in the runner guide.

## Reading the report

Text findings and complexity rows carry compiler unit labels, followed by a table of the package,
target, test mode and source of each unit. Library and test builds of the same source stay
distinguishable, including on replay.

Root JSON holds `diagnostics`, `summary`, `fix_groups` and `functions`, with explicit scope and
planning completeness. The `summary` line reads `summary unavailable`, with `summary_complete: false`
in JSON, when no compiler evaluation completed. That exits 2.
