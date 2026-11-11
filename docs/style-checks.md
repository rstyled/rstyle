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
| `cyclomatic_complexity` | Functions whose cyclomatic score, which counts source decisions, is above the limit. |
| `cognitive_complexity` | Functions whose cognitive score, which also charges nesting, is above the limit. |
| `mutation_per_target` | Too many writes or `&mut` borrows of one root: a local, parameter or `self`. |
| `mutated_targets` | Too many distinct roots mutated in one function. |
| `side_effect_density` | Too many effects for the number of statements. See below. |
| `invalid_suppression` | An `rstyle-allow` comment with no reason or an unknown rule id. |
| `fn_spacing`, `guard_clause`, `primitive_obsession`, `value_object` | Off by default. See [strict style](#strict-style). |

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
| `exclude` | `["target/**"]` | Paths and globs of original source to skip entirely, relative to the root reports show paths from. A directory takes everything under it. A callee they remove keeps its callers' effects incomplete. A build script's output is generated code, which they don't select. `--exclude` on `rstyle go`, `cxx`, `py` and `rust` adds to it for one run, and [the runner guide](frontends.md#leaving-files-out) has the syntax. |
| `fn_length.max_lines` | 50 | Code lines per function |
| `fn_params.max_params` | 4 | Parameters per function |
| `fn_params.count_self` | `false` | Count `self` as a parameter |
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
| `ignored_result.min_used_percent` | 90 | How much of a callee's calls must use its result, from 1 to 100. See [`ignored_result`](defects.md#ignored_result) |
| `ignored_result.min_calls` | 5 | Fewest calls of a callee that say anything. At 90%, one drop already takes 10 |
| `deviant_argument.min_methods` | 4 | Fewest sibling methods, counting the one that differs, that make the call; at least 3. See [`deviant_argument`](defects.md#deviant_argument) |
| `deviant_argument.min_shared_callees_percent` | 50 | How much of their callees two methods of a type share to be siblings, from 1 to 100 |
| `grouping.caller_rules` | `["fn_params"]` | Rules whose fix also edits the function's callers |
| `grouping.caller_hops` | 1 | How many caller hops those fixes reach |
| `strict_style.*` | off | See [strict style](#strict-style) |

The default weights are `mutation = 1`, `io = 3`, `network = 3`, `global = 2`, `unsafe = 4`, `time = 1`,
`random = 1`, `panic = 0` and `unknown = 1`.

`[side_effects.paths]` classifies callees by path prefix for `io`, `network`, `time`, `random` and
`panic`. The longest match wins, and setting a list replaces its default:

```toml
[side_effects.paths]
network = ["std::net::", "reqwest::"]
```

A frontend can also ship a table of its language's library calls, keyed by operation.
[Go's](../crates/rstyle-core/src/cpg/contracts/effects/go.toml) knows that `os.WriteFile` is
`io` and that `strings.ToUpper` has no effect, and treats a library call it doesn't list as
`unknown`. [Rust's](../crates/rstyle-core/src/cpg/contracts/effects/rust.toml) only lists
standard methods and functions that have no effect whatever type they're called with, such as
`slice::iter`, `std::mem::swap`, `Vec::push`, `Option::unwrap` and `format!`, since allocating,
panicking on a bad argument and formatting into a `String` count as no effect, and leaves every
other library call unclassified, so density doesn't count it. A call is classified by the
implementation it selects, when the frontend names one, and by the function it names otherwise: `?`
on a `Result`, `String`'s `clone` and `to_string` are pure, but `Option`'s `clone`, which runs the
element's, isn't listed. [Python's](../crates/rstyle-core/src/cpg/contracts/effects/py.toml) lists
the standard library's effects, such as `open`, `subprocess.run`, `os.getenv` and `time.monotonic`,
and leaves third-party calls unclassified, as Rust's does. Python runs a value's own methods almost
everywhere, so its `none` list only holds what runs nothing it's handed, such as `id`, `typing.cast`
and `list.append`. Builtins such as `len`, `isinstance` and `str`'s methods are in
`none_without_callback`: pure where the frontend says the call calls nothing of the workspace's back,
and unclassified elsewhere, since `len` of a class runs its `__len__`. `[side_effects.paths]` wins for
any callee it matches, so you can still name a third-party package there.

## Suppressing a finding

Put a comment on the line directly above the function, with a reason:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
fn shim(a: u32, b: u32, c: u32, d: u32, e: u32) { /* ... */ }
```

`abstraction` is also a valid rule id, and keeps a function out of [pattern mining](patterns.md). So
are [`dead_store`, `ignored_result` and `taint_flow`](defects.md).

## Strict style

These rules are opinions about how code could be shaped, not limits it has passed, and plenty of good code disagrees with
them. So each is off until its table under `[strict_style]` in `rstyle.toml` turns it on, and says how much it takes:

```toml
[strict_style.guard_clause]
enabled = true
min_nested_statements = 3
```

A rule that's on behaves like any other here: it reports through `check`, fails the run, shows up in `check.sarif` and
baselines, and `rstyle-allow` silences it for a function. Every language gets them from the same facts. A rule that's off
is left out of what a baseline compares, whatever its other settings hold.

### `fn_spacing`

A body with many top-level statements and no blank line anywhere, so nothing marks where it moves from setting up, to
working, to giving a result. It counts statements, not lines, so a long initializer is still one thing to read, and a
blank line in the signature counts too.

| Setting | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | |
| `min_statements` | 5 | Bodies with this many top-level statements, the tail expression included, need a blank line somewhere |

Unlike the other strict style rules, an [override](../README.md#per-path-overrides) can set it for some paths, as
`strict_style.fn_spacing = { enabled = false }`.

### `guard_clause`

An `if` whose `else` ends by returning, around statements that don't:

```go
if ok {
    a := first()
    second(a)
} else {
    return errBad
}
```

Returning early lets the main path stop being nested, and in Rust, `let ... else` often says it best. An `else if` is left
alone, since returning early there means restructuring the whole chain.

| Setting | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | |
| `min_nested_statements` | 2 | Fewer statements under the `if` than this are no main path to bring out |

### `primitive_obsession`

A parameter whose name says what it means, held as plain text or an integer in several functions of a run:

```rust
fn charge(user_id: &str, amount: u64) {}
fn refund(user_id: &str, amount: u64) {}
```

A named type would say what each holds, and stop two of them being swapped. It reads a name's whole words, so `userEmail`
and `user_email` hold `email`, and `valid` doesn't hold `id`.

Each function reports each meaning once, and a type someone named is never reported. It looks through references, so
`&str`, `String` and `const std::string &` are all text. Most results on real code are `name` and `id`, so expect to take them
out of `meanings`, or suppress or exclude some.

| Setting | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | |
| `min_functions` | 2 | Fewer functions than this holding a meaning say nothing about a codebase |
| `meanings` | see below | Words of a name that mean something a type could say. Each is one lowercase word, and anything else is an error, since it could never match |

The default words are `email`, `phone`, `url`, `uri`, `uuid`, `guid`, `id`, `name`, `address`, `street`, `city`, `state`, `zip`,
`postal`, `country`, `currency`, `price`, `amount`, `quantity`, `ssn`, `passport`, `license`, `username`, `password`, `token`,
`apikey`, `secret`, `creditcard`, `iban`, `isbn`, `sku` and `serial`.

### `value_object`

Plain values under the same names, in several functions of a run:

```go
func dial(host string, port int) {}
func probe(host string, port int, timeout int) {}
func listen(port int, host string) {}
```

A small type could hold them. Only the largest such group is reported, and a function can be in several. Names that say
nothing about a value, such as `x`, `tmp` or `arg`, can't make a group on their own, and a name starting with `_` isn't part
of one. It only reads parameters, so it can't say the values really belong together: the callers decide that. Signatures
matching by accident is the commonest reason to suppress one.

| Setting | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | |
| `min_parameters` | 2 | Fewer values than this have nothing to travel with |
| `min_functions` | 3 | Fewer functions than this could share a set of names by chance |
| `placeholders` | see below | Names that say nothing about a value. Each is one lowercase word, and anything else is an error |

The default placeholders are `foo`, `bar`, `baz`, `qux`, `tmp`, `temp`, `val`, `var`, `arg`, `args`, `param`, `params`, `obj`,
`item`, `elem`, `el`, `p1`, `p2`, `p3`, `a1`, `a2`, `a3`, `v1`, `v2`, `x1`, `x2`, `y1` and `y2`.

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

`rstyle rust`, `rstyle go`, `rstyle cxx` and `rstyle py` save their finished runs elsewhere. See
[saved runs](frontends.md#saved-runs) in the runner guide.

## Reading the report

Text findings and complexity rows carry compiler unit labels, followed by a table of the package,
target, test mode and source of each unit. Library and test builds of the same source stay
distinguishable, including on replay.

Root JSON holds `diagnostics`, `summary`, `fix_groups` and `functions`, with explicit scope and
planning completeness. The `summary` line reads `summary unavailable`, with `summary_complete: false`
in JSON, when no compiler evaluation completed. That exits 2.
