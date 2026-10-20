# Fields of reference-bearing tuple and struct values

Data-v3 extraction 3.23.0 adds `rust.reference-value-fields`; matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Existing evaluated-value field projections now
accept reference-bearing tuples, named structs and tuple structs. Constructed
temporaries, initialized immutable aggregate locals, aliases, saved values,
shadowing, nested selection and conditional aggregates compose. Scalar fields of
these aggregates also remain ordinary evaluated-value projections.

The existing `data.value-field` operation records the selected declaration slot
and one evaluated base operand. Construction retains source evaluation order and
the existing nominal field-slot mapping. No memory location is invented for a
constructed value or an immutable local value. Reading a previously supported
formal field to obtain an aggregate retains that original read and its location.

Whole-value origin annotations remain unions. Selecting a component reconstructs
its origins from existing evidence rather than filtering the constructor's union.
Source verification follows the exact constructor slot, same-typed immutable
initializer and original binding identity. Nested selectors accumulate until the
corresponding component is reached. Conditional values select that component in
each alternative before joining its origins. Formal aggregates and captured field
loads retain their parameter and exact field paths; only the selected subtree's
reference paths are enumerated.

Independent graph verification carries the selection through existing declaration,
reaching-definition and alternative edges. At a constructor it follows the exact
selected operand, accounting for named-field evaluation order. At a formal input
or captured load it reconstructs the selected relative field paths. Existing source
slot, concrete type, local-flow and projection checks still apply. Copying a sibling
origin or changing the selected slot is rejected even when the aggregate union is
unchanged. No frontend record, annotation format, provenance, retained component
map, cache or graph operation is added.

A verified root returning a reference from an aggregate need not qualify as a
helper. Existing shared-result summaries require whole-formal, same-type shared
referents and memory-free bodies. That result boundary is checked before helper
certification, and its computed output origins are passed directly to summary
construction. Unsupported helper results leave the root available without aborting
capture or publishing a broader helper summary.

Existing nominal proof boundaries apply: a body-only named constructor without its
declaration in the supported interface registry remains excluded. General nominal
body-type discovery is separate S09 work. Mutable reference-bearing bindings,
rebindings, local/temporary storage borrows, reference-bearing struct updates and
arrays, helper aggregate results and broader memory-reading helper substitution
remain unfinished. Data-v2 retains its previous exclusions.

The composed Cargo fixture checks distinct same-typed reference fields, reordered
named fields, tuple structs, nested selection, immutable aliases, shadowing, formal
aggregate copies, captured aggregate loads, conditional origin unions, invariant
selected components and scalar components. Equivalent functions reach accepted
matching. Corrupt origin annotations, selected slots and base operands fail both
independent document and strict producer verification. Separate negative cases
exercise the retained mutation, borrow, update, helper and nominal boundaries.
Final validation and frozen natural inventory results are recorded in the roadmap.
