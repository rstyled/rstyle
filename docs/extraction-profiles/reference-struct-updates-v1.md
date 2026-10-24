# Reference-bearing struct updates

Data-v3 extraction 3.24.0 adds `rust.reference-struct-updates`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Existing same-typed, drop-free named struct
updates can contain supported shared-reference components. Immutable local bases,
constructed bases, nested/chained updates and conditional aggregate bases compose.
Existing [reference value fields](reference-value-fields-v1.md) can select either
an overridden component or an inherited component of the result.

The existing struct slot vector remains the entire constructor descriptor. Each
explicit operand maps to its declared field; the final `null` operand supplies the
remaining fields. Source operands, exact types, nominal declarations and evaluation
order retain their existing independent checks. Explicit operands evaluate in
source order, followed by the base once, including when every field is overridden.
Unit components and scalar effects retain their original operations and operands.
There is no expansion into per-field memory operations, cloned source subtrees or
reevaluated base expressions.

Whole-value reference origins are recomputed from result components. An overridden
field follows its explicit operand; an inherited field selects the corresponding
component of the base using existing component-origin reconstruction. An inherited
nested selection retains its full selector until that component is reached. Only
reference-bearing result fields contribute origins. The result excludes origins
that occur solely in overwritten fields, while preserving an origin shared with an
untouched field. Subtracting a set of overridden origins from the base's union
would lose that distinction and is not used.

Source reconstruction obtains slots from the original constructor. Independent
graph reconstruction obtains them from the serialized descriptor and follows the
actual ordered operand edges. The existing constructor verifier independently
compares that descriptor with the original source and exact nominal declarations.
Selections from ordinary struct values use the same serialized-slot semantics,
without reparsing original constructor source during graph traversal. The existing
whole-value origin annotation format is unchanged. No new frontend record, graph
operation, provenance, snapshot, retained component map or cache is introduced.
Reference-free updates keep their existing path.

The composed Cargo fixture checks formal and immutable local bases, no overrides,
constructed and nested bases, chained updates, reordered explicit fields, all-field
overrides, shared origins, conditional bases and projections of inherited and
overridden fields. It checks exact result parameter origins and retained formal
field paths, including elimination of origins appearing only in the base's
overwritten fields. Effectful overrides, base evaluations and unit slots retain
their assignment and operand counts. Every admitted graph passes independent
document and strict producer checks; equivalent updates reach accepted matching.
Reintroducing an overwritten origin, changing an override slot or removing the
base edge fails both verification paths. Data-v2 retains its update exclusions.

Mutable reference-bearing local bindings/rebindings, by-value reference-bearing
arrays, local/temporary storage borrows, general ownership/partial-move/drop
tracking, aggregate helper results and broader memory-reading helper substitution
remain unfinished. Existing nominal declaration proof boundaries still apply.
This capability establishes none of those broader contracts. Validation and frozen
natural inventory measurements are recorded in the roadmap continuation log.

Data-v3 3.25.0 additionally supports [body-only nominal types](body-nominal-types-v1.md),
so a body constructor/update need not have its nominal type in the signature.
The remaining source-unit restrictions still apply.
