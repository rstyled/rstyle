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
Add `--select-guards --guard-clauses --changed-files PATH` to select guard advice
for text and finding exits. `analysis.guard_changed_scope` retains selected guard
counts and availability; global assessments, completeness and guard plans remain
intact. Multiple closure opportunities count once per owner, and incomplete global
guard coverage still exits two. Without this flag guard presentation stays global.
Add `--review-file reviews.json` for exact guard reviews. Dismiss a completed
opportunity with `--replay-run RUN --dismiss-guard FUNCTION_KEY OPPORTUNITY_INDEX
--review-reason TEXT`; indices list the function-tail opportunity before closures.
Reviews hide individual guard text/finding exits, while raw advice and plans stay
available. Exact guard and enclosing closure text define staleness; closure ordinals
can conservatively stale after insertion/reordering. Absent advice remains unavailable
for whole-store status and cannot authorize pruning. Global incompleteness stays exit two.

Add `--changed-files PATH` for explicit changed-file source diagnostics and finding
exits. JSON input is `{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}`;
paths are normalized workspace-relative Rust paths. All assessments and totals
remain available, with selected counts in `analysis.changed_scope`. Unobserved
paths and unavailable evidence remain incomplete; whole-scope incompleteness still
exits two. Density can also use the manifest, independently of source rules, with selected
counts in `mutation_effects.changed_scope` after numeric reviews. Other consumers
and planning populations retain their existing behavior. Selection
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
parameter count, receiver presence, numeric policy or detector-version changes
require review, and changed build contexts cannot inherit it. Identity covers
uniquely indexed original declarations in captured root source, inline modules and
independently checked
ordinary external modules. External methods support unique same-module
struct/enum owners with no type/const parameters and inherent/nongeneric local-trait
impls, with exact cfg/source/header checks. Plain lifetime-only owner/impl
parameters are allowed with matching lifetime argument counts; locator names omit
lifetimes and canonical method collisions refuse identity. Lifetime bounds, where
clauses, generic traits, type/const-generic, aliased/imported/qualified owners or traits, default trait methods,
remapped/generated modules, ambiguity, missing ownership and exhausted bounds
stay unavailable.
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
original terminal-if region. Synchronous block closure opportunities identify
their closure return scope with `closure_span`; never move that return into the
enclosing function. The enclosing assessment stays incomplete. Request helper or parameter plans alongside it to
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

Saved numeric mutation choices use review-store schema 6 and JSON
`mutation_review` (`rstyle.mutation-rule-review/1`). With an explicit saved run,
review file and reason, use `--dismiss-mutation-per-target FUNCTION_KEY` or
`--dismiss-mutated-targets FUNCTION_KEY`. Only completed selected visible numeric
findings with unique original owners qualify. Choices retain exact rule/threshold,
build context and the complete sorted count distribution; changing those makes
them stale. Numeric equality does not approve semantics. Reviews hide text and
finding exits, counted by `mutation_analysis.review_hidden_findings`, while raw
evidence/rules and completeness remain unchanged. Status/prune distinguish
omitted, missing, stale and unavailable owners; only an exact-context completed
selected negative can authorize pruning. Older review schemas remain readable.

`clones --mutation-effects` includes an independent original-source nested statement
inventory (`rstyle.original-source-statements/1`); unavailable counts never
authorize a density ratio. It uses schema `rstyle.mutation-effects/4` for original
scalar writes through at most two same-unit call edges (`caller → wrapper → leaf`).
Checked ordinary branches and primitive loops retain each original call/write
site once, including mutually exclusive arms and zero-iteration bodies. Lexical
shadowing keeps unrelated locals distinct from mutable formals. This explicit
effect observation leaves ordinary call-roster/formal-use certification unchanged;
unsupported control shapes, aliases and opaque work remain incomplete. No runtime
frequency, purity or termination conclusion follows from complete static sites.
Inspect `summary_site_index`, original write owner/site, `call_path` and projected
`mapped_root` separately from direct assignment counts. Repeated paths remain
distinct while completed summary proofs are reused. Third edges, recursion,
aliases, unsupported signatures and opaque calls stay incomplete; this is no
purity, density, runtime-frequency or rewrite-safety claim. Existing owner/call/site
budgets bound summary/path work. Replay recomputes from immutable original facts,
without compiler tools or live source. See `docs/clones-mutation-effects.md`.


