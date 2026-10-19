# Reference-bearing tuple and struct values

Data-v3 extraction 3.22.0 adds `rust.reference-bearing-values`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Tuple, named-struct and tuple-struct literals may
contain existing supported shared-reference values, including nested tuples and
structs, scalar components and unit slots. These whole values can flow through
initialized immutable local bindings, aliases, shadowing, conditional values and
returns. Existing interface, nominal declaration, source ownership and drop-free
boundaries still apply.

Constructor verification retains each original source operand, its concrete type,
evaluation position and declared field slot. Tuple slots use declaration order;
struct evaluation order remains distinct from declaration order. Existing constructor
operations and operand edges represent the values. No memory location is created
for the newly constructed value, and constructing a reference component does not
read its referent. Stored reference loads, supported formal-field borrows and verified
shared helper components keep their existing location and origin proofs.

The existing reference-origin annotation records the union of the constructor's
component origins. It is a whole-value origin set, not a component map. Independent
graph verification reconstructs that union from reference-bearing operand values;
independent constructor verification separately checks every typed operand against
its exact original source slot. Swapping two same-typed reference values can preserve
the union, so the slot check remains necessary. Nested constructors and conditional
reference components compose through the same existing edges and source rules.

Whole-value local reads follow their original same-typed initializer and must occur
after that complete initializer. Recursive immutable alias resolution strictly
decreases source positions. Existing declaration operands and binding definitions
carry the value. Independent checks verify the original typed initializer, exact
binding and serialized CFG reaching definitions; reference graph verification follows
the declaration to its reference-bearing value. Mutable reference-bearing local
bindings and whole-value reassignments remain excluded. Reference-free local
assignment behavior is unchanged.

No new frontend record, graph operation, origin format, provenance, snapshot or
retained cache is added. Ordinary reference-free construction keeps its path.

The real Cargo fixture covers tuple/named/tuple-struct constructors, nested values,
immutable reference and aggregate aliases, shadowing and saved aggregates, stored
reference fields, formal-field borrows, conditional references, blocks, wrapped
reference-bearing formals, unit components, verified shared helper components and
effectful component evaluation. Every admitted pair passes independent document and
strict producer verification. Equivalent functions reach accepted evaluated matching
with score 1. Exchanging same-typed reference operands is rejected despite preserving
their origin union. An effectful struct retains exactly one assignment and all three
ordered operands, including its unit slot. Earlier fixtures now admit reference
construction and whole immutable aggregate locals. Data-v2 retains its construction
and local-value exclusions while still admitting the ordinary shared identity helper.

Reference-bearing struct updates remain excluded: a whole-base union cannot remove
the origins of overwritten fields. By-value reference-bearing arrays, component projections
from constructed/local reference-bearing values, by-value memory-reading helper
argument substitution and reference-bearing helper results need additional
component/origin correspondence. Mutable reference
rebindings and local/temporary storage borrows remain separate work. D11 and D12
stay Partial, D18 remains unfinished, and the full roadmap remains active. Frozen
inventory and final validation are recorded in the roadmap continuation log.
