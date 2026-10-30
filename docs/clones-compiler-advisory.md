# Compiler advisory reuse

The normal style command offers an opt-in route:

```sh
cargo rstyle --style-engine compiler -p my-package --format json
```

It selects all authored compiler diagnostics and the existing cross-unit planner,
uses workspace `rstyle.toml` or global `--config`, and returns0/1/2 for completed
clean/findings/incomplete-or-error. Package, manifest, all-targets and format
selection use the existing capture path. Strict matching and pattern mining are
unrequested; the pattern ledger does not affect style findings. The compiler
JSON exposes the usual `diagnostics`, `summary`, `fix_groups` and complexity
`functions` fields, borrowed from completed evaluations and the cross-unit planner.
The parallel `diagnostic_units` and `function_units` arrays identify each row's
compilation context through `unit_contexts` and `observed_units`; identical source
locations in different builds remain separate rows. The summary combines exact
completed density totals before rounding. It describes known evaluated functions;
`summary_complete=false` identifies partial scope, and no completed evaluations
produce `summary=null`, rather than a fabricated clean summary.

`compiler_advisory` retains per-unit evaluations, metric versions, outside-scope
generated owners and limitations. `planning_input_complete` and
`planning_required_edits_complete` preserve the planner's independent statuses;
numeric completion does not imply complete callers or safe rewrites. The full
clones evidence report remains at `<run_directory>/report.json` for inspection and
saved replay. This is an extended style report, with compiler metric semantics;
equal field names do not imply equal legacy density denominators or scores.
The legacy backend remains the default, available explicitly through
`--style-engine legacy`. The style-engine option applies only without a subcommand;
`patterns --engine compiler` retains its separate opt-in route.

Live advisory-only captures without dependency capture reuse private builds in
`target/rstyle/clones/advisory-build`, under one lock through planning, compilation
and reconciliation. Completed raw owner snapshots are reused only when current
Cargo reports the exact artifact fresh and the build/collector plan and tool
metadata match. Source changes follow Cargo's normal freshness decision. Missing
or invalid indexes, missing snapshots and collector/tool changes force selected
package cleanup and recapture, including host and target variants. A failed build
leaves no completed index; an unmatched fresh artifact retries with cleanup in a
new run directory. Consumer policy changes reevaluate retained raw facts.

Each successful run owns its evidence files, so saved replay remains independent
of previous runs and builds. Compiler incremental compilation stays disabled;
strict and explicit dependency captures retain isolated per-run build directories.
No source or compiler-artifact content scans or new hashes identify cache entries.

The compiler wrapper clears extraction controls before applying the selected
capture plan. It preserves Cargo build-script `rustc-env` values and arbitrary
project compilation inputs, including names beginning with `RSTYLE_`. Those
inputs must reach rustc unchanged on cold, reused and changed builds.

Measured warm rstyle-core captures took1.08–1.18s with all31 artifacts fresh and
no rebuilt owners, compared with the preceding5.20–5.35s owner-rebuild observations.
Reports and numeric summaries agree. These single-package observations are not a
randomized or equal-cache benchmark. Broader default-readiness comparisons remain
open; see `eval/clones-capability/project-advisory-owner-reuse-study.json`.

`clones --extraction-profile hir-data-v3 --compiler-advisory` captures the existing
Rust style observations during the same compiler invocation. It calls the driver
mutation collector (`ExprUseVisitor`) and effect/call/parameter-use collector,
then passes their `FnFacts` to `rstyle_core::evaluate`. That evaluator supplies
rule policy, bounded call-graph propagation, fix groups and parameter stacking.
There is no second implementation of those algorithms and no strict clone
admission requirement for these observations.

Add `--compiler-advisory-only` when you want compiler advice without strict clone
analysis. This explicit mode reuses the same collectors, evaluator, compiler
patterns and compiler plans, but skips strict source correspondence and instance
projections, helper certification, strict bundle import, admission and pair matching. `matching_execution` and
`strict_assessment_execution` are `not_requested`; the matching payload is absent
and pair-completeness flags are false. Success still requires complete capture,
handoff and requested advisory consumers. Missing owners remain incomplete;
unresolved callees also keep default all-rule style/effect coverage incomplete.
Use `--compiler-patterns` without `--compiler-advisory` for structural proposals
without numeric rule evaluation. The raw capture mode accepts either consumer. Local source, guard, mutation and
effect consumers can also run alone with `--compiler-advisory-only`; no compiler
numeric observations are captured or evaluated unless a compiler consumer is
requested. A local-only report omits `compiler_advisory`.

Fresh compiler-input captures also retain direct mutation, external-call and
intrinsic-effect events for macro-generated function signatures. Per-unit
`generated_effects` rows give the function key/name and expansion callsite, raw
events and classifications under the ordinary core config. They use the existing
collectors and classifier; source/tool-free replay can reclassify retained calls.
Fresh captures use `rstyle.compiler-generated-effects/2`, retaining function IDs,
implemented trait methods and call edges already produced by those collectors.
Borrowed authored/generated owner views feed the existing core call graph and
bounded effect propagation once. Authored callers can therefore inherit observed
effects through generated functions. Per-unit generated rows also expose the
completed `propagated_effects`. Configuration, propagation depth, authored
suppression boundaries and the local-only unsafe/unknown rules remain unchanged.
Generated expansion comments do not supply authored function suppression scopes.

These partial rows do not provide authored source, complexity, density or
structural graphs. Generated owners remain in `unavailable`; default all-rule coverage
remains incomplete by default, and the default exit remains 2. Empty observations do not establish
purity; opaque behavior and unresolved calls remain coverage gaps. Edit plans
retain generated caller topology but print source edits only for authored owners.
Required generated edits remain unavailable; unrelated generated functions do not
make an otherwise complete authored edit worklist incomplete.

