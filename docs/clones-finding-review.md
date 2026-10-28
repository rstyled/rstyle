# Abstraction finding reviews

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

Only explicit dismissal writes the file, using a temporary file and replacement.
A nonempty reason and `--replay-run` are required. Invalid selectors, unavailable
snapshots, malformed stores and repeated exact dismissals fail without writing.
`--review-allow-growth` records an explicit choice for future membership growth;
retaining a larger group requires fresh independent evidence for every member
pair and unchanged original snapshots/contexts/versions. The current helper
consumer produces two-member proposals only, so connected pairs never create
reviewed groups. Exact membership is the default; member loss becomes stale.

This bounded review capability does not migrate legacy suppressions or dismissals,
provide automatic rewrites, or satisfy the remaining legacy retirement gates.

Lookup assesses current helper findings. Stored entries with no current helper
finding remain retained; this release does not provide a separate whole-store
status/prune command or infer deletion from an unmatched review. The pure
membership contract tests stale member loss and explicit all-pair growth; live
helper groups and whole-store changed-scope presentation remain later work.

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
source or graph authority. Whole-store status/pruning, legacy ignore migration and
broader review families remain open. Source reviews are bounded to 4,096 retained
entries and 256 KiB per canonical declaration; exceeding a bound fails explicitly.
