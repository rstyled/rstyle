# Finding reviews

`clones --helper-proposals --review-file reviews.json` applies an explicit,
versioned JSON review store. A missing file means no previous reviews; lookup
does not create or modify it. The legacy ignore file remains separate.

Dismiss one pair from an exact saved run after reviewing its source locations,
ordered differences and helper evidence:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --helper-proposals \
  --review-file reviews.json \
  --dismiss-helper-pair function:u0:f0 function:u0:f1 \
  --review-reason 'Reviewed this repeated computation; keep these separate.'
```

The endpoint handles select findings within that saved run. Persistent identity
uses the full workspace-relative Cargo manifest, package name/version,
package-relative source path, qualified free-function name, compilation target,
effective compiler/build settings, matching thresholds and scoring, extraction
and consumer policy versions, signature and ordered scalar expression. No hash,
HIR ordinal, source byte position, source checksum or run directory is used as a
persistent key. Moving lines or renaming formal binders preserves review; relevant
body, literal spelling, signature, build-context or policy edits require review.
Distinct qualified owners and contexts cannot inherit each other's dismissal.

Snapshots are currently available for uniquely owned original free functions
(including inline modules) within the captured workspace, using the bounded u32
helper profile. Methods, nested functions, generated functions, ambiguous
conditional declarations and external package paths remain unreviewable with
explicit reasons. Complete helper analysis can coexist with incomplete review.

JSON `review` records execution, coverage, original finding count, hidden/visible
counts, retained entry count and per-pair status: `visible_new`, `hidden_exact`,
`hidden_growth`, `stale` or `unavailable`. Stale entries and their reasons are
retained. Missing members or incomplete scope never prune stored decisions.
`helper_proposals.findings` always retains the unfiltered count; when reviews are
requested, `--fail-on-findings` uses the visible helper count. Other requested
analyses and strict pair decisions keep their own exit conditions. Incomplete
helper/review coverage still exits two, including when every known finding is
hidden. Without `--review-file`, reviews are not requested and finding exits
remain unchanged. Text output includes both original proposals and review status.

Explicit dismissal and pruning write the file, using a temporary file and replacement.
A nonempty reason and `--replay-run` are required. Invalid selectors, unavailable
snapshots, malformed stores and repeated exact dismissals fail without writing.
`--review-allow-growth` records an explicit choice for future membership growth;
retaining a larger group requires fresh independent evidence for every member
pair and unchanged original snapshots/contexts/versions. The current helper
consumer produces two-member proposals only, so connected pairs never create
reviewed groups. Exact membership is the default; member loss becomes stale.

This bounded review capability does not migrate legacy suppressions or dismissals,
provide automatic rewrites, or satisfy the remaining legacy retirement gates.

Ordinary lookup assesses current helper findings. Stored entries without a
current finding remain retained. Explicit whole-store status is described below;
it does not infer deletion or prune entries. The pure membership contract tests
stale member loss and explicit all-pair growth; live helper groups remain later
work.

## Dispatch and ordered capability findings

The same `--review-file` also applies to requested `--enum-dispatch` and
`--capability-proposals` analyses. Select a completed finding from its saved run:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --enum-dispatch \
  --review-file reviews.json --dismiss-enum-dispatch function:u0:f0 \
  --review-reason 'Reviewed the dispatch; retain the explicit match.'

cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --capability-proposals \
  --review-file reviews.json \
  --dismiss-capability-pair function:u0:f0 function:u0:f1 \
  --review-reason 'Reviewed the two interfaces; keep them separate.'
```

Only one dismissal selector is accepted per invocation. Dispatch reviews cover
one original caller, its enumeration and ordered variant-to-method selections.
Capability reviews cover exactly two original wrappers and their two ordered
method roles. Group growth is available only for helper reviews. Existing trait
suppression is a separate detector decision; it cannot be selected for dismissal.

