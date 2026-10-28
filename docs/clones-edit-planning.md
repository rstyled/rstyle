# Advisory edit planning

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

## Parameter signature intents

Request `--source-rules --parameter-plans` with the usual `--analysis-config`
source policy. The optional `parameter_planning` report
(`rstyle.parameter-planning/1`) contains one advisory signature intent per visible
`fn_params` finding. Named suppression and workspace path exclusion prevent an
intent for that finding. A completed clean parameter population has no intents;
unavailable or budget-limited population cannot become a clean empty result.
The source-rule report is evaluated once and supplies these findings directly.

Each plan contains the exact original signature region and checked same-unit
caller expression regions. Signatures start at visibility or the first signature
token and end immediately before the opening body brace, excluding outer
attributes. Regions share immutable source ownership; no original files are
reread. `arguments` retain absolute original spans and checked formal-root
ordinals where available. `formal: known(null)` denotes a checked literal/local
argument. This is source-root provenance, not proof that a formal is unchanged,
only forwarded, removable or suitable for a bundle.

`caller_scope` is captured selected units and `call_resolution` is
`same_unit_as_target`. Each defining unit has a separate `caller_census` with
expected/scanned owner counts, call/argument counts, execution, coverage and
reasons. Exact definition identities, native paths, source call/argument order
and selected-unit contexts must agree. A single bounded scan builds the target
index; target queries borrow completed observations rather than rescanning or
proving callee effects. Signatures and call regions for each owner are resolved
once within this consumer. The scan initially uses the existing nongeneric
scalar call vocabulary; unsupported signatures and result types remain incomplete.

Opaque macros, closures/control flow, indirect/associated/external calls,
unexamined nested arguments and unavailable owners prevent a complete caller
census. Supported call witnesses can coexist with incomplete coverage; missing
witnesses never prove absence of callers. Functions from excluded source paths
still participate in caller scanning because their calls may need editing.
The scope does not prove all Rust callers, function-value uses or external API
consumers have been captured. Public functions can have a complete declared
same-unit census while external coverage remains unknown.

`coordination` reuses the checked overlap consumer over each signature followed
by its caller regions. Its region ordinals refer to that order, and relations
coordinate witnessed physical edits only. Disjoint call expressions in a shared
caller do not manufacture an overlap. Whole-plan independence and stack order
remain unknown; the replacement signature/argument design, imports, names,
visibility and uncaptured callers are unknown edits. No rewrite is performed.
Limits are 4,096 scanned owners per unit/population, 16,384 calls and 65,536
arguments per unit, plus the existing plan/overlap limits. The
`--max-analysis-functions` allowance also bounds caller scans across relevant
units. Exhaustion preserves checked witnesses and exits two. Full/compact
strict replay uses the same captured source and caller observations without
source files, Cargo or a driver.

Local guard intents, forwarding bundles, pass-through/all-use inventories,
external caller closure and broader signatures remain pending. Legacy grouping
and stacking remain operational.
