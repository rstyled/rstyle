# Struct update values

Data-v3 extraction 3.15.0 adds `rust.struct-update-values`, extending
[named struct values](named-struct-values-v1.md). Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Supported updates construct reference-free,
drop-free nominal values with explicit field overrides and one same-typed base.
The existing exact nominal declaration/path proof and original source checks apply.

Capture and lowering evaluate explicit field values in source order, then the base
once. The base remains an operand even when every field is explicitly supplied:
its evaluation can have scalar effects. No expansion into per-field base reads,
subtree cloning or reevaluation is introduced. Formal values, nested constructors,
blocks with supported scalar effects and conditional aggregate values compose as
bases; general aggregate local reads still require D12.

The existing `data.struct` construct and literal slot vector carry the semantics.
Each explicit operand maps to its declared field index; a final `null` identifies
the single base supplying the remaining fields. For `Record { right: x, ..old }`
with declared `left, right, stamp`, the vector is `[1,null]`. This extends the
existing attribute without new frontend records, provenance, witnesses or caches.
A base must have the exact constructor type and original source span. Arity,
duplicate/unknown fields, field types and source order are checked independently.
Unit fields use explicit declared slots; a struct base is never a scalar unit slot.

Independent document verification derives the complete mapping from original
source and nominal declarations, checks the ordered base value and all explicit
operands, and rejects a changed override index even for same-typed fields. Strict
producer verification reconstructs the complete graph. The real Cargo fixture
checks a formal base, no overrides, nested/conditional bases, explicit field and
base effects, and fully overridden construction with a still-evaluated base.
Every admitted pair passes both verification paths; equivalent updates reach an
evaluated accepted pair with score 1. Missing base capture fails frontend proof.

Tuple-struct constructors, reference-bearing values, qualified qself construction, local aggregate reads,
partial-move tracking and nontrivial destruction remain unavailable. Legacy
selections do not gain updates. D11 remains Partial for reference-bearing values
and broader constant counts; D12–D13 and the full roadmap remain active. Frozen
inventory measurements are recorded separately in the roadmap continuation log.

Data-v3 3.24.0 extends this contract to [reference-bearing struct updates](reference-struct-updates-v1.md),
using component-specific origins. Earlier construction/local-value extensions and
this update extension supersede the corresponding original exclusions above.