Fresh authored aggregate metrics use `rstyle.compiler-style-metrics/7` to identify
the additional propagation. Historical metric-6 replay preserves its original
results; generated-effects/1 records remain direct-only. Older captures without
a generated record produce no row, rather than a fabricated empty measurement.
Strict matching remains unchanged. No additional hash, source verification pass
or authority proof is used.

For a numeric workflow, select the rules and population explicitly:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory-only \
  --compiler-advisory --compiler-diagnostic-scope authored \
  --compiler-diagnostic-rules fn_length,fn_params,fn_spacing,cyclomatic_complexity,cognitive_complexity
```

Both selection flags require `--compiler-advisory`. Rule IDs also include
`mutation_per_target`, `mutated_targets` and `side_effect_density`. Omission keeps
the existing all-rule, captured population. Invalid suppressions always surface.
The authored population lists explicitly generated missing owners outside scope;
missing authored observations and unknown origins remain incomplete. Missing
complexity matters when a requested rule consumes it. Density requires
the existing complete effect inputs and resolved calls when propagation depth is
positive. With existing `side_effects.transitive_depth=0`, explicitly requested
authored density uses only the applicable direct events; missing generated source
and unresolved callees do not block that numeric result. Missing authored facts
still do. The default without diagnostic selection remains unchanged. Explicit
captured selection still requires every captured owner's direct observations.
For inherited density, current connected generated-effect rows can supply the
required callee events without authored numeric facts. Every missing effect owner
and unresolved target still blocks completion. Old direct-only rows cannot supply
call connections. Generated numeric inputs remain unavailable, so default/captured
numeric checks and authored edit plans retain their separate incomplete status.

Each unit's optional `diagnostic_scope` records the request, outside-scope owners,
unavailable counts, requested completeness and original `diagnostic_indices`.
The full `evaluation` and its summary remain the unchanged core result across all
rules; its effect measurements can be incomplete even when requested numeric
rules complete. `effect_inputs_complete` states this separately. Top-level
findings, text, review visibility, changed-file presentation and plan seeds use
the requested indices. Plans retain their full caller graph and its separate
completeness requirements. Baselines record rule/population scope even when empty;
scope changes cannot establish a clean comparison. Saved reviews for unrequested
rules are not retired as numeric negatives. No new collector or evaluator runs.

The mode publishes the existing shared raw-facts format with an exact
`compiler-advisory-only` phase, using the existing streaming transport seal and
successful Cargo/unit ledgers. Raw function references identify
`artifact_kind: advisory-raw-facts` and carry no strict namespace. They cannot be
imported as strict evidence. Replay requires the original capture mode and can
run without source files or compiler tools. Selected dependencies and separate
check/test contexts keep their original Cargo scope. Compiler reviews, changed
files, policy reevaluation and baselines use the ordinary advisory workflow.

Source rules, guard advice and scalar mutation/effect consumers can be requested alongside this mode with
`--source-rules`, `--guard-clauses`, `--mutation-rules`, `--mutation-effects` and
`--effect-density`, including `--parameter-plans` and
`--guard-plans`. Fresh captures then reuse the existing original file census,
sharing compiler-owned buffers and checksums. Compiler-only captures retain their
existing path. Local source capture preserves the census limits of 256 files and
8MiB per unit; exceeding them fails capture, rather than publishing truncated
ownership. It captures no namespace/import/documentation/expansion proof inputs.
All existing source ownership, generated-function, syntax, caller and planning
coverage checks still apply. Conditional callers and opaque bodies can leave
planning incomplete even with exact known edit regions.

Replay cannot add a missing census to an older advisory-only capture: source
assessment remains unknown, with incomplete coverage and exit 2. No live source
fallback is used. The saved plan records whether local source facts were
requested; replay keeps the original optional evidence and compilation contexts.
Mutation, summary and density requests additionally reuse the existing mutable-u32 formal and builtin
control-type collectors. Signature capture normalizes parameter bindings through
the same completed type observations, without enriching the body. Old raw captures
without these optional observations remain incomplete on mutation replay. The
direct-u32 vocabulary, policy, numeric reviews and unknown boundaries are unchanged;
compiler mutation events remain a separate metric.
The existing bounded summary session and density event cache consume these raw
observations directly. Density-only execution requires its own complete event
and statement evidence; requesting scalar mutation summaries additionally requires
that axis to complete. Unknown calls cannot gain ratios through zero weights.
Unsupported native shapes may retain different diagnostic/work-budget details
from combined captures because their extra body types are absent; neither grants
complete measurements. Saved density reviews, baselines and changed-file selection
retain their existing contracts and cannot hide global incompleteness.
Pair, dispatch and capability consumers still require the ordinary
combined mode. Raw extraction keeps the original
source/type observations, source ownership, census and generated-source markers.
It omits strict correspondence and generic/slice instance projections. Raw source
diagnostics therefore omit correspondence processing; compiler advice uses its
independent observations and keeps the same availability outcomes. This mode
does not make extraction free or change any strict matching permission.

The JSON `compiler_advisory` field contains separate compilation-unit results,
source-located diagnostics and fix plans, observed statement/mutation counts,
missing-function reasons and unresolved workspace callees. Diagnostic paths are
relative to the reported workspace `source_root` (absolute for outside sources).
The separate `package_root` identifies the selected package. Locations use the
already captured source path; replay never consults the filesystem to resolve it.
Check and test units never merge. Raw
observations include excluded functions; the core evaluator applies exclusions
and original source suppressions to its own evaluation. The report includes its
full effective configuration.

Use `--compiler-advisory-config PATH` for an explicit TOML core policy. Without
that flag the core defaults apply; the workspace's `rstyle.toml` is not implicitly
loaded. `--fail-on-findings` includes advisory diagnostics. Incomplete coverage
still exits 2, even if assessed functions have no findings. An explicit `--review-file` applies exact saved choices to these diagnostics.
An explicit `--changed-files PATH` selects numeric diagnostic presentation and
finding exits as described below. The compiler diagnostic baseline workflow is
also described below. Compiler
proposals have a separate review identity described below.
The normal legacy style command remains available.

Both the record and metric identities are versioned:
`rstyle.compiler-advisory/1` and `rstyle.compiler-style-metrics/7`. These are
compiler style metrics, not the newer source/native mutation-effect contract.
Mutable borrows count as mutation events; compiler statement counts use the
existing HIR traversal, excluding synthetic question-mark arms in metric 3;
effect propagation uses the existing bounded depth, boundaries and
trait-target policy. Unclassified external paths are governed by the reported
core configuration. A policy density of zero does not establish effect absence,
purity, termination, or safe refactoring.

Metric 6 reuses the shared [complexity policy](complexity.md). The existing
expanded-AST collector runs once for an explicitly requested advisory capture,
before HIR lowering adds synthetic branches. Its results are borrowed by the
shared function collector, which adds the already collected direct-recursion
events and scores them with `rstyle_core::complexity`. The existing evaluator
supplies cyclomatic/cognitive diagnostics, contribution details, ranking,
thresholds, exclusions, suppressions and fix groups. There is no separate
advisory complexity algorithm. Strict clone eligibility and matching are
unchanged.

The core report summary exposes `complexity_measured` and
`complexity_unavailable`. Modern owners without a score are also listed in
`unavailable_complexity`; other observations remain usable, while the advisory
scope stays incomplete. Missing measurements cannot retire saved complexity
reviews as clean negatives. Exact numeric reviews and baselines use the existing
compiler-diagnostic workflow for both complexity rules. Replays use completed
measurements without rereading or parsing source. Historical metric versions
remain readable with unavailable complexity and retain their original metric
scope; they cannot claim modern measurements or establish current review
negatives.

Add `--compiler-complexity-top N` (positive N) to show the highest COG, then CC
scores per compilation unit, including functions below diagnostic thresholds.
This presentation reuses completed core measurements and its existing source
location/name tie order. It borrows all selected facts for sorting and copies only
the requested display rows; it does not rescore, parse source or add proof/hash
checks. Core policy exclusions apply; diagnostic suppressions and review or
changed-file masks do not filter this raw ranking. Finding counts stay unchanged.

Ranking can also run without `--compiler-advisory` or `--compiler-patterns`:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory-only \
  --compiler-complexity-top 20 --locked --format json
```

