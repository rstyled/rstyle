---
name: rstyle
description: Run `cargo rstyle` to find Rust functions that are too long, take too many params, mutate too much, or have high side-effect density, then fix them in independent groups; also `cargo rstyle patterns` finds parallel code that should be one trait, generic fn or helper. Use when asked to check/enforce Rust style, reduce function complexity, clean up rstyle violations, or find abstractions to extract.
---

# rstyle

Deterministic Rust style analysis on rustc HIR. Same input gives byte-identical output.

## Run

```sh
cargo rstyle --format json            # machine-readable; prefer this
cargo rstyle -p <pkg> --all-targets   # one package, include tests/examples/benches
```

Other flags: `--config PATH` (default `<workspace root>/rstyle.toml`), `--manifest-path PATH`.

Exit codes: `0` clean, `1` violations found, `2` tool/config/compile error or
incomplete analysis. Tool errors appear on stderr; unavailable measurements stay
in the report.

## Read the output

JSON top level: `diagnostics`, `summary`, `fix_groups`, `functions`.

- `diagnostics[]`: `path`, `line`, `column`, `rule_id`, `message`, `actual`, `limit`, `details`.
- Rule ids: `fn_length`, `fn_params`, `fn_spacing`, `cyclomatic_complexity`, `cognitive_complexity`, `mutation_per_target`, `mutated_targets`, `side_effect_density`, `invalid_suppression`.
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
compiler lowering and opaque macro expansions do not add hidden branches. See
`docs/complexity.md` for policy.

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
- `fn_params`: introduce a params struct or move state into a type. Do not hide arguments in tuples.
- `mutation_per_target` / `mutated_targets`: return new values instead of writing through `&mut`; split functions by the thing they mutate.
- `side_effect_density`: separate pure computation from IO/network/time/random/unsafe. Keep effects in a thin outer function.

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

JSON: `signature_groups[]` (methods of different types with one normalized signature; a blocking key, noisy), `candidates[]` (ranked), `suppressed[]` (already behind one trait).

Candidate fields: `kind`, `score_milli` and `breakdown` (`support`, `lift_milli`, `holes`, `coverage_milli`), `observation` (what differs: `T0 = Dog | Cat; M0 = Dog::bark | Cat::meow`; `F` function, `L` literal holes too), `inference`, `possible_refactor` (sketch, names are placeholders), `sites[]` (callers, `path`/`line`), `definitions[]` (functions filling the method holes), `counter_evidence[]` (read it before acting).

Kinds:
- `trait_method`: one method hole joined to a signature shared by several types. Extract a trait.
- `capability_set`: two or more method holes, the types are used through a set of methods. Extract a trait with those methods (check every type really has them: `definitions[]` should list each type).
- `enum_dispatch`: a `match` whose arms call analogous methods. Extract a trait and dispatch through it.
- `generic_fn`: same body over several workspace types. One generic fn.
- `parameterize`: same body differing only in callees, literals or foreign types. One helper taking those as parameters.

Score (natural-log units, a ranking and not a probability): `(ln(support) + lift) * coverage`, divided by `holes / 3` when holes > 3, halved on effect mismatch. `lift` is how rare the shared signature is (`ln(methods in workspace / methods sharing it)`, about 0 for getters). `generic_fn` is halved and `parameterize` divided by 3, with lift 0, so compare scores within a kind: those two stay below about 1 even when good. For the trait kinds, a score under about 0.7, or counter-evidence "signature is common in this codebase", is noise. `capability_set` loses half its score to 6 holes (types times methods) without being weak; read it anyway.

Workflow: take the top candidates, open each `sites` entry, confirm the parallel bodies really mean the same thing, then extract the trait or generic fn. A candidate is a hint, not a verdict: low `lift_milli` (common signature) or effect-mismatch counter-evidence means be skeptical, and support 2 is thin evidence. Opt out with `// rstyle-allow(abstraction): reason` above the function. Oracle for evaluation: `--hide-implements Trait` pretends the trait is not implemented. Output is deterministic; facts are cached in `target/rstyle/facts-sem` (separate from style facts).

Reviewed and rejected candidates: every candidate has an `id`. Only when the user agrees a candidate is not worth merging, run `cargo rstyle ignore <id> --reason "why"` (add `--allow-growth` to keep it hidden when more functions join). That writes `rstyle-ignore.toml`, which `patterns` then applies as a filter (`ignored_count` and `ignored[]` in JSON, `--show-ignored` in text, `--no-ignore` to see everything; `--fail-on-candidates` exits 1 while any candidate is left, the CI gate). Never dismiss to quiet the output unprompted. `cargo rstyle ignore --status` shows where each entry's functions are now and why an entry went stale; `--prune [--dry-run]` removes entries that match nothing. Delete the file to re-review everything. Pass the same clustering flags to `ignore` as to `patterns`.