`clones --effect-density [--effect-config PATH]` requests a separate static-event
policy metric in `mutation_effects`. It shares completed scalar helper proofs,
deduplicates direct escaped roots, counts each delegated write call once, and
classifies configured non-mutation call paths through at most two edges. Safe
external free calls need independent native identity/safety, owned-u32 arguments
and unit/u32/never results. Retained same-unit helpers with original owned-u32
parameters/arguments can propagate these classified events, with complete body
boundary checks and no escaped direct writes. The external edge counts toward
the two-edge bound; mutation-summary completeness remains independent.
Unknown work prevents ratios and size exemptions even
at weight zero. Complete counts use legacy weights/minima and checked milli
arithmetic; `--fail-on-findings` uses visible density findings. Named density
suppressions preserve raw measurements; explicit config exclusions preserve raw
unavailability. Both request flags require both axes complete. Config-only replay
uses captured source policy without live source/tools. This is a static policy
metric, not runtime frequency or purity. See `docs/clones-mutation-effects.md`.


Saved density choices use review-store schema 7, `density_review`
(`rstyle.density-review/1`), and `--dismiss-density FUNCTION_KEY` with an explicit
saved run, review file and reason. Exact complete event distributions,
statements/code lines, all density policy and original owner/build identity are
retained. Changed numeric evidence/policy is stale; equal complete numeric
measurements may retain a choice without semantic approval. Reviews hide only
presentation/finding exits; raw density evidence and completeness remain.
Status/prune requires a selected completed numeric negative under the exact
retained policy. Unknown, exempt, source-suppressed/excluded, unrequested and
budget-limited owners cannot retire choices. Full/compact replay needs no live
source or compiler tools. See `docs/clones-mutation-effects.md`.


For independent density migration comparison after a pinned workspace build, run
`python3 eval/clones-capability/run_density_compare.py --output NEW_DIRECTORY`.
The ten-case source corpus checks exact events, source denominators, effective
policy and visibility. Intentional metric differences have individual review
reasons. Expected unknown cases stay outside the measured denominator and force
`fully_complete: false`; successful acceptance does not establish clean project
coverage or retire legacy style/patterns.


## Density CI baseline inventories

`clones --effect-density --write-density-baseline NEW_PATH` exports a complete
whole-scope numeric inventory to a new file; it cannot use changed-file selection.
Use `--density-baseline PATH --fail-on-new-density` to fail only for selected new
numeric findings or policy/exclusion/build-context drift. Existing
`--fail-on-findings` still includes existing findings. Baselines preserve raw
reports and do not create saved dismissals. Current incomplete analysis exits two;
retained suppression, exclusion, missing owners and unavailable identities remain
explicit rather than resolved. Empty baseline headers still detect policy drift.
Both live analysis and source/tool-free replay work. Original root/inline and
checked ordinary external free-function and concrete local method identities,
and bounded inventory limits apply; see
`docs/clones-mutation-effects.md` before using this as a project-wide CI gate.

## Source-rule CI baseline inventories

`clones --source-rules --write-source-baseline NEW_PATH` exports completed
whole-scope parameter, length and configured-spacing findings. Export creates a
new file and conflicts with changed-file selection. Compare using
`--source-baseline PATH --fail-on-new-source`; new/changed selected visible
findings or policy/exclusion/build-context drift fail separately from ordinary
`--fail-on-findings`. Reviews apply before selected counts; raw findings remain.
Only a completed same-policy numeric negative resolves a retained finding.
Suppressed, excluded, missing and unavailable owners are never silently resolved.
Incomplete analysis exits two and cannot export; empty headers still check drift.
Live capture and explicit source/tool-free replay share completed observations.
The 4-MiB/4,096-entry bounds and checked original owner limitations apply; see
`docs/clones-source-rules.md` before adopting the gate across a project.

## Compiler advisory migration

