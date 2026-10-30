# Bounded scalar loop helpers

Data3.76 (`rust.bounded-scalar-loop-helpers`) connects the unchanged ryu1.0.23
`pow5factor_32 → multiple_of_power_of_5_32` family. The
[source selection](../../tests/fixtures/function-families/scalar-loop-callee-selection.json)
fixes original bytes and three compiler/target contexts independently of gains.

The contract accepts the complete alpha-renamable factor-of-five grammar:
mutable u32 input, count0, active builtin debug assertion against zero,
quotient/remainder of the same saved input by5, remainder-controlled break,
quotient assignment, checked increment and final count. Distinct lexical/HIR
bindings, source spans, typed operands and original-operation anchors are checked.
Strict producer regeneration and cyclic verification establish full normal-flow
agreement; the additional proof binds the single backedge, break and dominance
order. Neighboring source and helper identity checks remain mandatory.

For positive inputs each backedge strictly reduces the positive input by5;
5^13 <= u32::MAX < 5^14 bounds execution to13 backedges/14 iterations. Zero
panics at the initial assertion. The summary records may_panic=true,
always_returns=false and bounded_loop={may_diverge:false,
positive_max_backedges:13}. This is normal-flow termination evidence, not a
complete exceptional CFG, panic-hook or cleanup model. Existing partial panic
limitations remain visible. Missing/disabled/unknown debug or overflow settings,
unpinned libraries, extra operations and other loop grammars do not certify.

Only independently proved loops pass the helper boundary. Both strict and
trusted-generated imports reconstruct bounded summaries; historical document
policies cannot acquire the new helper permission. Whole caller/helper and
transitive-wrapper composition, alpha-peer matching, source mutations, unbounded
loops, recursion, settings and resealed summary/graph/link/policy attacks are
covered. General termination, recursion and other loop families remain separate.

The independent arbitrary-precision divisibility oracle exercises42810 cases,
including power boundaries and zero p=0. Native execution uses exact source-hashed
private snippets and is labeled a private-source witness. Natural acceptance uses
unchanged full Cargo packages on all three targets; native x86/macOS execution is
not inferred from cross-target capture. Measurements and final gates are in the
root roadmap; no held-out detector or rewrite-safety claim follows.