This uses the same captured scores, core observation selection and ordering,
without numeric diagnostics, effect classification/propagation or fix planning.
`diagnostic_execution` and `effect_execution` are `not_requested`; absent effect
results do not mean pure or measured-empty. Direct unresolved calls stay visible;
without propagation, they do not include generated-body routing or its unresolved
callees. They do not invalidate complexity scores. Missing scores or owners still
prevent a complete ranking in the default captured population, including generated
signatures with unavailable measurements.
Add `--compiler-advisory` for threshold findings and effects, or
`--compiler-patterns` for proposals; each retains its own input requirements.
Ranking alone has no findings to dismiss or threshold exit; `--fail-on-findings`
requires a requested finding analysis. Strict matching stays independent; the
example explicitly leaves it unrequested.

Add `--compiler-complexity-scope authored` to rank authored functions explicitly.
It requires `--compiler-complexity-top`. Expansion-generated signatures stay in
the raw unit `unavailable` roster and are listed in ranking
`source_scope.outside_scope_functions`; they do not count as missing authored
scores. Missing authored scores and unknown origins still prevent completeness.
Default/explicit `captured` scope is unchanged. This scope applies only to ranking;
numeric diagnostics, effects and compiler-pattern populations keep their own
requirements and scope. It works in source/tool-free replay without recapture.

Each unit's optional `complexity_ranking` uses schema
`rstyle.compiler-complexity-ranking/1`. Parallel `owners` and `functions` arrays
pair report-local handles with score/location rows. `known_functions`,
`measured_functions`, `unavailable_measurements` and `unavailable_owners` describe
the full input, independently of `requested_limit`. Missing measurements sort
last and retain an unavailable score; missing owners remain in the unit's
`unavailable` list. Either makes the requested ranking incomplete within its selected population, including when
replaying historical metrics without scores, and prevents a successful exit.
The limit truncates presentation only. The flag works on fresh captures and
source/tool-free replay, including advisory-only mode. Without it, the existing
report is unchanged. Rankings over incomplete input are not whole-project maxima.
The offline [complexity validator](../eval/clones-capability/README.md) accepts
`--require-scores` to compare these completed rows with independent numeric
labels, including clean functions. Missing/truncated rows remain unavailable;
ordinary threshold-outcome comparisons stay separate. Numeric validators require
completed core evaluation membership before interpreting absent diagnostics as
clean; captured owners excluded by policy remain unavailable. Complexity and
parameter-plan labels require an independently reviewed `unsuppressed: true`
precondition. These checks reuse existing report data without new runtime work.

Metric version 2 omits lowered iterator roots from source mutation budgets.
The shared expression-use collector tags these bindings by their HIR
origin and `ForLoop` span, without parsing or rechecking the source. Explicit
iterator bindings (including one named `iter`), borrowed source iterators,
mutable loop-pattern bindings, closure captures and macro-local writes still
count. All mutation events, escaping/delegated effects and workspace type hints
remain available. Raw `direct_mutations`/`escaping_mutations` count those retained
events; source budgets filter the typed `for_desugaring` origin before grouping.
Metrics 2 through 6 require explicit origins, while historical missing origins mean source
under metric 1. A metric-1 record cannot claim a desugared origin. Statement
counts were unchanged in metric 2; budget exclusion does not establish iterator purity.
Legacy style reuses the mutation-origin correction.

Metric version 3 excludes the two synthetic bare match arms created by each `?`
from statement counts. The collector reads the existing arm-span
`QuestionMark` desugaring marker; it does not parse source, recheck types or walk
the body again. Authored match arms, operand-block statements and all calls and
effects remain collected. Legacy style explicitly retains its original statement
metric. The two extraction entry points select their metric in the same collector;
only synthetic question-mark arm counting differs. The 44 density findings seen
when applying the corrected denominator to the legacy workspace gate remain
quality debt to assess before retirement, not resolved findings.

