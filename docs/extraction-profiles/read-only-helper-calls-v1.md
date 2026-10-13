# Local read-only helper calls, version 1

Status: frozen implementation contract; **not registered for admission yet**.
The existing memory-free helper contract remains unchanged. This contract is an
extension of `rstyle.rust.hir-data/1.0.0` and `rstyle.ccgraph.data/1.0.0` only.
Implementation slices must retain explicit exclusions until the complete evidence
boundary below crosses the real Cargo command and independent verification.

The narrower existing memory-free helper boundary now accepts canonical Data
shared-reference argument tokens with scalar/unit results. Callees still require
verified empty memory-location inventories, nonpanicking returning acyclic flow,
no generics and same-unit dependency evidence. This subset retains
`rstyle.helper.source-body/1`: shared arguments alone do not imply referent reads.
Registered slice/str metadata summaries can occur in those bodies. It does not
admit field-reading callees, borrowed call results or inherent-method dispatch.

The next bounded borrowed-result slice uses identity
`rstyle.helper.shared-return/1` and an optional exact `reference_outputs` summary
field. It is restricted to a single shared-reference result, no callee memory
locations, and return origins that are whole shared formal referents of exactly
the result type. The field is omitted for historical scalar/unit summaries.
Verified callee RETURN annotations determine the complete sorted origin set;
independent callee source/graph verification remains mandatory. Substitute only
those selected actual argument origins, never all reference-valued operands.
Source and serialized graph verifiers must check that same substitution through
the exact CALL operand positions. Aggregate-valued result components, projected
callee origins and stored-reference callee reads remain outside this slice.
This prospective slice is unavailable until its connected implementation/tests.

## Scope and resolved identities

Direct functions and inherent methods in the selected compilation unit may use
the registered concrete Data interfaces. Lifetime parameters only are permitted;
type/const parameters, traits, unresolved dispatch and external callees without a
separately registered summary remain excluded. A method receiver is argument zero.
Resolved definition identity, exact source ownership, instantiated parameter and
result trees and receiver adjustments must agree. Display names are navigation,
never identity. Module paths and aliases must resolve to that same identity.

All selected bodies must be captured before dependency resolution. Acyclic calls
are resolved in dependency rounds independent of capture order. Existing budgets
of depth eight and eight MiB per call evidence remain explicit execution limits;
overflow is incomplete execution, never a partially certified graph. Cross-unit
and selected-package calls remain unavailable until an invocation-wide verified
handoff links exact planned/observed units; no artifact-directory search substitutes
for that prerequisite.

## Verified effects and references

A summary is derived from independently verified callee frontend evidence, exact
serialized PDG/CFG and manifest. Its versioned identity must differ from the
memory-free `rstyle.helper.source-body/1` identity. It records concrete argument
and result trees, exact referent reads, possible aliases, returned-reference origins,
normal returning behavior and all transitive evidence identities. Mutation,
interior mutability, drop-bearing values, allocation, unsupported panic behavior,
nonreturning flow and recursion exclude the callee and its callers. A caller flag
cannot authorize purity. Initially loops in a summarized callee remain excluded
until returning and epoch-sensitive summary proofs are registered.

Each read names its formal parameter and ordered resolved field/element/referent
path. Stored-reference loads and reads through those references remain distinct.
Every location must have a verified type and source-backed operation. Actual
argument origins substitute formal roots without adding or removing a referent
projection. Multiple shared arguments may alias; retain every possible origin and
read without invented disjointness or writes. Missing substitution evidence excludes
the call. Reference-bearing by-value arguments require component-specific origin
evidence rather than an unordered origin set. Returned shared values require exact
return-component-to-formal-origin correspondence, including all branch alternatives.

## Source operations, CFG and dependence

Calls remain original source-backed CALL operations, never inlined graphs.
Arguments evaluate once in Rust source order; the adjusted receiver precedes
explicit method arguments. Every argument has a verified value operand and type.
CALLs retain dependencies on argument values and substituted referent reads.
Normal continuation is permitted only under the verified nonpanicking returning
boundary. No exceptional edge, synthetic exit or generic memory node is invented.
Scalar reaching definitions and divergence-sensitive dependence remain unchanged.

## Build, serialization and independent verification

Caller and callee must agree on extraction identity, semantic compiler/build
settings, overflow and debug assertions, panic strategy, target/type evidence and
selected-unit provenance. Native-width involvement is recursive through concrete
type trees and transitive callees; matching such pairs requires equal known widths.
Changing callee source, transitive evidence or build settings invalidates dependent
summaries. Missing/corrupt manifests or contradictory captured/serialized evidence
are incomplete execution, not ordinary unsupported outcomes.

Independent document verification checks actual CALL inventory, exact source spans,
dispatch, signature, operand order, summary identity, read locations, alias sets,
reference outputs and callee evidence dependencies against source and verified
callee documents. Re-lowering and comparing bytes is insufficient. Extension
payloads must fit unchanged closed transport schemas; if that is impossible the
slice stays unavailable pending explicitly identified schema authority work.

## Matching and negative cases

Use deterministic Data type tokens without nominal erasure or lifetime-based
canonicalization. Keep the CALL label/callee representation and existing nine
features, gates, strings, directed WL, weights, multiplicity and exact score.
The Data projection records source-level substituted location reads using the
existing location interpretation; reference bindings and referent reads remain
distinct. It does not project callee bodies into callers. No threshold changes.

Required complete families include scalar field getter wrappers, nested method
calls, borrowed getters and branching outputs, aliasing shared inputs, multi-level
calls, qualified aliases and caller invalidation after a callee edit. Negative
fixtures include custom same-named methods, unsupported receiver adjustment,
ambiguous reference-component origins, recursion, mutable/interior-mutable or
drop-bearing referents, panicking/nonreturning callees, missing/corrupt dependencies,
summary/location/output tampering and semantic build mismatches. Every admitted
family must reach exact manifests, frontend/producer/imported verification and
exhaustive unordered pair evaluation with trace invariance.
