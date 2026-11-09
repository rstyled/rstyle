---
name: rstyle
description: Run `cargo rstyle` to find Rust functions that are too long, take too many params, mutate too much, or have high side-effect density, then fix them in independent groups; also `cargo rstyle patterns` finds parallel code that should be one trait, generic fn or helper. Use when asked to check/enforce Rust style, reduce function complexity, clean up rstyle violations, or find abstractions to extract.
---

Compiler-only release: engine selectors and flat facts caches have been removed.
Use the commands below without `--style-engine` or `--engine`. Historical comparison
artifacts retain their original metric versions; they are not current CLI contracts.

# rstyle

Deterministic findings from captured Rust compiler facts. Capture paths and run metadata may differ between runs.

## Run

```sh
cargo rstyle --format json            # machine-readable; prefer this
cargo rstyle -p <pkg> --all-targets   # one package, include tests/examples/benches
```

Other flags: `--config PATH` (default `<workspace root>/rstyle.toml`), `--manifest-path PATH`.

Compiler style supports changed-file finding selection:
`cargo rstyle --changed-files changes.json --format json`.
The manifest is `{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}`;
paths are relative to the workspace root. This selects diagnostics and their exit
status while metrics and fix groups retain the full captured scope. Invalid
suppressions remain visible. Unobserved selected paths and missing required
inputs remain incomplete (exit 2), even when no findings are selected. Add
`--replay-run PATH` to reevaluate an explicit saved run without compiler tools.
A new capture removes all but a workspace's 5 most recently used runs; set
`RSTYLE_KEEP_RUNS` to another count, or to `all`, to keep more. A function a build
script writes to `OUT_DIR` is generated code: it gets no findings, and its effects reach
its callers, wherever the target directory is.

Use `--write-baseline baseline.json` to export a complete
whole-scope inventory to a new file; ordinary findings still return 1 after export.
Use `--baseline baseline.json --fail-on-new` with the compiler engine for CI:
unchanged existing findings alone return 0; new/changed selected findings and
policy/metric/build-context drift return 1. Raw findings and plans remain visible.
Incomplete analysis still returns 2. Comparison supports changed-file selection;
export does not. Both support explicit `--replay-run PATH`, reusing the existing
diagnostic baseline format and comparison workflow.

Exit codes: `0` clean, `1` violations found, `2` tool/config/compile error or
incomplete analysis, including a run with no completed compiler evaluation
(`summary unavailable`, JSON `summary_complete: false`). Tool errors appear on
stderr; unavailable measurements stay in the report.

Density findings block by default. Existing `// rstyle-allow(side_effect_density):
reason` comments work. To avoid code comments, use the existing saved diagnostic
review file, one finding at a time:

```sh
cargo rstyle --replay-run RUN --review-file rstyle-reviews.json \
  --dismiss-compiler-diagnostic UNIT DIAGNOSTIC_INDEX --review-reason "Reviewed IO boundary"
cargo rstyle --review-file rstyle-reviews.json --format json
```

Take RUN from `run_directory` and UNIT/DIAGNOSTIC_INDEX from that root diagnostic's
parallel `diagnostic_units`/`diagnostic_indices` arrays or text labels. Lookup never
writes; `--review-status` inspects retained entries. Changed findings can reappear,
other findings remain blocking, and raw per-unit evaluations stay intact. Never
bulk-dismiss findings merely to make a gate pass.

## Read the output

JSON top level: `diagnostics`, `summary`, `fix_groups`, `functions`.

- `diagnostics[]`: `path`, `line`, `column`, `rule_id`, `message`, `actual`, `limit`, `details`.
- Rule ids: `fn_length`, `fn_params`, `fn_spacing`, `cyclomatic_complexity`, `cognitive_complexity`, `mutation_per_target`, `mutated_targets`, `side_effect_density`, `guard_clause`, `primitive_obsession`, `value_object`, `invalid_suppression`.
- `fix_groups[]`: violations whose fixes overlap. Different groups never conflict.

## Complexity in the same report

```sh
cargo rstyle
cargo rstyle --format json
```

CC counts source decisions; COG also charges nesting. Each function carries
`complexity` with both scores, `policy_version`, and per-increment source evidence.
JSON includes these under `functions` for functions with findings or missing
measurements. Text ranks those functions by COG, then CC, together with all
findings and fix groups. Clean functions are omitted from both formats.
The summary reports `complexity_measured` and `complexity_unavailable`.
Missing scores are unavailable, never zero; the command exits 2 on missing
measurements and retains other findings. Closures contribute to their owner;
compiler lowering and a macro's own code add no branches, while code you pass to a macro
or that a macro wraps counts like any other.