Durable locators use original qualified declarations and workspace/package-relative
paths. Context contains the exact compiler, build, target and extraction settings.
Matching thresholds and graph admission do not define these local proposals and
are excluded from their identities. Canonical source tokens and explicit ordered
roles preserve comments, line moves and normalized local binding renames. Selected
method headers and nominal declarations are relevant; opaque method body edits
preserve review. Changed role order, selected declarations, signatures, target or
policy become visible for review. No report handles, byte positions, source hashes
or full graph snapshots are persisted as identity.

JSON `abstraction_review` retains original/hidden/visible counts, separate visible
dispatch/capability counts and `decisions` with family, saved-run endpoints, state
and reason. The original consumer reports remain unfiltered. Requested finding
exits use the visible counts; unavailable scope still exits two even when every
known finding is hidden. Review lookup never writes or prunes. Invalid selection,
unavailable identity, duplicate dismissal or malformed store fails without writing.

Helper-only stores keep their schema-1 shape. Adding a dispatch or capability
review upgrades the envelope to schema 2 with typed `source_entries`; existing
helper entries are retained unchanged. Schema-1 files remain readable. Neither
family can hide a different family, and serialized review data grants no current
source or graph authority. Parameter reviews use schema 3 as described below.
Legacy ignore migration remains open. Source reviews are bounded to 4,096 retained
entries and 256 KiB per canonical declaration; exceeding a bound fails explicitly.

## Parameter-count findings

Add `--source-rules --analysis-config analysis.toml --review-file reviews.json`
to assess parameter reviews. Dismiss one completed, visible, selected `fn_params`
finding from an exact saved run:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --source-rules \
  --analysis-config analysis.toml --review-file reviews.json \
  --dismiss-parameter function:u0:f0 \
  --review-reason 'Keep this public parameter boundary.'
```

The selector is local to the saved run. Durable identity retains detector versions,
the original package and qualified declaration locator, exact compilation/build
context, numeric policy (`max_params`, `count_self`) and measured parameter count
and receiver presence. Parameter types and return types are outside this numeric
rule's review identity.
Body edits, binder renaming and line/layout changes preserve review. Changed
arity, numeric policy or detector versions require review; changed build contexts
cannot inherit it. Lib and test compilation owners remain distinct. No body
source, source hash/checksum, byte position, report handle or run directory is
persisted as identity.

Parameter dismissal upgrades the store envelope to schema 3 with typed
`parameter_entries`, preserving helper `entries` and dispatch/capability
`source_entries`. Schema-1 and schema-2 files remain readable without lookup
rewrites. Missing stores mean no reviews. Suppressed, clean, unselected,
unavailable, invalid or duplicate selections reject without writing.

Identity covers uniquely indexed original declarations in captured root source,
inline modules and independently checked ordinary external modules (`name.rs` or
`name/mod.rs`). External locators support free functions and methods on unique
same-module structs/enums, including type/const-generic inherent and local-trait
implementations. Generic owner/trait arguments retain canonical original tokens;
`Owner<u8>` and `Owner<u16>`, or `Trait<u8>` and `Trait<u16>`, cannot share a review.
Bounds and where clauses on these declarations do not require dispatch proofs.
Lifetime-only owners retain their existing external nominal locator spelling and
arity rules; mixed generic paths retain their lifetime tokens as written.
Inherent methods use `crate::module::Owner::method`; local-trait methods use
`crate::module::<Owner as Trait>::method`, with declaration kind `method` and the
applicable generic arguments. Member cfgs use the captured compiler selection
before checking duplicates. Original file/range/header/receiver correspondence
remains mandatory. These source identities grant no dispatch, effect, strict
clone or refactoring permission. Existing complete root names retain the old
source-index route; generic/lifetime root names that were incomplete use the same
cached declaration index as external methods. Old malformed names such as
`crate::::method` remain unobserved and cannot hide findings or be pruned as
numeric negatives.

For these original impl methods, prelude, imported and qualified trait origins
reuse the complete captured compiler function name as an opaque locator when no
local source trait name is available. This needs no numeric advisory input,
import resolution or dispatch proof. Empty or unstable impl-index names remain
unavailable; the existing review roster rejects duplicate final identities.
Actual source-local locators stay unchanged. Root names that previously mistook
a nonlocal trait for a local declaration are corrected; retained old names remain
unobserved and cannot hide or prune current findings.

Aliased/imported/qualified owners, lifetime-generic-only local traits,
default trait methods, remapped/generated ownership, ambiguity and exhausted
bounded work remain unavailable. Observation and retained-entry work are each bounded to 4,096 entries.

JSON `parameter_review` (`rstyle.parameter-review/1`) reports execution, coverage,
original/visible/hidden counts, retained entries, `hidden_selected_findings` and
per-function decisions (`visible_new`, `hidden_exact`, `stale`, `unavailable`).
Text hides exact reviewed parameter diagnostics. `--fail-on-findings` subtracts
only selected hidden parameter findings; other source rules keep their own gates.
Raw `analysis.findings`, `analysis.visible_findings`, assessments and parameter
planning populations remain unchanged. Unselected decisions remain reported.
Incomplete scope keeps exit-two precedence. Exact replay requires neither
original source files nor compiler tools.

## Function-length findings

`--source-rules --review-file reviews.json` also assesses exact numeric length
reviews. From an explicit saved run, use `--dismiss-length FUNCTION_KEY` with
`--review-reason` to retain one completed visible selected `fn_length` finding:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --source-rules \
  --analysis-config analysis.toml --review-file reviews.json \
  --dismiss-length function:u0:f0 \
  --review-reason 'Keep this measured function boundary.'
```

