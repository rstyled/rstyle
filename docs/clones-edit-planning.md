# Advisory helper edit planning

Request `cargo rstyle clones --extraction-profile hir-data-v3 --locked
--helper-proposals --edit-plans` to coordinate existing body edits for the explicit
intent of introducing a shared helper while preserving each original signature
with a wrapper body. This selects an advisory intent; it does not apply edits.

The `edit_planning` report (`rstyle.edit-planning/1`) consumes actual completed
helper findings. It does not infer edits from accepted-pair similarity or a
parameter-count diagnostic. Each plan retains its two member handles and indices
into `regions`. Regions retain the exact original half-open file byte range,
workspace-relative physical source path and package-manifest provenance. Function
handles join the report's original compilation-context evidence and remain local
navigation handles, not persistent review identities.

Relations carry two plan indices. `overlap_witness` supplies original intersection
spans and region ordinals **within each plan's `required_regions` list**. Following
those indices resolves the corresponding entry in the report's `regions` table.
Connected overlap groups are review coordination groups. They do not combine pair
proposals into an all-member abstraction, prove that the changes can be stacked,
or choose an order of application.

`disjoint_known_regions` only establishes that the witnessed existing body edits
are disjoint. Helper declaration insertion, name and scope remain unknown for every
plan. Whole-plan independence stays unknown, and stack order is not proved.
Neither this report nor a completed helper finding establishes rewrite safety.

A physical source path identifies the editable resource within the captured
workspace. Package manifests and lib/test/features contexts remain separate
provenance. The same source body compiled in two contexts still overlaps. Exact
captured file bytes must agree before comparing coordinates. Conflicting snapshots
make the affected plans explicitly incomplete; path/checksum equality cannot
replace byte equality. Original source files are never reread or canonicalized.
Unambiguous workspace-relative ownership is required; source outside that root
remains unavailable.

Source ownership is checked before bounded parsing. Body views retain immutable
owners through `Arc`, and endpoint resolution is reused across helper pairs.
Mutation and planning consumers share conservative lexical parsing bounds.
Planning limits are 4,096 plans, 65,536 pair and region comparisons, and 16 MiB of
nontrivial snapshot-byte comparisons. Budget exhaustion preserves already checked
witnesses and reports incomplete execution. Missing/unsupported source evidence
and unavailable helper populations remain explicit; an empty incomplete report is
not a clean result. No snapshots or source bytes are duplicated into this report.

Strict `--replay-run` can request planning from saved evidence without source files,
Cargo or the driver. Full and compact presentation use the same consumer.
Incomplete planning exits two; existing helper finding/exit policy remains in
force. Legacy review dismissals are not treated as chosen edit intents, and this
initial consumer coordinates all actual helper findings, including reviewed ones.

Local guard intents and concrete caller-aware parameter plans remain pending.
Parameter plans require selected parameter positions, checked call/argument and
function-value-use inventories, and explicit public/external caller limits.
Legacy grouping and stacking remain operational.
