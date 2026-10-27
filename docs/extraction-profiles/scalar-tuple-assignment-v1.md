# Scalar tuple assignment: Data-v3 3.34.0

This increment advances the source-selected production `frac::gcd` chain. It adds
`rust.scalar-tuple-assignment`; matching remains `rstyle.ccgraph.data-calls/1.0.0`.
It does not remove the independent F10 loop/value-merge restriction.

The bounded source shape is `(a,b)=(x,y)`, including one-element tuples. Targets
must be distinct mutable lexical bool/integer bindings, and RHS must be a flat
literal tuple of matching scalar elements. Existing supported scalar operand
expressions, effects, partial arithmetic and scalar-loop composition apply.
Nested targets, repeated targets, wildcards, reference/aggregate payloads,
nonliteral tuple RHS values and field/index/dereference targets remain excluded.
There are no implicit user drops or alias-independence assumptions.

Rustc lowers destructuring assignment into a block with a generated tuple-pattern
let, temporary reads and individual writes. Capture recognizes only the exact
AssignDesugar local source, a complete no-rest flat tuple pattern, equally sized
literal RHS, no trailing expression and one assignment per generated binding.
Each write must read its corresponding compiler-generated temporary and target a
resolved scalar path. Capture then retains the original tuple assignment, LHS
paths and RHS tuple with their source spans and existing resolved types/bindings.
It does not publish generated scaffolding as original source or omit source
operations that could not be explained. Unsupported shapes retain their ordinary
rejections. No additional persisted provenance, snapshots or caches are added.

Eligibility independently parses the original assignment and checks arity,
source target names, resolved binding identity, mutability, distinctness and
concrete scalar types. Existing original-source/HIR inventory and aggregate
checks cover ordered RHS operands. The LHS tuple is a place list, not an aggregate
value construction or a set of reads.

Lowering evaluates the entire RHS tuple once in source order, retaining its
aggregate constructor. Only then does it emit the ordered scalar writes, each
using its saved corresponding operand. Thus `(a,b)=(b,a%b)` maps (17,5) to (5,2),
and remainder failure occurs before either tuple write. Effects inside RHS
expressions retain their original ordering. The implementation uses existing
operation, operand, definition and CFG records and shares the ordinary scalar
write builder. It does not clone the assignment subtree per target or introduce
a whole-function pass for functions without tuple assignment.

The selected natural gcd now retains only its post-loop value-merge rejection;
its generated-binding/source-correspondence gaps are resolved. The unchanged
seven-function development selection advances from 1 admitted / 6 excluded to
2 admitted / 5 excluded (tuple_step gained, none lost). This is foundational
progress toward the natural chain, not a claim that the production routine is
supported. F10 eligibility and serialized cyclic-value verification still both
need finite proof for merges outside loops. The function must pass its full
original-library capture, independent verification, strict admission and matching
before the target is complete.

Real-Cargo composition tests also exercise tuple updates inside scalar loops,
single bool targets, RHS assignment effects and the explicit negative shapes.
Graph expectations check original formal reaching definitions for all RHS reads
and the source operand of each write. Swapped serialized write operands,
repeated/immutable targets and reordered RHS evidence are rejected by verification
and strict admission. Independent runtime expectations enumerate divisors for
1024 pairs and cover wide u128 values and zero-divisor failure. Exact duplicate
step functions are strictly admitted and matched under unchanged settings; no
held-out detector-quality or rewrite-safety claim follows.