For existing compiler mutation/effect observations and core fix plans on a clones
run, use `cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory`.
An explicit `--compiler-advisory-config PATH` selects core TOML policy; otherwise
core defaults apply. The `compiler_advisory` report keeps check/test units separate,
missing/generated owners unavailable, and unresolved workspace callees visible.
It reuses `ExprUseVisitor`, core propagation, fix groups and parameter stacking;
strict clone admission is not required. Captured observations support source-free
`--replay-run` and policy reevaluation. These metric versions differ from strict
mutation-effect metrics; zero policy density is not a purity proof.
Add `--compiler-advisory-only` to request compiler advice without strict unit
metadata, source correspondence, call instances/projections, body type enrichment,
local-consumer location records, helper certification, admission or matching.
The existing concrete-type collector still normalizes the complete signature and
retains its interface observations, preserving saved review/baseline identity.
Original native bodies, bindings, source snapshots/ranges, compiler contexts,
generated-function markers and separate source inventory remain captured.
Compiler patterns/plans and their review/baseline workflows still work. Strict
execution is `not_requested`, raw artifacts have no strict namespace, and replay
requires the original mode. Omitted records are not complete empty inventories.
Missing/generated owners and unresolved calls remain incomplete, including in
test builds. Source rules/guard advice, `--mutation-rules`, `--mutation-effects`,
`--effect-density`, and parameter/guard plans can run in advisory-only mode. Fresh requests reuse the
existing original file census within its256-file/8MiB limits; compiler-only
requests omit it. Older raw captures without that ownership stay unknown on source
replay. Missing optional mutable-formal/control types
in older raw captures stay unavailable. Fresh mutation requests reuse the existing
bounded collectors; signature bindings share the completed interface type capture.
Compiler mutation events remain a separate metric. Exact known regions do not
establish complete caller/planning coverage. Scalar summaries and density reuse
the existing bounded session/cache; density-only completion remains separate
from scalar mutation completion. Unknown events cannot gain ratios through zero
weights. Unsupported shapes may differ in diagnostic/work-budget details from
combined body enrichment; no complete measurement follows. Pair, dispatch and
capability consumers still require the combined mode;
ordinary raw/strict capture and explicit instance diagnostics retain their records
and existing census budgets. Large advisory captures can publish useful findings
without establishing complete coverage or a clean result.
The version-6 shared bundle preserves deep generated HIR through flat preorder
transport. Generated functions remain unavailable to the advisory collector;
their JSON nesting no longer discards original functions in the same unit.
Older version-4/5 bundles retain their original grammar and strict checks.
`--changed-files PATH` selects numeric compiler diagnostic presentation and finding
exits after reviews. Raw evaluations/fix plans remain intact; proposals stay
global by default and invalid suppressions always stay global.
Add `--select-compiler-patterns` with `--compiler-patterns --changed-files PATH`
to select proposals touching a changed file at any usage site or definition.
`patterns.selected_candidates` records the file mask independently of review
visibility. Text, finding exits and new-proposal baseline counts intersect both;
raw proposals, classifications, plans and baseline header checks stay global.
Empty selection cannot hide incomplete inputs. Selection reuses captured paths
and works in source-free replay. `changed_scope` reports file owner availability;
unknown files or whole-scope missing inputs still prevent a clean result.
Add `--compiler-complexity-top N` (positive N) to rank clean and flagged functions
per unit using completed core COG/CC measurements. Policy exclusions apply;
changed-file/review masks do not filter the raw ranking. Only N score/location
rows are copied; compiler bodies and completed measurements are borrowed for
sorting. Missing scores/owners remain incomplete even outside the displayed N;
historical inputs without scores cannot become zero measurements or clean exits.
The optional `complexity_ranking` pairs `owners` with `functions` and reports full
measured/unavailable denominators. It works in source/tool-free replay and
advisory-only mode, without new source scans or hashes.

For independently labeled numeric validation, the offline
`eval/clones-capability/compiler_complexity.py --require-scores` compares exact
completed policy/CC/COG values separately from threshold outcomes. Missing or
truncated rows remain unavailable; a clean diagnostic result cannot imply zero
or validate an unobserved score. Numeric validation requires completed core
evaluation membership before inferring a clean outcome. Complexity and parameter
plan source labels require an independently reviewed `unsuppressed: true`;
captured-but-excluded owners remain unavailable. No extra runtime scans are added.
See [compiler advisory](../../docs/clones-compiler-advisory.md).

When the caller and dependency workspace packages are selected, compiler effects follow
unique captured direct Cargo edges to known free functions. The report's
`resolved_dependency_callees` maps compiler IDs to report-local unit IDs.
Core propagation borrows captured owners and completed effects move into rule
evaluation, with no extra hash, artifact scan, source proof or body/graph copy.
Hop limits and suppression boundaries are unchanged. Missing, generated, excluded,
ambiguous and associated-method targets remain unresolved; indistinguishable
check/test graph descriptors never share inputs. Cross-unit plans require the
explicit flag below; proposal mining remains per unit. Strict clone admission
and pair budgets remain independent.