Defaults (WL similarity 600, alignment coverage 600, 3 holes max for `parameterize`) are the evaluated setting. Opt-in experiments, not recommended unless asked: `--normalize all|ctor,pred,case,exit,counter,cmp,try` (canonicalize graphs; on CodeNet it removes about one unmatched node per pair and did not change what gets flagged, `case` does nearly all the work), `--pair-coverage N`, `--max-holes N`, `--single-call-guard`, `--threshold N`, `--min-coverage N` (more recall of strangers' spellings at the price of more false positives; `--pair-coverage 650 --max-holes 4` on CodeNet: about 20 extra cross-problem flags per 350k pairs).

## Requirements

Needs the pinned nightly from the project's `rust-toolchain.toml` (installed by rustup automatically) and `rstyle-driver` installed next to the `cargo-rstyle` binary.


## Opt-in clones source rules

`cargo rstyle clones --extraction-profile hir-data-v3 --source-rules` evaluates
captured length and parameter evidence independently of strict graph admission.
Use `--analysis-config analysis.toml` for those two policies and
`--fail-on-findings` for exit one on visible findings; incomplete scope exits two.
Existing `rstyle-allow(fn_length, fn_params): reason` comments use captured source
and apply only to those named rules. Invalid annotations remain findings. JSON
retains raw `analysis.findings` plus `visible_findings`, `suppressed_findings` and
per-function `source_suppression`. Saved-run replay preserves original comments.
Add `[fn_spacing]` to the analysis config to request spacing from checked original
direct statements. Its default threshold is five; zero disables it. Unsupported
statement transformations remain incomplete, independently of length/arity.
Named `fn_spacing` suppression applies when requested.
Signature-only inspection parses a bounded borrowed header, independently of body
size or support. Statement and guard parsing use the shared lexical syntax budget;
exhaustion keeps available length/arity findings but reports incomplete execution
and exit two. Saved replay uses the same bounds without requiring source files.
Source policy also accepts top-level workspace-relative `exclude` globs (default
`["target/**"]`; `[]` disables). Excluded owners remain explicit complete skips
in `source_exclusion` and scope counters; other consumers still evaluate them.
Add `--changed-files PATH` for explicit changed-file source diagnostics and finding
exits. JSON input is `{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}`;
paths are normalized workspace-relative Rust paths. All assessments and totals
remain available, with selected counts in `analysis.changed_scope`. Unobserved
paths and unavailable evidence remain incomplete; whole-scope incompleteness still
exits two. Other consumers and planning populations remain unchanged. Selection
can be changed during source/tool-free replay; no Git or live source lookup runs.
See `docs/clones-source-rules.md` for scope and association limitations. Keep both
legacy style and patterns gates until their capability retirement gates pass.


## Opt-in numeric source-rule reviews

Add `--review-file reviews.json` to source rules for explicit parameter reviews.
Only dismiss a completed visible selected `fn_params` finding after the user
chooses to retain that boundary. Use an exact `--replay-run`,
`--dismiss-parameter FUNCTION_KEY` and a nonempty `--review-reason`; do not dismiss
findings merely to quiet the output. Parameter dismissal writes schema 3 while
preserving existing helper and source entries. Lookup/status never write or
upgrade stores. Body edits, binder renaming and line moves preserve review;
parameter count, receiver presence, numeric policy or detector-version changes require review, and changed
build contexts cannot inherit it. Identity initially covers uniquely indexed
original free functions in captured root source and inline modules; external
module files, ambiguity, missing ownership and exhausted bounds stay unavailable.
Only selected hidden parameter findings leave the text/finding gate. Raw analysis
counts, assessments and advisory parameter plans remain unfiltered; other rules
and incomplete-scope exits retain their meaning. Explicit saved-run
`--review-status` reports retained parameter entries. Explicit `--review-prune`
can remove only completed negatives under the retained numeric policy and current
detector versions; raised limits, missing owners and unavailable evidence never
authorize pruning. See `docs/clones-finding-review.md` for commands and bounds.


Length choices use the same review workflow with `--dismiss-length FUNCTION_KEY`.
They require an explicit saved run and reason, write schema-4 `length_entries`,
and appear in `length_review`/retained `length` status. Identity retains checked
owner/context, detector versions, `max_lines` and completed code-line count.
Same-count edits and blank/comment insertions preserve this numeric choice;
changed count or policy requires review. Dismiss only after the user chooses to
retain that measured boundary. Parameter and length choices remain independent;
other findings, raw assessments/plans and incomplete exits retain their meaning.
Only same-policy/current-version completed negatives can be explicitly pruned.

Spacing choices use `--dismiss-spacing FUNCTION_KEY` from an explicit saved run
with an explicit `[fn_spacing]` policy and reason. Schema-5 `spacing_entries`
retain `min_statements` and the completed direct-statement count; JSON reports
`spacing_review` and retained status family `spacing`. Same-count positive findings
preserve review. A same-policy blank-line negative can be explicitly pruned;
disabling or changing the threshold stays stale, and omitted spacing is
not-requested. Three source families share owner resolution but keep independent
hiding and counts. Never dismiss merely to silence findings.


## Opt-in caller-aware parameter plans

Use `cargo rstyle clones --extraction-profile hir-data-v3 --locked --source-rules
--parameter-plans --analysis-config PATH` for advisory signature and witnessed
caller regions from visible parameter-count findings. `parameter_planning`
reports same-unit caller census completeness separately from exact witnesses;
opaque or unexamined work and budgets remain incomplete and exit two. Named
suppression and path exclusion apply to findings, while excluded callers are
still scanned. Argument formal ordinals describe source roots. Forwarding bundles
retain repeated exact call witnesses for distinct immutable owned-u32 formal
reads, with separate support coverage and budget outcomes. Mutable/borrowed
formals, aliases and transformed arguments do not establish unchanged forwarding.
Bundles do not prove pass-through or removable parameters; their search does not
enumerate every subset or prove maximality. Each unit's `parameter_uses` separately
records owned-u32 formal occurrences. A complete immutable formal used only as
unchanged arguments into selected signature intents yields an advisory
`pass_through_candidate`; unused, other-use and unavailable outcomes stay distinct.
Opaque syntax and unresolved targets cannot prove forwarding. Use retention is
bounded across units; this does not establish safe removal or propagation.
`coordination` checks known physical edit overlaps;
external callers, replacement design, whole-plan independence and stack order
remain unknown. Strict saved-run replay supports this consumer without original
source/tools. Legacy parameter grouping remains available for its broader scope.


## Opt-in guard planning and shared coordination

Use `clones --guard-clauses --guard-plans` to retain each guard finding's required
original terminal-if region. Request helper or parameter plans alongside it to
see `edit_coordination`: typed family/local references, exact overlap witnesses,
known-region disjointness, conflicting snapshots and explicit coverage/budgets.
These source regions share captured snapshots; strict replay needs no live files.
Unknown caller populations remain incomplete even when an overlap is witnessed.
No report proves whole-plan independence, rewrite correctness or a stack order.
See `docs/clones-edit-planning.md` for limits and intent semantics.

## Bounded scalar helper mutation effects

Direct `clones --mutation-rules` counts static scalar assignment sites through
ordinary unexpanded `if`/`else` branches, preserving original lexical roots.
Mutually exclusive arm sites each count once; runtime execution counts remain
unknown. Unlabelled loops, primitive bool while conditions and half-open u32 for
ranges also count original sites once, including source after plain break/continue.
Wildcard and immutable scalar range binders retain exact body scopes. Pattern
conditions, labels, value breaks, arbitrary iterators and opaque work remain
incomplete. This direct inventory does not certify loop/conditional helper calls
or parameter uses.
See `docs/clones-mutations.md` for policy, provenance and replay semantics.

Mutation schema `rstyle.mutations/2` keeps raw evidence/rules separate from
`policy` decisions. Use `visible_findings` for finding exits; named captured
`rstyle-allow(mutation_per_target, mutated_targets): reason` comments hide only
those rules. Suppression does not repair incomplete evidence. Mutation-config
`exclude` uses original workspace-relative globs (default `target/**`, `[]` opts
out); excluded owners are explicit policy skips with raw evidence retained.
Inspect raw and policy coverage/counters separately. Replay reevaluates config
against captured paths and comments without reading changed live files.

`clones --mutation-effects` uses schema `rstyle.mutation-effects/2` for original
scalar writes through at most two same-unit call edges (`caller → wrapper → leaf`).
Inspect `summary_site_index`, original write owner/site, `call_path` and projected
`mapped_root` separately from direct assignment counts. Repeated paths remain
distinct while completed summary proofs are reused. Third edges, recursion,
aliases, unsupported signatures and opaque calls stay incomplete; this is no
purity, density, runtime-frequency or rewrite-safety claim. Existing owner/call/site
budgets bound summary/path work. Replay recomputes from immutable original facts,
without compiler tools or live source. See `docs/clones-mutation-effects.md`.