The durable choice contains detector versions, checked declaration locator and
build context, `policy.max_lines`, and `measurement.code_lines`. It reuses the
completed source-rule count. There is no body fingerprint, stored source, recount,
or new graph requirement. Blank/comment-only insertions and body edits preserving
the code-line count preserve review. A changed count, length
policy or detector version requires review; different build contexts cannot
inherit the choice. This numeric choice does not approve every future body edit.

Length dismissal adds `length_entries` in schema 4. Existing helper, abstraction
and parameter entries survive unchanged, and schemas 1–3 remain readable without
read-only upgrades. Parameter and length findings on one function are reviewed
independently. Their original owner is resolved once per function, under the same
4,096-owner bound and original free-function coverage described above.
Each numeric family permits at most 4,096 retained entries.
Unavailable, suppressed, unselected or clean findings cannot be dismissed.

JSON `length_review` (`rstyle.length-review/1`) reports the same counts, selected
hidden count and decision vocabulary as parameter reviews. Exact length reviews
hide only selected length text diagnostics and selected length finding exits.
Parameter, spacing and guard findings retain their gates; raw assessments/counts
and advisory plans stay unfiltered. Strict full/compact saved-run replay requires
neither source files nor compiler tools. Retained status uses family `length`.
Only same-policy, current-version completed negatives are eligible for explicit
pruning. Raising `max_lines` leaves the old choice stale and ineligible; omitted
owners and unavailable evidence remain retained.

## Function-spacing findings

With an explicit `[fn_spacing]` section, source-rule review also covers completed
visible spacing findings. Use `--dismiss-spacing FUNCTION_KEY` from a saved run
with `--source-rules`, `--analysis-config`, `--review-file` and a nonempty
`--review-reason`. Dismiss only when you intend to retain this measured boundary.

The choice stores checked locator/build context and detector versions,
`policy.min_statements` and `measurement.direct_statements`. The completed finding
establishes the missing blank line; the review adapter does not parse or scan
source again. Same-count edits that still produce a spacing finding preserve
review. Changed counts or policy require review. Inserting a blank line can
produce a completed negative under the retained policy, eligible for explicit
pruning. Disabling spacing or raising its threshold changes policy and leaves
the old entry stale and ineligible. Omitting `[fn_spacing]` leaves it
`not_requested`. Unsupported statement transformations remain unavailable.