Version-1 and version-2 saved compiler observations remain readable and replay
with their original counts and metric identifiers. No replay relabels or
reconstructs them as version 3. Mixed metric versions in one advisory analysis are rejected;
run their scopes separately. Diagnostic review/baseline artifacts retain the
actual observed version, including historical exports. A version change makes
positive reviews stale, prevents numeric negative retirement, and changes even
an empty baseline's scope. Historical artifacts remain readable. No hashes,
artifact scans, graphs, body copies or additional source proofs are needed.

Captured observations replay through `--replay-run`, including changed core
policy, without Cargo, rustc or original source files. Runs captured without the
flag retain unavailable observations. Expansion-generated functions unavailable
to the original collector remain unavailable. Missing facts never become empty
function records; the adapter rejects omitted fields that older `FnFacts`
deserialization would fill with zero/empty defaults. The enclosing bundle and
selected compiler invocation retain the compilation context. No new hash is used.
Strict clone matching and certification do not consume this advisory record.

Shared bundle version 6 encodes deep HIR bodies as flat preorder records, so
compiler-generated derives cannot lose an entire unit's advisory observations
solely through recursive JSON nesting. Every node and its order are retained;
generated functions still have the collector's original unavailable outcome.
The JSON nesting limit, source census budget and strict checks are unchanged.
Older version-4/5 bundles remain readable within their original tree grammar.

## Cross-package compiler effects

Captured non-test selections can reuse core effect propagation across
uniquely identified direct Cargo dependency edges. Select both workspace packages
with the existing package selection; missing packages are not captured implicitly.
Each callee remains scoped by its report unit and compiler function ID. The
adapter uses the existing captured Cargo unit graph, planned contexts, target,
features and platform, requiring a unique correspondence. It performs no new
artifact scans or checksums. Renamed imports preserve the compiler's callee ID.

`units[].resolved_dependency_callees` maps resolved exact compiler definition IDs to their
report-local dependency unit IDs; unresolved IDs retain the existing roster.
Facts are borrowed while core constructs and propagates the shared call graph.
Completed effect vectors then move into each unit's ordinary rule evaluation,
without copying bodies/graphs or repeating propagation. Source rules, fix groups,
parameter plans and proposal mining remain separated by compilation unit by default.
Existing hop limits, outward effect kinds and suppression boundaries apply to
cross-package calls exactly as to local calls. This is the existing compiler
metric, not a purity or semantic-equivalence proof.

This connection covers uniquely identified free functions and inherent associated
functions/methods, including generic inherent bodies. Signature metadata does not
exclude an exact compiler target. Trait implementation aliases alone cannot
resolve a dependency call: the captured definition ID must equal the callee ID.
This does not add open-world trait-dispatch resolution. Same-path functions in
multiple direct dependencies,
uncaptured/generated/excluded targets and ambiguous Cargo contexts remain
unresolved. Cargo can emit indistinguishable descriptors for check/test library
occurrences; neither is chosen by order. A dependency's missing propagation inputs
also prevent completed density negatives, while direct rule assessments remain
independent. Without the planning flag below, single-unit runs retain their
existing evaluation path. Matching
budgets and strict clone admission do not grant or prevent advisory propagation.
Source/tool-free replay uses the same captured graph and owner identities.

## Cross-unit compiler planning

Add `--compiler-plans` to `--compiler-advisory` to run the existing core fix-group
and parameter-stacking planner over the same resolved graph used for compiler
effects. This is an analysis switch, so an already captured advisory run can
request it with `--replay-run` without source files, Cargo or rustc. Without this
flag the shared graph and planning owner registry are not retained.

`compiler_advisory.planning` has schema `rstyle.compiler-planning/1`, the known
function count, captured `input_complete` status, limitations and groups. Each
group contains the ordinary core `evaluation` plus report-local function keys:
`members` align with its functions, `step_owners` align with plan steps, and
`bundle_owners` retain the distinct carriers of each bundle. Source paths/names
remain presentation fields; owners stay unit-qualified even when locations or
names coincide. The existing core text formatter also displays these plans.

`known_functions` counts authored planning owners; `known_call_graph_functions`
also counts connected generated owners. `call_graph_complete` describes the
captured resolved graph under the selected scope, using the existing propagation
coverage. It does not enumerate unknown dynamic targets or callers outside that
scope. Full `input_complete` still requires authored diagnostic/source inputs.
The existing grouping footprints and shared-caller hints use all known vertices,
so a common generated caller can connect two authored signature fixes. Authored
parameter steps count known incoming generated callsites. Each group's
`unavailable_owners` lists required edit owners without source/parameter facts;
they produce no invented step coordinates, parameter bundles or pass-through
claims. `required_edits_complete` can be true with incomplete full inputs when
the requested authored worklist needs no unavailable edits and its required
caller inputs are complete. Body-local edits still need no caller census.

Completed owner-indexed rule results feed the planner; it does not evaluate
rules again, clone graphs/bodies, parse source, check hashes or resolve dependency
edges again. Per-unit diagnostics, summaries, fix groups, findings and strict
matching remain unchanged. After planning, original facts move into optional
proposal mining. Named suppressions, core grouping/caller-hop policy and path
exclusions apply. Reviews and changed-file selection affect diagnostic
presentation as before; these raw plans continue to show the whole assessed scope.

A plan describes captured callers and heuristic edits, not a safe rewrite or
whole-program independence. Missing/generated/excluded targets and ambiguous
contexts keep inputs incomplete. Even `input_complete=true` does not enumerate
callers outside the selected Cargo scope. Same-body edits across compilation
contexts may overlap. Parameter name/type bundle keys and shared-type hints
retain core's advisory heuristic policy; they do not establish cross-unit semantic
type identity. This flag neither grants strict clone admission nor changes
finding-exit counts. Legacy style/planning remains available.

