# Shared field borrows, version 1

Implementation baseline: `eb658c4`, clean `ccgraph/read-only-api-coverage`.
This supplement belongs only to the separately registered `hir-data-v1` local
extraction/matching extension. Historical profiles and pinned compat are unchanged.
The later `field-value-context-v1.md` supplement corrects the base interpretation
below: non-reference aggregates compute places, but stored reference bases require
their actual value read before further projection. Its context/type proof applies
to all new admission under this local Data contract.

Support original shared `&place` invocations where place is a verified formal
struct/tuple field path reached through a shared referent. The resolved result
must be precisely a shared reference to the resolved field type. The first slice
excludes reference-bearing target values, local/temporary borrows, raw/mutable
borrows, indexes, explicit dereferences and arbitrary place expressions. Existing
source, nominal drop-free and effect proofs apply recursively. The compiler still
establishes valid Rust lifetimes; matching omits lifetime spelling and preserves
reference/referent and nominal identity.

Retain each original borrow and field projection as its own source/HIR operation.
For a borrow operand, field projections compute places rather than loading field
values. Each evaluates its base once in source order, has one base value edge,
no reads/writes/definitions, and a precise formal location annotation. Reading the
shared formal binding remains a binding read. No scalar/aggregate referent read
or reaching-definition edge is invented merely to construct an address.

The borrow operation computes a reference with one operand edge to its final
field place, no mutation or panic, and exact result/referent type correspondence.
Its output origin is the existing formal field location; it does not append an
extra dereference to that path. Shared formal roots retain conservative possible
aliasing, including self aliases. Supported ordinary branching retains all borrow
origins in value alternatives and returns. Loop value merges retain their current
explicit exclusions.

Independent verification must recover all borrow/place ownership from the exact
original source and resolved HIR, prove every location against the formal type,
and inspect actual serialized place/borrow operands, roles, types, effects and
reference origins. A supplied place marker or origin list is insufficient. Distinct
reference counts include actual binding/field-value reads, not address projections.
Projection, features, string/WL comparison, gates, thresholds and scoring are
unchanged. Place/borrow computations use the existing OTHER interpretation.

Existing field/source/type budgets apply without truncation. Missing, duplicate,
tampered or unresolved source/type/location/value correspondence fails closed.
Compilation/capture, assessment, enumeration and evaluation completeness remain
separate. This slice must pass the real Cargo command and exhaustive pairing,
including borrowed nested getters and conditional outputs, before admission.