Add `--compiler-plans` to reuse the existing core fix groups and parameter
stacking across uniquely resolved captured dependency units. The optional
`compiler_advisory.planning` reports exact unit-qualified member, step and bundle
carrier keys alongside core plans. It consumes completed rule owners and the
already resolved effects graph; no new body proofs, hashes or graph copies.
It can also be requested for the first time during saved-run replay. Raw plans
retain the full assessed scope when reviews/changed files filter diagnostic
presentation.
Missing/ambiguous inputs remain incomplete; external callers and cross-context
physical edit overlaps are not certified, and bundle type names remain heuristic.
Default per-unit evaluation and strict clone matching stay unchanged.

Add `--compiler-patterns` to reuse existing compiler graphs and the core
trait/capability/generic/helper/dispatch proposer. Graph collection is opt-in;
saved runs without graphs report unavailable inputs. Each unit's `patterns`
report includes actual default clustering policy, input coverage and existing
core proposals/counter-evidence. Explicit core path exclusions apply before
mining. `--fail-on-findings` includes candidates; incomplete inputs still exit 2.
Graphs and proposals remain advisory, independent of strict matching. No ignore
ledger is implicitly loaded. Keep legacy patterns until its retirement gates pass.
Default core clustering borrows captured graphs without copying their contents;
enabled normalization retains owned rewrites. No extra hashing or source proof
is required for that reuse.

Compiler helper/generic notes also borrow the completed effects already used by
core diagnostics. `core-policy effect mismatch` compares recorded kinds under
`compiler_advisory.effective_config`, including its bounded propagation depth;
empty kinds do not mean purity. `patterns.effect_evidence` identifies that basis,
the parent metric/build context, and effect-input completeness separately from
graph completeness. Missing owner results are errors; unavailable owners or
unresolved workspace callees keep effect coverage incomplete. This display-only
annotation changes neither ranking/membership nor strict results. Structural
reviews/baselines keep their existing identity; these notes are recomputed under
the current effect policy and are not themselves reviewed policy evidence.

Compiler proposals accept `--review-file PATH` for read-only exact lookup. Only
after the user agrees a finding should remain separate, replay its explicit run
and pass `--dismiss-compiler-pattern UNIT CANDIDATE_ID --review-reason REASON`.
The numeric unit and ID come from that saved report; IDs are selection handles,
not persistent equality authority. Full captured graph/signature/role/policy and
build-context equality hides reviewed proposals. Site graphs are shared without
cloning, hashes or repeated body proofs. Schema 8 preserves earlier reviews and
uses the shared atomic publication; stores have a 4 MiB limit. Raw proposals stay
visible in JSON with separate review decisions and visible candidate indices.
`--review-status` includes retained compiler proposals. Missing proposals remain
unavailable because bounded discovery cannot prove absence; these entries are
never automatically pruned. Compiler proposals also support the explicit baseline workflow below.


Compiler diagnostics also accept `--review-file` for read-only exact lookup. Only
after the user agrees a finding is acceptable, replay its explicit run and pass
`--dismiss-compiler-diagnostic UNIT DIAGNOSTIC_INDEX --review-reason REASON`.
Use the zero-based index from that saved report. Schema 9 preserves earlier
families; full numeric observations, metric versions, signature, effective policy
and build context control equality. Raw diagnostics/fix plans remain intact,
with separate visible indices and decisions. Invalid suppressions cannot be
reviewed away. Explicit status and diagnostic baselines reuse completed core
suppression and owner scopes. Known selected owners under the same policy and
versions can establish numeric negatives; density also needs complete propagation.
Suppressed/excluded/unavailable inputs cannot establish negatives. Unselected
owners remain not-requested; missing owners stay not-observed or unavailable.
Only explicit saved-run `--review-prune` removes completed negative choices;
`--review-prune-dry-run` previews. Default analysis retains no extra scope roster.
Owners/policy values are shared; no graphs, repeated parsing, body proofs or hashes
are added. Incomplete compiler coverage keeps exit 2 even when all findings are hidden.

Compiler retained status reuses the complete declaration-name counts already
computed for diagnostic identities, including excluded and ambiguous names.
Matching package/target feature variants share this census; check/test targets
stay distinct. The existing 4,096 owner-payload cap per unit remains: known
unique retained owners can be assessed, while omitted payloads stay unavailable.
Changed policy makes a current numeric negative stale, so it cannot prune the
retained choice. Whole-scope compiler baseline export keeps its existing limits.