Add `--compiler-patterns` to collect the existing compiler advisory dependence
graphs and run the existing core abstraction proposer. It can run independently
of `--compiler-advisory`; graph construction does no work without the explicit flag.
It reuses `pdg::pdg_of`, signature mining, trait-call preparation, clustering,
alignment, proposal ranking and existing-trait/abstraction suppression policy.
No new consumer algorithm, body proof, hash or strict clone permission is added.
Collected graphs move between consumers rather than being copied during input
preparation. With numeric style requested, the evaluator retains its selected
facts for the proposer. Without it, a small core input seam reuses the same
merge, exclusion and completed-effect steps, skipping numeric rules, density
summary, fix groups, diagnostic ownership and review snapshots. Supplied completed
effects move without reevaluation or another graph build; original source buffers
also move. The explicit core configuration's path exclusions apply to both.
Default clustering also borrows these graphs: with no normalization rules it
does not copy their nodes, edges or metadata. Explicit normalization still uses
an owned rewrite, preserving the original observations.

The legacy command can opt into the compiler workflow with
`cargo rstyle patterns --engine compiler --format json`. The default engine stays
`legacy`. The compiler route performs one advisory-only HIR-data-v3 capture and
returns the existing scoped compiler report, including run directory, unit
contexts, coverage and counter-evidence; strict matching and numeric rules are
not requested. Shared mining and score/top flags keep their existing meaning.
`--fail-on-candidates` uses displayed compiler proposals; incomplete scope exits2.

This route consumes `--config` or the workspace `rstyle.toml` for core selection
and effect classification. Package, manifest and all-target selection remain
explicit. `--locked` applies to fresh capture. Use `--replay-run PATH` to reevaluate
an exact saved run, including without source/compiler tools; configuration and
review files remain explicit runtime inputs. `--review-file PATH --review-status`
uses the existing scoped compiler review workflow.

Legacy dismissals are not automatically converted into compiler reviews. A
nonempty legacy ledger produces a migration message; `--no-ignore` explicitly
bypasses it and leaves its bytes unchanged. Scoped compiler reviews still apply.
`--show-ignored` lists legacy records and is rejected for the compiler route; use
compiler review status instead. Use the explicit preview/import workflow below for migration.
Compiler-only options reject the legacy engine. No legacy backend is retired.

Preview legacy dismissal migration with
`cargo rstyle patterns --engine compiler --preview-legacy-reviews --format json`.
`--ignore-file PATH` selects a different ledger. Direct clones requests use
`--legacy-pattern-ledger PATH` with compiler-patterns. The explicit ledger must
exist. The preview reads raw proposals before score/top, changed-file or review
filters, reuses the existing legacy matcher and retained scoped snapshots, and
never creates/updates a review file by itself. `pattern_review.legacy_migration`
reports each legacy ID, growth setting, reason and matching unit/candidate handles,
including unavailable snapshot reasons. Missing scope stays unavailable; a
`no_current_match` result refers only to the selected complete compiler scope and
never authorizes pruning the legacy entry.

Import a selected preview match from its saved run with
`--review-file reviews.json --replay-run PATH --import-legacy-pattern LEGACY_ID UNIT CANDIDATE_ID`.
This records the current checked scoped snapshot through the existing atomic
review workflow. It uses the ledger reason or explicit `--review-reason TEXT`;
a nonempty reason is required. Old IDs suggest compatibility matches and do not
become compiler review identity. The existing exact context/body/config lifecycle
still applies. Multiple check/test matches stay separate and require a unit
selector. Growth entries require explicit `--legacy-current-members-only` to
convert the current members to an exact review; future growth is not carried over.
Positive available matches can be imported while other scope is incomplete, but
the run remains incomplete. Failed selectors or snapshots leave the review file
unchanged. Original TOML bytes/comments are never rewritten. After import, use
`--no-ignore --review-file reviews.json` to apply scoped reviews while explicitly
bypassing the retained legacy ledger. No legacy backend or ledger is retired.

Compiler patterns keeps the captured-function population by default. Add
`--compiler-pattern-scope authored` to either entry to explicitly assess original
function declarations, matching the population used by the existing legacy
collector. Only unavailable owners with the captured compiler
`macro_generated_function` marker fall outside this requested population. Missing
observations, missing graphs and unknown-origin owners remain unavailable.
`patterns.source_scope` records the authored selection and every outside-scope
owner handle. The raw unit observations/unavailable inventory and effect
limitations remain intact; outside scope is not a zero metric or a purity claim.

Authored pattern-only analysis can complete in crates containing derives or
macro-generated functions. Numeric diagnostics, effect/planning coverage and
strict clone decisions retain their existing prerequisites. Changed-file pattern
reports count outside-scope owners separately, including generated-only files;
unknown selected files remain incomplete. Positive proposal reviews still refer
to the same body/context. Proposal baselines retain the discovery scope in their
header, so switching between authored and captured populations reports scope
drift even for empty inventories. Historical baselines without the field retain
the captured population. Saved runs can select authored scope using retained
compiler markers without source files, compiler tools or new hashes/proofs.

Compiler proposals accept the existing legacy mining controls directly:
`--threshold`, `--min-coverage`, `--pair-coverage`, `--normalize`,
`--single-call-guard`, `--max-holes` and repeatable `--hide-implements`.
They require `--compiler-patterns` and reuse the same CLI conversion and core
options as `cargo rstyle patterns`; defaults are unchanged. These controls affect
advisory proposals only. They do not relax strict clone matching. Existing
experimental controls retain their experimental status.

Saved captures can be reevaluated with these options without source files or
compiler tools. Reports retain actual `policy` values and a nonempty
`hide_implements` list. Reviews retain both inputs and become stale when either
changes. Baseline headers retain them even when no proposal exists, so a different
configuration produces explicit scope drift. The existing schema-1 records read
an absent hide list as empty: earlier versions could only use the default empty
oracle. Empty lists are omitted on publication, preserving default record bytes.
No hash or source scan is needed for these configuration comparisons.

