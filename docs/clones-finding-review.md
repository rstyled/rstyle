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

Identity initially covers uniquely indexed original free-function declarations
in captured root source and inline modules. External module files, ambiguous
declarations, unavailable source ownership/indexes and exhausted bounded work
remain unavailable when a positive review identity is needed. Observation and
retained-entry work are each bounded to 4,096 entries.

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
4,096-owner bound and root/inline-module free-function coverage described above.
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
are never created. Schemas 1–4 are not migrated or rewritten by lookup or status.
Request `--source-rules` and the relevant `--analysis-config` to assess parameter
entries.

JSON adds `review_status` (`rstyle.review-status/1`) only when requested. Each
`records` row retains its family, zero-based index within `entries` or
`source_entries` or `parameter_entries` or `length_entries`, original reason, durable member locators,
state and explanation.
These indices locate rows in this file; they are not persistent finding IDs.

| State | Evidence and meaning |
| --- | --- |
| `hidden_exact` / `hidden_growth` | A completed current finding is hidden by this exact retained entry or its explicit helper growth policy. |
| `stale` | A completed current finding overlaps the retained identity but differs in versions, membership, context or relevant body/roles/signature/policy. A completed negative under changed parameter or length policy is also stale. |
| `context_changed` | Every retained endpoint is observed, with at least one present only in a different context; no incomplete identity observations can conceal the original context. |
| `no_finding` | Endpoints resolve uniquely in their retained context and the relevant function/pair has a completed negative assessment under matching detector/identity versions. Parameter negatives also require the retained `max_params` and `count_self`; length negatives require the retained `max_lines`. Existing-trait and source-rule suppression can be completed negatives. |
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
