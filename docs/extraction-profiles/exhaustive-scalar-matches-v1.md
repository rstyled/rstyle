# Exhaustive scalar matches without a wildcard

Data-v3 extraction 3.9.0 adds `rust.exhaustive-scalar-match`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. A bounded integer/bool match no longer requires
an unguarded final wildcard when the supported unguarded patterns independently
cover its entire resolved scalar domain. The existing 1–16-arm limit, supported
pattern grammar, guard restrictions and result/control restrictions still apply.

Coverage uses closed intervals in an ordered u128 coordinate domain. Boolean
values occupy two points; signed integers use an order-preserving offset that
retains the full i128 range. Native integer limits come from the captured target
pointer width. Literal bases/suffixes, negative values and inclusive/exclusive/open
ranges retain their existing source-backed spellings. Same-unit constant values
and named range bounds use their previously verified canonical values. OR arms
contribute the union of supported alternatives. Overlap never fills a gap, and a
guarded arm contributes no unconditional coverage, even for a constant guard.
Malformed/out-of-domain values cannot establish exhaustiveness.

The proof sorts compact intervals and checks for gaps; its storage depends on
pattern syntax, never interval cardinality. Existing final-wildcard matches use
a direct fast path without parsing values or sorting intervals, preserving their
previous graphs. No frontend fields, serialized coverage witnesses, source
snapshots, provenance tables or caches are added.

Lowering still evaluates the scrutinee once and retains source-order selectors,
guards, bodies, return paths and expression-value alternatives. When coverage
proves that an unguarded selector must accept every remaining value, that selector
has only its true continuation. Its original pattern and saved-scrutinee operand
remain represented. Subsequent source arms remain assessed and captured as
unreachable operations; unsupported suffixes do not become certified remainders.
A final wildcard remains an unconditional selector under its earlier contract.

Admission checks source/HIR pattern correspondence and the existing constant/range
contracts before accepting this coverage proof. Strict verification reconstructs
the expected graph from those facts; the independent serialized value-site checker
also rejects an invented failure path that leaves an expression value unavailable.
Projection retains the existing pattern literals/descriptors and control/data
edges, with no new matching vocabulary. Verified helper callers can compose with
supported exhaustive scalar bodies under the existing helper-flow limitations.

Earlier Cargo selections retain their final-wildcard boundary. This does not add
character/float domains, enum/payload/reference patterns, nonlocal/generic/trait
constants, arbitrary nested OR patterns, effectful guards or value merges in
loop-containing functions. F04/F06 remain Partial for those broader forms.

Validation includes an independently enumerated small-domain coverage oracle,
signed/unsigned 128-bit endpoints, actual native width, overlap/gaps and excluded
upper bounds. Real Cargo cases compose boolean and constant patterns, mixed OR and
ranges, guards, cfg-removed arms, early returns and helper calls. Negative checks
reject guarded-only coverage, holes, unsupported domains and corrupted failure
edges; natural admissions are measured separately on the unchanged frozen sample.