Compiler patterns also accept `--min-score N` (natural-log units) and `--top N`.
These reuse the legacy inclusive score filter and ranking order per compilation
unit, independently for candidates and signature groups. Reviews hide candidates
before the top limit; changed-file selection intersects the resulting list
without refilling it. Nondefault display settings publish `patterns.presentation`
with candidate/group indices into the complete raw `evaluation`. Finding counts,
text and finding exits use the displayed selection. `--top 0` hides all rows but
cannot turn missing inputs into a complete result. Review identity, baseline
inventories and baseline-new exits remain independent of these display options.
Defaults omit the presentation object and preserve existing reports.

For a standalone capture without strict matching:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-patterns \
  --compiler-advisory-only --locked --format json
```

The containing `compiler_advisory` report sets `diagnostic_execution` to
`not_requested` and omits unit `evaluation` objects. Adding `--compiler-advisory`
evaluates numeric rules as before and sets diagnostic execution to `completed`.
Pattern-only completion depends on captured owners/graphs and selected-file
coverage, while incomplete effects remain explicit in `patterns.effect_evidence`.
It may exit 0 (or 1 with `--fail-on-findings`) despite unresolved effect callees;
requesting numeric advisory on that same saved capture still exits 2. Missing
owners/graphs and failed capture remain incomplete. Strict matching runs normally
unless `--compiler-advisory-only` explicitly skips it.

Each unit gains a `patterns` result with schema `rstyle.compiler-patterns/1`,
the actual clustering parameters and the existing core report, including
signature groups, candidates, definitions and counter-evidence. Paths are
relative to that unit's `source_root` (absolute for outside sources). Text output also displays the core
proposals. `--fail-on-findings` counts proposal candidates in addition to style
diagnostics; signature groups alone do not fail the gate. No ignore ledger is
loaded implicitly. An explicit review file can hide exact reviewed proposals;
explicit compiler proposal baselines are described below. Core candidate IDs retain their existing fingerprint convention and
must not be treated as the newer compilation-context review identities.

Helper and generic-function proposals reuse core's existing recorded-effect
comparison for counter-evidence. A difference in intrinsic effect kinds or
escaping mutations adds a warning without changing their score, candidate ID or
emission. The comparison does not evaluate calls, configured IO or propagated
effects. An empty class means no recorded intrinsic or escaping-mutation effects;
absence of a warning establishes neither purity nor compatible behavior. Trait
proposal score damping retains its existing policy.

Cluster proposals describe aligned graph regions and recorded substitutions.
When the completed mean node-alignment coverage is below 1000, counter-evidence
calls out unmatched nodes and the refactoring sketch requires preserving their
behavior. Coverage averages template-to-member alignment, including the template
at 1000; it is not all-pairs coverage, source-line coverage or semantic equivalence.
Even full coverage with no recorded substitutions does not establish an exact
source clone. Aligned nodes need not form a contiguous source fragment; this
adds no fragment discovery or automatic extraction behavior. Scores, candidate
IDs, membership, thresholds and strict matching are unchanged.

The separate `rstyle.compiler-pattern-graph/1` capture record keeps the original
style metric contract unchanged. Missing graph capture in an earlier saved run
remains unavailable, while a known-empty graph remains observed. The adapter
checks required field presence rather than silently accepting historical serde
zero/empty defaults. It does not reread, reparse or hash original bodies.
`policy_selected_functions` counts observations after core path exclusions;
`observed_graphs` counts graphs in that selected population before proposal
suppression. Missing known-function graphs and unavailable functions are reported
separately. The parent unit retains the full observation/unavailable roster and
its expected population.

`input_complete` describes graph input availability, not exhaustive discovery or
complete Rust semantics. These are approximate advisory graphs: closures can be
opaque, macro lowering is approximated, and core discovery uses its existing
bounded blocking and heuristic policy. A proposal is a review hint, not evidence
of equivalence or a safe rewrite. Findings may coexist with missing inputs;
incomplete coverage still exits 2. Check/test contexts remain separate, saved
graphs replay without source files or a compiler, and strict matching does not
consume these graphs.

Use `--review-file PATH` with compiler patterns to look up existing choices. Lookup
does not create or modify the file. To record a reviewed finding, replay its exact
saved run and select the numeric unit and candidate ID from that report:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --compiler-advisory --compiler-patterns --replay-run PATH_TO_RUN \
  --review-file reviews.json --dismiss-compiler-pattern 0 CANDIDATE_ID \
  --review-reason 'Why these sites should remain separate'
```

Only dismiss proposals after reviewing their sites, definitions and
counter-evidence. The candidate ID is a selection handle; an ambiguous ID in a
unit rejects the write. Persisted identity uses full observations, not that ID,
the legacy shape fingerprint or a new hash. Schema-8 stores retain earlier
review families and use the same single atomic publication. Reads and writes
are bounded by the existing baseline artifact convention of 4 MiB; oversized or
invalid stores and failed selectors leave the existing file untouched.

The core proposer returns the prepared facts it actually used. With an explicit
review file, only referenced site/definition owners are converted. Each site
graph moves into one immutable shared owner, so overlapping proposals and the
saved choice share graph buffers. Without a review file, this conversion does
no work. Stable locators reuse core owner/trait names and the existing package,
source and build-context adapter. Ambiguous locators and unresolved impl-number
names remain unavailable. Signatures come from captured compiler observations;
no source parsing, body proof or checksum is added.

Exact snapshots retain proposal kind, actual clustering policy, ordered role
rosters, full site graphs and definition signatures. Source lines are erased and
known local callee IDs use stable locators, so function movement, parameter
renaming and unrelated impl-block renumbering preserve reviews. Relevant graph,
signature, membership, kind, policy or build changes make them visible again.
Definitions are declaration observations: changing an opaque callee body alone
does not invalidate the review. A site graph with opaque macro/closure nodes also
retains its original function text, so hidden body edits cannot disappear behind
an unchanged approximate graph. This conservative case may stale on harmless
text edits. Graph equality is an advisory review contract, not semantic
equivalence or rewrite permission.

