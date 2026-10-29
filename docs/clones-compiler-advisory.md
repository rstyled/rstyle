# Compiler advisory reuse

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
handoff and requested advisory consumers. Missing owners or unresolved callees
still produce an incomplete result and exit 2.

The mode publishes the existing shared raw-facts format with an exact
`compiler-advisory-only` phase, using the existing streaming transport seal and
successful Cargo/unit ledgers. Raw function references identify
`artifact_kind: advisory-raw-facts` and carry no strict namespace. They cannot be
imported as strict evidence. Replay requires the original capture mode and can
run without source files or compiler tools. Selected dependencies and separate
check/test contexts keep their original Cargo scope. Compiler reviews, changed
files, policy reevaluation and baselines use the ordinary advisory workflow.

Pair and native source/effect consumers conflict with this mode; use the ordinary
combined command to request those analyses. Raw extraction keeps the original
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
`rstyle.compiler-advisory/1` and `rstyle.compiler-style-metrics/6`. These are
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

Each unit's optional `complexity_ranking` uses schema
`rstyle.compiler-complexity-ranking/1`. Parallel `owners` and `functions` arrays
pair report-local handles with score/location rows. `known_functions`,
`measured_functions`, `unavailable_measurements` and `unavailable_owners` describe
the full input, independently of `requested_limit`. Missing measurements sort
last and retain an unavailable score; missing owners remain in the unit's
`unavailable` list. Either makes the requested ranking incomplete, including when
replaying historical metrics without scores, and prevents a successful exit.
The limit truncates presentation only. The flag works on fresh captures and
source/tool-free replay, including advisory-only mode. Without it, the existing
report is unchanged. Rankings over incomplete input are not whole-project maxima.
The offline [complexity validator](../eval/clones-capability/README.md) accepts
`--require-scores` to compare these completed rows with independent numeric
labels, including clean functions. Missing/truncated rows remain unavailable;
ordinary threshold-outcome comparisons stay separate.

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
graphs and run the existing core abstraction proposer. This flag requires
`--compiler-advisory`; graph construction does no work without the explicit flag.
It reuses `pdg::pdg_of`, signature mining, trait-call preparation, clustering,
alignment, proposal ranking and existing-trait/abstraction suppression policy.
No new consumer algorithm, body proof, hash or strict clone permission is added.
Collected graphs move between consumers rather than being copied during input
preparation. The style evaluator retains its policy-selected facts for the
proposer; the explicit core configuration's path exclusions apply to both.
Default clustering also borrows these graphs: with no normalization rules it
does not copy their nodes, edges or metadata. Explicit normalization still uses
an owned rewrite, preserving the original observations.

Each unit gains a `patterns` result with schema `rstyle.compiler-patterns/1`,
the actual default clustering parameters and the existing core report, including
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
baseline header drift. `changed_scope` counts numeric diagnostics only.

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

Missing/generated owners, uncaptured graphs, unresolved callees, unavailable
identities or incomplete capture keep exit 2 and prevent export. The existing
4 MiB artifact and 4,096 owner/entry/context bounds apply. Imports validate closed
schemas, graph observations, versions, membership keys and build contexts; export
creates a new file without overwriting an existing one. Broader project quality,
coverage and legacy retirement remain open.
