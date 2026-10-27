# Named struct values

Data-v3 extraction 3.14.0 adds `rust.named-struct-values`, composing with
[aggregate literals](aggregate-literal-values-v1.md) and
[repeat arrays](repeat-array-values-v1.md). Matching remains
`rstyle.ccgraph.data-calls/1.0.0`.

Fully enumerated named struct constructors evaluate supported reference-free
fields in source order. Empty braces, shorthand fields, aliases from external
modules, `Self`, nested aggregates and unit fields compose with the existing
concrete nominal and drop-free source proof. Constructor paths must resolve to
the exact nominal declaration; matching a field shape or a final path name is
insufficient. Each declared field must appear exactly once with its expected
type and original source correspondence.

The captured children and graph operands retain source evaluation order, which
can differ from declaration order. The existing operation attributes carry
`construct = "data.struct"` and a JSON field-slot vector in `literal`: for declared
`left, right, stamp`, source `right, left, stamp` carries `[1,0,2]`. This semantic
mapping preserves field identity without adding frontend records, source witnesses,
provenance tables or caches. Unit-slot validation uses the same mapping to locate
the declared unit field. No per-field subtree clones or operand reevaluations are
introduced.

Independent serialized verification derives the mapping again from original
source and nominal resolution, checks ordered operand identities and types, and
rejects an altered mapping even when the affected fields have identical types.
Strict reconstruction/import verifies the complete graph. The real Cargo fixture
checks aliases, inherent `Self` construction, reordered/shorthand fields, scalar
effects, nested constructors, unit fields and empty structs. Every admitted pair
passes independent document and strict producer verification; equivalent functions
reach an evaluated accepted pair with score 1. Swapping captured same-typed children
also fails frontend source correspondence.

Struct update syntax, qualified qself constructors, reference-bearing construction,
general aggregate local reads and helper-result substitution remain unavailable.
Nominal resolution currently requires types reachable from the function interface;
a nominal used only within an otherwise scalar body can remain excluded. Existing
whole-unit nominal restrictions still apply. Legacy selections retain their
construction exclusions. D11 remains Partial, and D12–D13, C11 and the broader
coverage roadmap remain active. Frozen inventory results are recorded separately
in the roadmap continuation log.