Raw `patterns.evaluation` remains intact. `patterns.visible_candidates` lists
the visible zero-based raw candidate indices when reviews are requested, and
`compiler_advisory.pattern_review` reports each decision. Finding exits and text
use that visibility. Missing input or unavailable positive review identity keeps
the result incomplete; hiding findings cannot turn incomplete analysis clean.
Structural proposal review coverage uses captured function/graph availability,
independently of numeric diagnostics, complexity rankings and unresolved effect
callees. `patterns.effect_evidence.input_complete` still records effect gaps.
Capture/handoff failures and unavailable functions/graphs keep proposal review
coverage incomplete. Overall advisory completion and exit codes continue to
include every requested advisory consumer, so complete structural review coverage
can coexist with overall exit 2. Proposal baseline coverage uses the same
structural input boundary; publication still requires all requested consumers
to complete. Pattern-only export does not request numeric diagnostics.
Check/test contexts remain separate. Retained status reuses completed lookups
instead of comparing the graphs again. `--review-status` accounts for every
stored entry, including omitted families. Missing proposals remain unavailable:
bounded heuristic discovery cannot establish a completed negative, so compiler
proposal entries are never automatically pruned. Other review families retain
their existing independently completed negative pruning rules.

Cross-package propagation and broader project proposal quality remain follow-up
work. No legacy
capability is retired by this increment.


## Saved compiler diagnostic reviews

Pass `--review-file PATH` with compiler advisory for read-only lookup. Raw
`units[].evaluation` diagnostics, summaries and fix groups remain intact.
`units[].visible_diagnostics` lists the visible zero-based diagnostic indices;
`compiler_advisory.diagnostic_review` contains the decisions. Text includes unit
and diagnostic indices, and `--fail-on-findings` counts visible diagnostics.
Fix plans still describe the full raw findings; a saved review does not remove
caller edits or prove independence.

After reviewing the finding, record an explicit saved-run choice:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory \
  --replay-run PATH_TO_RUN --compiler-advisory-config policy.toml \
  --review-file reviews.json --dismiss-compiler-diagnostic 0 DIAGNOSTIC_INDEX \
  --review-reason 'Why this finding is acceptable'
```

The unit and index are selection handles from that exact report, not durable
identity. Core retains the diagnostic-to-owner association while sorting; the
adapter does not infer owners from matching locations. Without a review file or explicit compiler baseline request, core retains no extra
owner roster and the adapter builds no diagnostic snapshots.
Known compiler owner/trait locators and captured signature/build context reuse
the same helpers as proposal reviews. Ambiguous names remain unavailable.

Schema 9 adds `compiler_diagnostic_entries` while preserving earlier families and
the shared atomic 4 MiB artifact boundary. All six style rules are supported:
length, parameters, spacing, mutations per target, mutated targets and effect
density. Invalid-suppression diagnostics cannot be dismissed. Snapshots retain
metric and detector versions, the full effective policy, captured signatures,
actual/limit values and ordered label/count details. Unrelated edits to the full
policy may conservatively stale a choice. Source lines and diagnostic
messages are presentation only. Movement and parameter renaming preserve choices
when these observations remain equal; signature, policy, context, count or
distribution changes make findings visible again. Mutation target renaming may
conservatively stale label-based details. Equality of numeric observations is a
review contract, not body equivalence or purity.

Diagnostics from the same function share one immutable owner/signature; snapshots
and saved choices share that owner and their policy value. No graph, body, source
parse, new checksum or hash is involved. Explicit status and diagnostic baselines
retain the core evaluator's completed suppression outcomes and move excluded
observations into the owner inventory. Ordinary lookup and default analysis do
not retain these extra scopes or parse suppression comments again.

A known selected owner evaluated under the same full policy and metric/detector
versions can establish `no_finding`. Effect-density negatives additionally require
complete propagation inputs; the five direct rules can complete despite unrelated
unavailable owners. Suppressed, invalid-suppression and excluded owners remain
unavailable. Unselected owners are not-requested; missing owners are not-observed
only in a complete identity scope, otherwise unavailable. Policy changes stay
stale and build contexts remain distinct. `--review-prune` requires an explicit
saved run and removes only completed negative choices; add
`--review-prune-dry-run` to preview without writing. Status alone never writes.
Compiler proposals remain positive-only. Hiding every known finding cannot make
generated/missing/unresolved input complete, and strict matching stays independent.


## Compiler diagnostic CI baselines

Export an inventory after a complete compiler advisory run, then compare explicitly:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-advisory-config policy.toml --write-compiler-baseline baseline.json
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-advisory-config policy.toml --compiler-baseline baseline.json \
  --fail-on-new-compiler
```

Both commands also accept an explicit `--replay-run PATH_TO_RUN`; comparison and
export need neither source files nor compiler tools on replay. The schema-1
inventory reuses completed diagnostic snapshots, metric/detector versions, full
effective policy and canonical build contexts. Snapshot comparison borrows values;
only explicit export copies compact measurements. One immutable policy value is
shared across all current units, owners and baseline publication. No graph, body,
source parsing, hashing or repeated analysis is added.

`compiler_advisory.diagnostic_baseline` reports current diagnostic handles,
existing/new visible counts, header drift and retained states. The unit/index
handles refer only to the current raw diagnostic roster and are never persisted.
All six numeric style rules use the same exact equality as saved diagnostic
reviews. Movement can preserve equality; changes to signatures, measurements,
distributions, policy or context make findings new/changed. Even an empty
inventory checks policy and build scope. The entire effective policy participates,
so an unrelated policy edit may conservatively cause drift.

