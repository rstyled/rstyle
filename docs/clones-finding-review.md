# Helper finding reviews

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
