# Fields of evaluated aggregate values

Data-v3 extraction 3.19.0 adds `rust.aggregate-value-fields`, composing with
[whole local values](local-aggregate-values-v1.md). Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Reference-free tuple and struct values support
component selection through local bindings, reassigned by-value parameters,
nested projections, and directly computed or conditional aggregate expressions.

A selection consumes the current aggregate value once. It does not invent a
memory location for a copied binding or a temporary. Existing formal-field memory
reads keep their location and alias proof. A merely mutable formal without a
whole-value assignment retains formal-field handling, for its reads. The existing formal-field observation
still records the operation and compiler field identity; for a value projection
its location is explicitly unavailable as a formal memory location. That diagnostic
is not authorization: independent source and type checks establish value semantics.
No new observation variants, frontend records, provenance, witnesses or caches are
introduced.

Verification requires a reference-free tuple/struct base, an exact original field
expression and base span, the original member's declaration slot, matching resolved
field position and nominal owner, and the exact result type. Tuple indices and
named/tuple struct fields are checked against existing concrete component types.
Nested projections evaluate the inner aggregate first. Ordinary formal chains exit
the value classification before parsing aggregate type tokens.

Lowering emits one computation with `construct = "data.value-field"`, a declaration
index in the existing literal attribute, and one whole-value operand. The existing
binding definition edges retain assignment and branch/loop flow. Unit-valued fields
keep their typed value identity when used in a constructor slot. The aggregate's
nominal identity remains in its existing concrete type; no per-field expansion or
reevaluation is introduced.

Independent document checks reconstruct the slot from original source and compiler
field resolution, forbid invented memory/scalar effects, and verify the one original
typed base operand. Strict producer reconstruction checks the complete graph.
Unreachable projections do not require runtime operands, under the existing
CFG reachability check. Formal and value projections remain in the same complete
source operation inventory; only formal projections install memory locations.

The real Cargo fixture checks local tuple/struct values, grouping, nested fields,
computed and conditional bases, reassignment and snapshots, unit fields, and mixed
formal-memory/value reads. Every admitted pair passes independent document and
strict producer verification; equivalent functions reach evaluated accepted matching
with score 1. Changing the serialized index between same-typed fields or the captured
compiler field position is rejected. An effectful base retains exactly one assignment
and one projection operand. Pure value projections have no memory locations; a mixed
function retains its one formal location. Legacy selections keep their exclusions and still admit the unchanged formal-read regression case.

Reference-bearing local values, reference rebindings, borrows of locals/temporaries,
indirect writes and unavailable nominal body-only resolution remain unfinished.
These projections introduce no ownership/drop or arbitrary dereference model.
D12 stays Partial; D13 and the full roadmap remain active. Frozen inventory results
are recorded separately in the roadmap continuation log.