[complexity] settings `max_cyclomatic` and `max_cognitive` both default to 12.
Scores strictly above their limit produce `cyclomatic_complexity` or
`cognitive_complexity` findings. They use normal suppressions and fix groups.
Prefer flatter control flow and cohesive helpers; do not split code just to lower
a score. The same command returns 1 for findings or 2 for missing measurements.

## Fix workflow

1. Run `cargo rstyle --format json`.
2. Pick one fix group. Groups are independent: one PR/commit per group, or work them in parallel.
3. Inside a group, fix callees first (listed order), then callers.
4. For `fn_params`, use the group's `plan`:
   - `signature` steps are re-signed functions, callee first.
   - `call sites` steps are callers that must follow.
   - `pass-through` params are only forwarded; drop them from the caller.
   - `bundle` is a params-struct candidate (params forwarded together at 2+ sites).
5. Re-run. Fixes change the call graph, so regroup after each change.
6. Run the project's tests. Do not stop at a clean rstyle run.

## Fix hints per rule

- `fn_length`: extract cohesive blocks into named helpers. Do not just compress lines.
- `fn_spacing`: add a blank line where the body changes phase (setup, work, result).
- `guard_clause`, only when `[strict_style.guard_clause]` turns it on: return early, so the main path stops being nested.
- `primitive_obsession`, only when `[strict_style.primitive_obsession]` turns it on: give what the parameter holds a named type.
- `value_object`, only when `[strict_style.value_object]` turns it on: gather the values into a small type, if the callers really pass them together.
- `fn_params`: introduce a params struct or move state into a type. Do not hide arguments in tuples.
- `mutation_per_target` / `mutated_targets`: return new values instead of writing through `&mut`; split functions by the thing they mutate.
- `side_effect_density`: separate pure computation from IO/network/time/random/unsafe. Keep effects in a thin outer function.

## Comparing two runs

`rstyle risk --base RUN --head RUN [--format json]` says what changed between two saved runs of any language, with no compiler: each function that changed, what it newly does and calls, and what reaches it. `[risk]` in `rstyle.toml` rates it, by the base run's policy, and with no rules it's `unrated`. It always exits 0. See `docs/merge-risk.md`.

## Suppression (last resort)

Line directly above the function, reason required:

```rust
// rstyle-allow(fn_params, fn_length): FFI shim mirrors the C signature
```

A missing reason or an unknown rule id yields `invalid_suppression`. Never suppress to make the run pass without being asked; prefer refactoring. Do not edit `rstyle.toml` thresholds to hide violations unless asked.

## Find abstractions to extract (`patterns`)

```sh
cargo rstyle patterns --format json [--all-targets] [--min-score 1.0] [--top 20]
```

JSON: each `compiler_advisory.units[].patterns.evaluation` contains `signature_groups[]` (methods of different types with one normalized signature; a blocking key, noisy), `candidates[]` (ranked), `suppressed[]` (already behind one trait). Presentation indices select the visible findings from these raw proposals.

Candidate fields: `kind`, `score_milli` and `breakdown` (`support`, `lift_milli`, `holes`, `coverage_milli`), `observation` (what differs: `T0 = Dog | Cat; M0 = Dog::bark | Cat::meow`; `F` function, `L` literal holes too), `inference`, `possible_refactor` (sketch, names are placeholders), `sites[]` (callers, `path`/`line`), `definitions[]` (functions filling the method holes), `counter_evidence[]` (read it before acting).

Kinds:
- `trait_method`: one method hole joined to a signature shared by several types. Extract a trait.
- `capability_set`: two or more method holes, the types are used through a set of methods. Extract a trait with those methods (check every type really has them: `definitions[]` should list each type).
- `enum_dispatch`: a `match` whose arms call analogous methods. Extract a trait and dispatch through it.
- `generic_fn`: same body over several workspace types. One generic fn.
- `parameterize`: same body differing only in callees, literals or foreign types. One helper taking those as parameters.

Score (natural-log units, a ranking and not a probability): `(ln(support) + lift) * coverage`, divided by `holes / 3` when holes > 3, halved on effect mismatch. `lift` is how rare the shared signature is (`ln(methods in workspace / methods sharing it)`, about 0 for getters). `generic_fn` is halved and `parameterize` divided by 3, with lift 0, so compare scores within a kind: those two stay below about 1 even when good. For the trait kinds, a score under about 0.7, or counter-evidence "signature is common in this codebase", is noise. `capability_set` loses half its score to 6 holes (types times methods) without being weak; read it anyway.

Helper and generic-function candidates also show recorded-effect mismatch
counter-evidence, without additional score damping. This compares intrinsic
kinds and escaping mutations, excluding configured calls and propagation.
An empty class or no warning does not establish purity or shared semantics.