Spacing dismissal adds `spacing_entries` in schema 5. Schemas 1–4 remain readable
without read-only rewrites. JSON `spacing_review` (`rstyle.spacing-review/1`) and
retained family `spacing` use the same counts and decision vocabulary. Parameter,
length and spacing choices remain independent; only selected matching diagnostics
and finding exits are hidden. Raw assessments, totals and plans remain unfiltered.
All three families share one original owner resolution and the existing 4,096-owner
bound, with 4,096 stored entries per family. Strict full/compact replay needs no
original source or compiler tools. The three families share the original owner
scope above, including checked ordinary external methods. Unavailable, suppressed,
unselected and clean findings cannot be dismissed.

## Inspect every retained review

Add `--review-status` to a normal analysis or an explicit saved-run replay:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run \
  --helper-proposals --enum-dispatch --capability-proposals \
  --review-file reviews.json --review-status
```

This read-only report includes **every entry in the selected file**, including
reviews with no current finding. It requires `--review-file` and conflicts with
dismissal selectors. Request each analysis family you want assessed; an omitted
family remains explicitly `not_requested`. Missing files mean an empty store and
are never created. Schemas 1–7 are not migrated or rewritten by lookup or status.
Request `--source-rules` and the relevant `--analysis-config` to assess source-rule
entries; spacing additionally requires an explicit `[fn_spacing]` section.

JSON adds `review_status` (`rstyle.review-status/1`) only when requested. Each
`records` row retains its family, zero-based index within `entries` or
`source_entries` or `parameter_entries`, `length_entries` or `spacing_entries`, `mutation_entries` or `density_entries`, original reason, durable member locators,
state and explanation.
These indices locate rows in this file; they are not persistent finding IDs.

| State | Evidence and meaning |
| --- | --- |
| `hidden_exact` / `hidden_growth` | A completed current finding is hidden by this exact retained entry or its explicit helper growth policy. |
| `stale` | A completed current finding overlaps the retained identity but differs in versions, membership, context or relevant body/roles/signature/policy. A completed negative under changed parameter, length or spacing policy is also stale. |
| `context_changed` | Every retained endpoint is observed, with at least one present only in a different context; no incomplete identity observations can conceal the original context. |
| `no_finding` | Endpoints resolve uniquely in their retained context and the relevant function/pair has a completed negative assessment under matching detector/identity versions. Parameter negatives also require the retained `max_params` and `count_self`; length negatives require the retained `max_lines`; spacing negatives require the retained `min_statements`. Existing-trait and source-rule suppression can be completed negatives. |
| `not_observed` | An endpoint has no checked identity in the selected capture. Omitted packages/targets, renamed or removed declarations are not distinguished; this is not proof of deletion. |
| `not_requested` | This entry's detector family was omitted. |
| `unavailable` | Identity, detector versions, admission, evidence, function/pair assessment or bounded work cannot establish status. |

Captured endpoints alone never prove a negative pair assessment. Helpers absent
from accepted strict pairs remain unavailable; capability pair budgets and
unsupported wrapper evidence remain unavailable. Dispatch requires an explicit
completed negative, not an unsupported or not-applicable result. Status reuses
completed proposal and detector decisions; it does not run another detector or
mint source/graph authority. Exact/stale entries do not require a second endpoint
index. Unresolved entries share lazy report-local identity indexes, bounded to
4,096 captured owners; exhausting that observation budget cannot establish an
absent endpoint or changed context. Identities use exact locators and context
values without additional hashes or graph/source clones.

`assessed_entries` counts exact, growth, stale, context-changed and completed
negative rows. Other rows leave status coverage incomplete and, when status is
explicitly requested, exit two. Ordinary invocations keep their existing exits.
Existing finding reports, review decisions and `--fail-on-findings` counts remain
unchanged; requesting status does not dismiss findings. Legacy ignore migration
remains separate future work.


## Prune completed negative reviews

Use an explicit saved run to preview removal:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run \
  --helper-proposals --enum-dispatch --capability-proposals \
  --review-file reviews.json --review-prune --review-prune-dry-run
```

