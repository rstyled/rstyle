# Integer range patterns

Data-v3 extraction 3.2.0 adds `rust.scalar-range-patterns`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. This advances F06 to inclusive/exclusive and
open-ended integer literal ranges, including negative bounds, native widths,
and u128 extremes. Ranges compose with literal OR alternatives and pure scalar
guards. Named/associated constants, character ranges, enum/payload patterns,
nested alternatives and exhaustive matches without the final wildcard remain
outside this slice. Earlier Cargo selections do not admit integer ranges.

Capture stores one `pat_range` with its resolved integer type and a compact
literal payload containing optional start/end spellings and upper inclusivity.
Absent upper bounds have canonical `inclusive: false`; rustc's internal inclusive
marker for an absent upper bound has no comparison semantics. Endpoints are
compile-time literals; there are no endpoint reads or per-element expansion.
Unsupported-type ranges retain their existing unsupported capture boundary.

Independent source inventory maps each range to the original pattern. Eligibility
reconstructs the literal bounds from the original source byte range and checks
the captured payload and scalar type. Lowering uses the saved scrutinee once and
one `match.range` selector per arm. Its true frontier enters the guard/body, and
its false frontier proceeds to the next arm. Guard failure also advances to the
next arm. A mixed OR arm uses `match.or-patterns` with ordered kind/payload pairs;
literal-only alternatives retain the existing `match.or` representation.
Guards and bodies are never copied per alternative or per integer in the range.

The existing match-arm count, final unguarded wildcard, branch joins, early exit,
loop restrictions and normal-flow limitations are unchanged. Serialized CFG/PDG
verification, strict import and matching projection retain the selector kind,
literal descriptor, saved operand and true/false frontier dependencies. This
adds no provenance field, executable fingerprint, source snapshot or new cache.

Validation includes independent descriptor tests for signed/open bounds,
inclusivity, suffixes, comments/Unicode and unsupported bounds. Real Cargo checks
cover integer extremes, mixed alternatives, guarded open bounds, earlier-profile
exclusion, character/named-bound exclusion, changed selector rejection and a
closed/open mismatch against the original source. Natural coverage is measured
separately with the unchanged frozen inventory.
