# Tuple struct values

Data-v3 extraction 3.16.0 adds `rust.tuple-struct-values`, composing with
[named struct values](named-struct-values-v1.md) and the existing reference-free
aggregate contracts. Matching remains `rstyle.ccgraph.data-calls/1.0.0`.

A compiler-resolved tuple-struct constructor call is captured as the existing
`expr_struct` aggregate kind. Only struct function constructors and the inherent
`Self` constructor receive this normalization under the new capability. Ordinary
functions remain calls, including functions sharing a named struct's spelling.
Arguments are captured and evaluated once in source order; constructor callee paths
are not runtime values or helper bodies. The original-source inventory maps call
syntax to the normalized aggregate at the same span in its existing traversal.
Independent constructor verification then proves the source declaration and slots.

Verification resolves the original path to the exact nominal type and parses its
already-retained original declaration, requiring unnamed fields. This distinguishes
an empty tuple struct from an empty named struct without adding shape flags,
frontend records, source witnesses, provenance or caches. Each argument must match
its declared component type and original source position. The existing finite,
drop-free, whole-unit nominal proof applies; constructing an unrelated nominal
used only within a scalar body can still lack the needed type resolution.

Lowering uses the existing `data.struct` operation and declaration-slot vector,
for example `[0,1,2]`. Unit components retain their typed operand slots, including
an empty block materialized as a unit value. Empty tuple structs retain an empty
slot vector. Independent serialized checks reconstruct that vector from source and
verify ordered operand identity/type; strict producer verification reconstructs
the complete graph. There is no per-component tree scan during operand collection,
subtree cloning, reevaluation or new retained cache.

The real Cargo fixture verifies external-module aliases, `Self`, empty tuple
structs, unit values, nested aggregates, scalar effects and a same-spelled ordinary
helper. All admitted pairs pass independent document and strict producer checks;
equivalent constructors reach evaluated accepted matching with score 1. Changing
the same-typed slot order or captured argument order fails verification. Relabeling
a helper returning a named struct as a tuple constructor also fails. Legacy
selections retain their exclusions.

Unit-struct path constructors, reference-bearing values, qualified qself forms and
general aggregate local propagation remain unfinished. D11 remains Partial, and
D12–D13 and the full roadmap remain active. Frozen inventory results are recorded
separately in the roadmap continuation log.
