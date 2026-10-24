# Repeat array values

Data-v3 extraction 3.13.0 adds `rust.repeat-array-values`, composing with
[aggregate literal values](aggregate-literal-values-v1.md). Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. A repeat array `[value; count]` evaluates one
supported reference-free value and produces an array with its resolved element
type and length. Literal integer counts, with optional grouping, radix/underscore
spelling and a `usize` suffix, are checked against the resolved length. Named and
general constant-expression counts remain unfinished with the broader V13 work.

Capture keeps exactly one runtime child: the repeated value. The count is a
compile-time array dimension, not a runtime operand or separately lowered body.
The independent original-source visitor inventories the value and does not invent
a runtime source operation for the count. Frontend verification checks constructor
kind, one child, element type, original value span and exact literal count. Existing
source ownership, full-body eligibility and nominal drop-free checks remain.
Reference-bearing values and unavailable const blocks stay excluded. Earlier
Cargo selections retain their construction boundary.

Lowering emits one constructor computation with one value operand. The existing
resolved array type carries the repeat count; no redundant count record is added.
Zero-length repeats still evaluate their value, including supported scalar effects.
Unit repeats retain one typed unit slot even at length zero. Nested repeats compose
without expanding element copies. The compiler's Copy requirements and the
existing nominal drop-free boundary exclude unavailable ownership/drop behavior;
this slice does not introduce a general trait or destruction model.

Independent serialized checks verify the one original component value and its
position/type, including unit-slot validation. Strict reconstruction/import retains
the complete graph. Graph node/edge counts do not grow with the numeric repeat
count. No frontend fields, serialized witnesses, provenance tables, snapshots or
retained caches are introduced.

The real Cargo fixture verifies zero-length effects, unit repeats, nested and
conditional values, literal radix/underscore counts and a repeated array of 65,536
elements. Its small and large repeats have identical graph node/edge counts. Every
admitted serialized pair passes independent data-document and strict producer
verification. Equivalent repeat spellings reach an evaluated accepted pair with
score 1. Altering the resolved count fails source/type verification. Named/complex
count and reference-bearing cases stay excluded, as does the legacy selection.

D11 remains Partial for struct construction/update and reference-bearing values;
D12–D13 local value/borrow work and V13 constant expressions remain active. Frozen
natural admission measurements are recorded separately in the roadmap log.