Compiler diagnostics support `--write-compiler-baseline PATH` for a complete
whole-scope inventory, then `--compiler-baseline PATH --fail-on-new-compiler`
for new/changed visible findings and policy/build-context drift. Use the same
explicit compiler policy and saved run as needed. Comparison reuses completed
numeric snapshots and shared baseline I/O; no hashes, body copies or extra
analysis. Raw diagnostics/fix plans remain intact; `--fail-on-findings` still
counts existing findings. Reviews can hide new findings but cannot hide header
drift. Old findings resolve only through the same completed owner/rule scopes
used by numeric review status; missing positives alone never imply resolution.
Incomplete or oversized scope exits 2 and cannot export; bounds are 4 MiB and
4,096 owners/entries/contexts. Exports create a new file without overwriting.
Changed-file selection filters new visible numeric findings in comparisons;
whole-scope policy/context drift remains global. Export conflicts with selection.
No proposal-baseline or legacy-retirement claim follows from this workflow.


Compiler proposals support `--write-compiler-pattern-baseline PATH`, then
`--compiler-pattern-baseline PATH --fail-on-new-compiler-patterns`, requiring
`--compiler-patterns --compiler-advisory`. Completed exact review snapshots feed
the shared baseline lifecycle without another mining pass or graph copy/hash.
The whole effective compiler policy, default clustering policy and build scope
participate even for empty inventories. Raw proposals/fix plans remain intact;
reviews affect visible new counts, while exports retain raw proposals. Comparison
stays global under changed-file selection; export conflicts with selection.
Missing proposals stay unavailable because bounded discovery cannot prove
resolution. Incomplete inputs exit 2 and cannot export. Shared 4 MiB and 4,096
owner/entry/context bounds and create-new publication apply. Source/tool-free
replay supports full/compact reports. Keep legacy patterns until its independent
quality and retirement gates pass.


Compiler proposal policy version 2 rejects fallback helpers whose completed
alignment contains only distinct, single-use callee substitutions and
parameter/return glue. The existing call-count pass collects the shape flag;
completed alignment holes supply the decision. Shared calls, repeated
substitutions, unknown identities and additional work retain advisory review.
Built-in decisions appear in `patterns.evaluation.suppressed` with source sites
and a reason; these are independent of user ignore/review entries. Earlier
compiler proposal reviews are readable but stale by detector version, and
version-1 compiler proposal baselines require explicit regeneration. No graph
schema, compiler metric, strict admission, hashing or body-proof change follows.


For retained compiler density reviews and baselines, `transitive_depth=0` uses
the known owner's completed direct core assessment. Unrelated unavailable owners
or unresolved callees do not prevent that same-policy numeric negative. Deeper
policies still require complete propagation inputs. Capture/handoff failures,
missing queried owners, suppression/exclusion, selection and version/context/
policy checks remain in force. Status is read-only; only explicit saved-run prune
removes eligible review entries. Global incompleteness and export restrictions
remain unchanged; this numeric result does not establish purity.


Compiler advisory metric4 reuses rustc's instance selection during the existing
collector walk to record concrete ordinary trait implementation call edges.
Method and UFCS calls feed the existing effect propagation and parameter planner;
UFCS receivers stay outside the parameter list. External effect-path and safety
classification keep the original call signature. Generic, dynamic, inferred and
non-Item targets keep their declaration/unknown behavior. Existing owner, Cargo
route, generated/missing/excluded and default-body alias handling remains in
force. Metrics1–3 remain readable with their original measured edges and counts;
metric drift makes retained diagnostics stale and baseline scopes different.
This adds no hashing, repeated source proof, graph copy or strict permission.


Compiler advisory metric5 uses the declared workspace source root independently
of package ownership. Authored shared `#[path]` modules inside that workspace can
supply the existing collectors even when outside a member package directory.
Explicitly captured dependencies outside the workspace retain their own package
root. Compiler locations normalize lexically through the existing path helper;
workspace-relative exclusions therefore treat relative and absolute spellings
consistently. No filesystem canonicalization or new checksum is used. Foreign
local source files and generated owners remain unavailable, and test/check
compilation contexts remain distinct. Metrics1–4 retain captured paths and
measurements on replay; unavailable-only bundles have no observed metric version
and retain the existing current-header fallback. Metric changes remain visible
to reviews and baselines.
Strict source census, extraction permissions and matching are unchanged.


Review and baseline locators normalize absolute source and root paths with the
existing lexical path helper before checking containment. Within-root shared
modules such as `src/../shared.rs` therefore use `shared.rs` in saved locators.
Equivalent lexical spellings preserve review identity; outside-root, relative,
empty and non-UTF8 locators remain unavailable. This uses captured paths without
filesystem canonicalization, new hashes or metric/context changes.