Cluster proposals describe aligned graph regions, with an unmatched-node warning
when template/member node coverage is partial. Preserve unmatched behavior when
considering a helper. Full coverage or no recorded substitutions establishes
neither an exact source clone nor semantic equivalence; aligned nodes need not
be one extractable source fragment. Ranking and candidate selection are unchanged.
Helper and generic candidates also warn when a retained graph contains an opaque
ordinary closure. Even full graph coverage does not compare closure bodies or
captures; inspect their contracts before extracting shared code.

Workflow: take the top candidates, open each `sites` entry, confirm the parallel bodies really mean the same thing, then extract the trait or generic fn. A candidate is a hint, not a verdict: low `lift_milli` (common signature) or effect-mismatch counter-evidence means be skeptical, and support 2 is thin evidence. Opt out with `// rstyle-allow(abstraction): reason` above the function. Oracle for evaluation: `--hide-implements Trait` pretends the trait is not implemented. Findings are deterministic; compiler captures are saved under `target/rstyle/runs/` and can be reevaluated with `--replay-run PATH`.

Reviewed and rejected candidates: every candidate has an `id`. Only when the user agrees a candidate is not worth merging, run `cargo rstyle ignore <id> --reason "why"` (add `--allow-growth` to keep it hidden when more functions join). That writes `rstyle-ignore.toml`, which `patterns` then applies as a filter (`ignored_count` and `ignored[]` in JSON, `--show-ignored` in text, `--no-ignore` to see everything; `--fail-on-candidates` exits 1 while any candidate is left, the CI gate). Never dismiss to quiet the output unprompted. `cargo rstyle ignore --status` shows where each entry's functions are now and why an entry went stale; `--prune [--dry-run]` removes entries that match nothing. Delete the file to re-review everything. Pass the same clustering flags to `ignore` as to `patterns`.

Defaults (WL similarity 600, alignment coverage 600, 3 holes max for `parameterize`) are the evaluated setting. Opt-in experiments, not recommended unless asked: `--normalize all|ctor,pred,case,exit,counter,cmp,try` (canonicalize graphs; on CodeNet it removes about one unmatched node per pair and did not change what gets flagged, `case` does nearly all the work), `--pair-coverage N`, `--max-holes N`, `--single-call-guard`, `--threshold N`, `--min-coverage N` (more recall of strangers' spellings at the price of more false positives; `--pair-coverage 650 --max-holes 4` on CodeNet: about 20 extra cross-problem flags per 350k pairs).

## Any language (`rstyle go`, `rstyle cxx`, `rstyle py`, `rstyle rust`)

`rstyle go`, `rstyle cxx`, `rstyle py` and `rstyle rust` find the frontend, run it and analyze what it produces. `--check`,
`--patterns` or `--filter check,patterns,bounds,flow,defects` select analyses; the default is everything.

`taint_flow`, under `defects`, follows input to where it's dangerous, across functions. `[taint]` in `rstyle.toml` names a project's own sources, sinks and sanitizers, and `ignored` drops what doesn't count. A result is a path to read, not proof of a bug.
Each run is saved whole in `.rstyle/runs/`, which keeps each language's newest 5 (`RSTYLE_KEEP_RUNS`
changes that), with `report.json.zst`, `report.sarif`, `check.sarif` and `patterns.sarif`. `--format json` prints plain JSON with `run`
naming that folder. `--from DIR` reads a saved run back. Exit 0 is clean, 1 is findings, and 2 is a
tool error. `RSTYLE_GO` points at a local `rstyle-go`, `RSTYLE_CXX` at a local `rstyle-cxx`, and `RSTYLE_PY` at a local `rstyle-py`.
Options for the frontend itself go after `--` and reach it as they are. For C and C++, rstyle-cxx finds the project's
`compile_commands.json` or has the project build one, and `--compilation-database` names one; without one it warns and
guesses flags, losing what it can't parse. Don't accept that: `rstyle cxx -- --require-database` stops instead and
says why, and `rstyle-cxx database-runbook` says how to work out the project's database. For Python, a file that doesn't parse ends the run, and `--exclude` names a
path or glob to leave out. An environment in the project is read and never run; `--python` names an interpreter to run instead.

## Other languages (`cpg`)

`cargo rstyle cpg check DOC.json [MORE.json ...] [--config rstyle.toml] [--format json]`
runs the same style rules over `compat.cpg` documents from a non-Rust frontend, and
`cargo rstyle cpg patterns DOC.json [MORE.json ...] [--complexity] [--fail-on-candidates]`
mines them for the same abstraction candidates and runtime bounds. A run is any number
of units: bundled unit documents, and split unit documents with their function
documents; name them together so effects propagate across units. Core derives
statement counts, complexity, calls, edges, effects, mutations, dependence graphs and
runtime-bound programs from the documents; no compiler runs and only an explicit
`--config` applies. Exit codes are as for the default command; invalid or mismatched
documents, or a function read twice, return 2.
`cargo rstyle cpg analyze DOC.json [MORE.json ...] [--filter check,patterns,bounds,flow,defects] [--sarif-dir DIR]`
runs them in one pass.

