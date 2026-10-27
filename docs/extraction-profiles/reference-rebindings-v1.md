# Reference rebindings and snapshot flow

Data-v3 extraction 3.27.0 adds `rust.reference-rebindings` (39 capabilities).
Matching remains `rstyle.ccgraph.data-calls/1.0.0`. Whole shared-reference values
and reference-bearing tuples/structs can use initialized mutable local bindings and mutable
formal bindings. Whole-value assignment replaces the binding value; it does not
write through a shared reference. Saved copies retain the origins of their own
reads. Component projection and struct updates preserve the existing exact
component correspondence.

A temporary source analysis follows existing HIR binding identities, original
initializer/RHS operations, and structured normal-flow transfers. Declaration and
assignment use kill/gen. Branches join reaching definitions; loop headers reach a
finite fixed point over those same definition identities. Return, break and
continue preserve their normal-flow roles. Scalar match arms, existing pure guards
and short-circuit flow retain their conditional transfers. Disabled debug assertions
validate their source operands without propagating operand assignments into normal
flow. Active assertions retain their evaluated operands. The analysis tracks only
reference-bearing definitions and runs only when the function has a mutable
reference-bearing binding under this capability. Other functions retain their
existing origin reconstruction.

Origin queries follow source reaching definitions to formal terminals, carrying
component selectors through aggregate values, updates and projections. Each query
visits an operation/component pair once, including cyclic dependencies. This is a
union of possible formal origins, not path correlation or a borrow/lifetime proof.
No iteration cap or bounded loop unrolling fabricates a result. The temporary
analysis uses existing facts; it adds no serialized provenance, source snapshots
or retained cache.

Serialized graphs retain existing whole-value assignment operands, binding reads,
reaching-definition edges and reference-origin annotations. Independent verification
checks source-derived origins against graph-derived terminal paths, with exact
component selection. Existing CFG kill/gen verification checks the serialized
binding edges, including loops. Strict producer verification remains required.
The data verifier also recognizes the existing unit-valued break/continue control
roles; predicates remain boolean.

The composed Cargo fixture covers assignment kills, saved references, conditional
replacement, loops, swaps through saved locals, break/continue, mutable formals,
scalar matches and pure guards, aggregate snapshots, reference-bearing updates and
whole-formal shared helper results, including a caller of a rebinding callee.
A release-build fixture checks that active assertions evaluate their assignments
and disabled debug assertions retain the earlier reference. Exact returned formal
origins are asserted.
Removing reaching-definition edges or changing origins is rejected for branch and
cyclic cases. Earlier immutable-local, aggregate, field-projection, struct-update
and body-nominal fixtures now admit their mutable/rebound whole-value cases.

This does not add pointee writes, mutable references, local/temporary storage
borrows, component assignment, reference-bearing arrays or aggregate helper results.
Shared local field-memory reads still require their separate single-origin immutable
alias boundary. Existing restrictions on conditional expression merges in functions
containing loops, callee effects, panicking/looping helper summaries and ownership/drop
remain. D11–D12 stay Partial; the complete roadmap remains active.