`--fail-on-new-compiler` exits 1 for new/changed visible findings or policy/context
drift. Existing raw findings still count with `--fail-on-findings`; a baseline
never dismisses them. An explicit review file can hide individual new findings
from the new-finding count, while exports always retain the raw numeric findings.
Reviews cannot hide header drift. Retained numeric findings become `resolved`
only through the same completed owner/rule scopes used by review status;
suppressed, excluded, missing and unavailable inputs cannot establish resolution.

Incomplete inputs, unavailable diagnostic identities, invalid suppressions and
oversized populations keep exit 2 and prevent export. The shared artifact boundary
uses a 4 MiB limit, 4,096 entries/contexts, and atomic create-new publication;
existing destination files are never overwritten. Compiler baseline assessment
also requires at most 4,096 captured owners. Import validates the closed schema,
versions, contexts and unambiguous rule/owner/compilation-target identities.
A compilation unit with no captured functions lacks the function-backed build
context required by this adapter and cannot establish a baseline; no empty
context is invented. Generated test-harness owners may make `--all-targets` incomplete even when all
known diagnostics are available. Known findings still appear in an incomplete
comparison, but that comparison cannot establish a clean CI result.

Baseline comparison applies changed-file selection to new visible numeric
findings; policy/build-context drift still covers the whole scope. Exporting a
compiler baseline with a changed-file manifest is rejected. Broader project
coverage and legacy retirement remain open.


## Changed-file compiler diagnostics and proposals

Add `--changed-files PATH` with the shared manifest format:
`{"schema":"rstyle.changed-files/1","paths":["src/lib.rs"]}`. Exact
workspace-relative paths select numeric diagnostics after explicit saved reviews.
Text and `--fail-on-findings` use `units[].selected_diagnostics`, zero-based
indices into the unchanged raw diagnostic roster. Raw evaluations, totals and
caller/fix plans remain intact. Compiler proposals stay global by default;
invalid-suppression diagnostics always remain global. No source-rule analysis is
implicitly enabled.

`compiler_advisory.changed_scope` reports per-file owner availability and selected
finding counts using the shared source/density coverage aggregator. Unknown files
stay unobserved/incomplete; empty selection selects no numeric diagnostics.
Generated/missing owners and unresolved callees still keep whole-analysis exit 2,
even when every selected file has available observations. Check/test occurrences
remain distinct. Selection is file-level presentation, not changed-line analysis
or selective compiler execution.

Add `--select-compiler-patterns` with `--compiler-patterns --changed-files PATH`
to select proposals whose usage sites or definitions touch any selected file.
Each unit records `patterns.selected_candidates` as zero-based indices into its
raw candidate roster, independent of `visible_candidates` from explicit reviews.
Text, `--fail-on-findings`, and new-proposal baseline counts use the intersection
of these masks. Raw candidates, totals, classifications, reviews, plans and strict
matching retain their full scope. The mask is computed once from completed
candidate locations and reused during replay; no source scan or graph copy is
needed. Empty selection hides proposals but cannot hide missing inputs or
baseline header drift. `changed_scope` counts displayed numeric diagnostics and
visible selected proposals for each requested consumer. Pattern-only mode reuses
the same owner-coverage report; each proposal counts once at its first matching
usage or definition path. Unknown selected files stay incomplete.

Live analysis and source/tool-free replay use the same captured paths and owner
IDs. Selection retains only diagnostic indices, scoped owner IDs and owned report
path strings. It creates no diagnostic review snapshots absent an explicit
review/baseline request, and adds no source parsing, graph/body copying, hashing
or repeated proofs.


## Compiler proposal CI baselines

With compiler patterns, export a complete raw proposal inventory, then compare:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-patterns --write-compiler-pattern-baseline proposals.json
cargo rstyle clones --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-patterns --compiler-pattern-baseline proposals.json \
  --fail-on-new-compiler-patterns
```

Both commands support explicit saved-run replay without source files or compiler
tools. `compiler_advisory.pattern_baseline` uses the shared baseline report:
current raw unit/candidate indices, existing/new visible counts, header drift
and retained positive states. All existing proposal families reuse completed
review snapshots and exact equality, including ordered site/definition roles,
signatures, graphs, opaque source when required, metric/detector versions and
build contexts. Membership keys contain borrowed owner locators; no new hash,
source proof, graph copy or second mining pass is added. Explicit export shares
immutable graph owners with snapshots until serialization. Normal runs without
review/baseline requests still avoid snapshot preparation.

Movement and parameter renaming can preserve equality. Relevant graph/signature
changes make proposals changed; kind or membership changes identify a new
proposal. The full effective compiler policy, actual default clustering policy
and canonical compilation contexts participate in header comparison, including
empty inventories. Thus unrelated compiler-policy edits may conservatively fail
the new-proposal gate. Candidate IDs are selection handles and are not persisted
as identity. A baseline establishes neither equivalence nor rewrite safety.

`--fail-on-new-compiler-patterns` exits 1 for new/changed visible proposals or
header drift. Explicit saved reviews can hide new proposals from that count;
raw evaluations and exports retain them. Existing proposals still count under
`--fail-on-findings`. Proposal comparison remains global with a changed-file
manifest by default. With `--select-compiler-patterns`, new/changed visible
proposal counts also require a matching usage or definition file; raw lifecycle
classifications, retained choices and header drift remain global. Export
conflicts with selection.
Missing old proposals remain unavailable, never resolved or pruned: bounded
heuristic discovery does not establish completed negatives.

Missing/generated owners, uncaptured graphs, unavailable identities or incomplete
capture keep exit 2 and prevent export. Unresolved effect callees additionally
prevent export when numeric advisory is requested; pattern-only baselines retain
the separate effect-input limitation. The existing
4 MiB artifact and 4,096 owner/entry/context bounds apply. Imports validate closed
schemas, graph observations, versions, membership keys and build contexts; export
creates a new file without overwriting an existing one. Broader project quality,
coverage and legacy retirement remain open.