`cargo rstyle cpg emit --out DIR` writes such documents for a Rust workspace.

`cargo rstyle cpg flow DOC.json [MORE.json ...] [--function NAME] --format json` reports,
for each function, immediate dominators and post-dominators, loops, and a dependence
graph: `data` (a write that can reach a read), `control` (a branch and what runs because
of it; `termination` marks an edge that holds only because a loop might never finish) and
`execution` (the nearest operation that always runs first). Read `standing` before relying
on them: `known`, `approximated` with `reasons`, or `unknown` where the frontend emits no
flow. An `approximate` data edge is of a binding that can also change through a borrow or
capture. Closure bodies are opaque, and unwinding is not a transfer.

## Requirements

Needs the pinned nightly from the project's `rust-toolchain.toml` (installed by rustup automatically) and `rstyle-driver` installed next to the `cargo-rstyle` binary.


## Saved runs

Every capture saves each crate's `compat.cpg` documents and a manifest sealing them under `target/rstyle/runs/`. Pass `--replay-run PATH` to reevaluate an explicit saved run without source files or compiler tools. Configuration and review files stay runtime inputs (`--config` applies); the run keeps its original selection, so new package, manifest or all-targets flags are rejected. Style and patterns read the same documents, so either can replay a run the other captured.

A new capture keeps a workspace's 5 most recently used runs. Set `RSTYLE_KEEP_RUNS` to another count, or to `all`.

Exit codes are 0 clean, 1 findings, 2 incomplete or error. Missing owners, scores or graphs stay unavailable; they never become zero or clean.

## Selecting findings

`--changed-files PATH` selects findings for CI, from `{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}` with workspace-relative paths. Metrics and fix groups keep the full captured scope, invalid suppressions stay visible, and unobserved selected paths still exit 2.

## Reviews and baselines

`--review-file PATH` looks up saved reviews and never writes. Only after the user agrees a finding is acceptable, replay its run and pass `--dismiss-compiler-diagnostic UNIT DIAGNOSTIC_INDEX --review-reason REASON`, with the numbers from that report. A review hides only the exact finding: changed values, policy or build context make it visible again. `--review-status` inspects retained entries without writing.

`--write-baseline PATH` exports a complete inventory to a new file. `--baseline PATH --fail-on-new` then exits 1 only for new or changed selected findings, or for policy or build-context drift. Incomplete scope exits 2 and cannot export.

## Pattern reviews

Repository dismissals live in `rstyle-ignore.toml` (`--ignore-file PATH`, or `--no-ignore` to bypass it), managed with `cargo rstyle ignore`. `--show-ignored` lists what the ledger hides. Pruning needs complete inputs from a full-workspace `--all-targets` capture, including for `--dry-run`, since narrower captures cannot show that an entry is stale.

For a review scoped to one compilation context, replay the run and pass `--dismiss-compiler-pattern UNIT CANDIDATE_ID --review-reason REASON` with `--review-file PATH`. Changing a member's context reports `context_changed`.

`patterns` defaults to authored functions; `--compiler-pattern-scope captured` includes generated signatures. `--fail-on-candidates` exits 1 on displayed candidates. `--min-score N` and `--top N` filter what is displayed, per compilation population, and cannot turn missing inputs into a complete result. `--threshold`, `--min-coverage`, `--pair-coverage`, `--normalize`, `--single-call-guard`, `--max-holes` and `--hide-implements` tune mining, and also apply on replay. Check and test populations stay separate.

Built-in decisions appear in `patterns.evaluation.suppressed` with their sites and a reason: a fallback helper whose alignment holds only distinct, single-use callee substitutions is rejected, independently of ignore and review entries.

## Effects across crates

When the caller and dependency workspace packages are selected, effects follow unique captured Cargo dependencies, direct or through a crate that re-exports them, to known free functions. `resolved_dependency_callees` maps compiler IDs to report-local unit IDs.

A trait's default body in a dependency and the overrides of it in the caller's crate are all targets. A method a trait declares without a body, with no implementation in the caller's crate, is an `Unknown` effect on its caller, listed in `unknown_callees`. It leaves coverage complete, so a complete report can still hold these calls, and as coverage grows they become known. Missing, generated, excluded and ambiguous targets remain unresolved.

Zero effect density is not a purity proof, and empty effect kinds do not mean pure.
