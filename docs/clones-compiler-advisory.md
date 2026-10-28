# Compiler advisory reuse

`clones --extraction-profile hir-data-v3 --compiler-advisory` captures the existing
Rust style observations during the same compiler invocation. It calls the driver
mutation collector (`ExprUseVisitor`) and effect/call/parameter-use collector,
then passes their `FnFacts` to `rstyle_core::evaluate`. That evaluator supplies
rule policy, bounded call-graph propagation, fix groups and parameter stacking.
There is no second implementation of those algorithms and no strict clone
admission requirement for these observations.

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
Changed-file filters and baselines do not yet apply to this field. Compiler
proposals have a separate review identity described below.
The normal legacy style command remains available.

Both the record and metric identities are versioned:
`rstyle.compiler-advisory/1` and `rstyle.compiler-style-metrics/1`. These are
compiler style metrics, not the newer source/native mutation-effect contract.
Mutable borrows count as mutation events; compiler statement counts include HIR
lowering; effect propagation uses the existing bounded depth, boundaries and
trait-target policy. Unclassified external paths are governed by the reported
core configuration. A policy density of zero does not establish effect absence,
purity, termination, or safe refactoring.

Captured observations replay through `--replay-run`, including changed core
policy, without Cargo, rustc or original source files. Runs captured without the
flag retain unavailable observations. Expansion-generated functions unavailable
to the original collector remain unavailable. Missing facts never become empty
function records; the adapter rejects omitted fields that older `FnFacts`
deserialization would fill with zero/empty defaults. The enclosing bundle and
selected compiler invocation retain the compilation context. No new hash is used.
Strict clone matching and certification do not consume this advisory record.

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
the baseline workflows do not yet apply to them. Core candidate IDs retain their existing fingerprint convention and
must not be treated as the newer compilation-context review identities.

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

Cross-package propagation, broader project proposal quality and baseline
migration remain follow-up work. No legacy
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
adapter does not infer owners from matching locations. Without a review file,
core retains no extra owner roster and the adapter builds no review snapshots.
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
parse, new checksum or hash is involved. Status reuses completed lookups through
the same positive-review workflow as compiler proposals. A missing diagnostic is
retained as unavailable: this adapter does not yet establish completed negative
assessments for pruning. Omitted compiler advisory is not-requested. Hiding every
known finding cannot make generated/missing/unresolved input complete, and all
strict matching results remain independent.
