# Whole local aggregate values

Data-v3 extraction 3.18.0 adds `rust.local-aggregate-values`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Reference-free tuples, arrays and supported nominal
values can flow through initialized local bindings, whole-binding assignments,
shadowing, conditional values, branch joins and the existing bounded loop policy.
Mutable aggregate parameters can also be reassigned. The compiler's move checks
and the existing drop-free nominal boundary remain necessary; this is not a general
ownership, memory-location or destruction model.

Reads must resolve to the exact existing binding and agree with its concrete type.
The frontend checks aggregate initializer and reassignment value types, allowing
never-completing expressions only under the existing divergence contract. Existing
constructor, interface and helper checks still establish the values themselves.
There is no new value snapshot, source witness, origin table, provenance record or
retained cache. Scalar binding behavior and reference-valued local exclusions remain.

Lowering already emits ordered whole-value operands for declarations/assignments
and binding reads. The existing kill/gen analysis now carries admitted aggregate
values through those operations, without field expansion or extra reevaluation.
A subsequent read depends on every reaching binding definition, including branch
and loop alternatives, while assignments replace the previous definition.

Independent document verification checks exact source binding identity, operation
roles, the single typed initializer/reassignment operand and its original source
expression. It reuses the existing serialized CFG fixed-point definition analysis,
which works for acyclic and cyclic binding flow. Formal-field memory reads remain
under their separate location proof. Missing definition edges, removed value
operands and same-typed value substitutions fail verification. Strict producer
reconstruction independently checks the complete graph.

Unreachable definitions and constructor components retain source/semantic checks
but do not require runtime operand edges. CFG reachability is consulted on the
missing-operand path; ordinary complete constructor checks gain no extra CFG walk.
The fixed-point proof runs only when aggregate local or mutable-parameter bindings
need it. No numeric unrolling or persistent analysis cache is introduced.

The real Cargo fixture verifies tuple/array/nominal values, copies and shadowing,
mutable parameters, assignments, branches, conditional initializers, a while loop,
struct-update bases and unreachable source. Every admitted pair passes independent
document and strict producer checks, and equivalent functions reach evaluated
accepted matching with score 1. Earlier construction fixtures now verify their
local-value cases as admitted. Reference rebindings and projections through local
aggregate roots stay excluded.

D12 advances to Partial. Local field projection, reference-bearing values and
rebindings need component/origin correspondence; borrows of locals/temporaries
remain D13. This contract does not admit indirect memory writes, destructuring,
uninitialized locals or aggregate-valued loop exits beyond existing policy. Frozen
inventory measurements are recorded in the roadmap continuation log; the full
roadmap remains active.