Remove `--review-prune-dry-run` to apply. Pruning implies whole-store status and
requires `--review-file` and `--replay-run`; it conflicts with dismissal selectors.
Only entries assessed as `no_finding` are eligible. Stale, changed-context,
missing, unrequested and unavailable entries remain. Independently completed
negatives can be removed from mixed incomplete coverage; incomplete coverage
still exits two. Capture or handoff failures block pruning without writing.
For parameter entries, raising the limit to remove a finding yields `stale`,
not `no_finding`; changed policy or noncurrent detector versions never authorize
negative pruning. Request source rules with the retained numeric policy to assess
parameter entries.

JSON adds `review_prune` (`rstyle.review-prune/1`), with eligible, removed and
retained entry counts and selections by family and original entry index. Status
and finding reports describe the store before pruning. A dry run removes zero
entries and leaves bytes unchanged. Applying a nonempty selection uses one atomic
replacement, preserving schema version, surviving entries and their order.
No-op applies and dry runs never create or rewrite the file.


## Save numeric effect-density choices

Density reviews require `--effect-density`, an explicit `--replay-run`,
`--review-file`, `--dismiss-density FUNCTION_KEY`, and a nonempty `--review-reason`.
Use the same `--effect-config` as the finding being reviewed. Schema 7 stores these
choices in `density_entries`; earlier schemas remain readable. See
[mutation and effect analysis](clones-mutation-effects.md) for an invocation.

The retained snapshot identifies the original declaration/build context and the
complete numeric measurement: event-kind counts, source statement/code-line
counts, metric versions and the full density configuration. This is a numeric
policy choice. Body edits that preserve that complete measurement and context
can preserve the review; it does not approve behavior or establish equivalence.
Raw effect evidence and density measurements remain visible in JSON.

Status reports the `side_effect_density` family. A completed numeric negative
requires the retained policy and current detector versions before pruning can
remove an entry. Unknown, exempt, suppressed, excluded, unselected, unavailable,
and budget-limited observations cannot authorize density negative pruning.


## Guard opportunity reviews

`--guard-clauses --review-file reviews.json` supports exact reviews in ordinary
and advisory-only captures. Dismiss one completed opportunity from a saved run:

```sh
cargo rstyle clones --extraction-profile hir-data-v3 \
  --replay-run /absolute/path/to/run --guard-clauses \
  --review-file reviews.json --dismiss-guard function:u0:f0 0 \
  --review-reason 'Reviewed this guard; retain the existing shape.'
```

The second selector is the zero-based opportunity index: the function-tail
opportunity, when present, precedes `closure_opportunities` in report order.
Each sibling closure has its own review. Persistent identity reuses the original
qualified declaration and build context, the exact guard source, fallback branch
and continuing-statement count. Closure reviews also retain the exact enclosing
closure source and its opportunity ordinal. Moving an unchanged function preserves
review; changed guard/closure text becomes stale. Inserting or reordering closure
opportunities can conservatively invalidate ordinal-based reviews. No new parser,
source scan, hash or strict clone admission is required.

JSON `guard_review` reports opportunity counts and indexed decisions. Exact
reviews hide only the selected guard text and its contribution to finding exits.
An owner with another visible opportunity still counts once. Raw assessments,
changed-scope totals and edit plans remain available. Changed-file selection also
limits dismissal when `--select-guards` is requested. Completed closure advice can
be reviewed within an incomplete enclosing owner, but incomplete global coverage
still exits two even when all known opportunities are hidden.

The store uses schema 10 for guard entries and reads schemas 1–9 without rewriting
them during lookup. Whole-store status treats an absent opportunity as unavailable;
it does not establish a completed negative or permit pruning. Guard baselines and
completed-negative lifecycle remain open. Existing entry, source and file-size
bounds apply, and invalid selectors or incomplete execution cannot publish reviews.
