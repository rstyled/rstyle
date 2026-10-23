# Mutable scalar formal memory flow

The separate `backtrack-match` development selection fixes unchanged safe
`lz4_flex 0.14.0::block::compress::backtrack_match` in the declared default-feature
library contexts. Data3.93 registers `rust.mutable-scalar-formals` for bounded
unit-returning functions over direct `&mut usize`, shared byte-slice and scalar
formals. The preceding observation-only increment (`da1be8c`) granted no admission.
D20/D21 remain broader unfinished work.

The existing formal-field record has an optional `scalar_formal_accesses` section.
Absence is missing evidence. A compiler-observed `&mut usize` formal triggers a
complete parameter roster; shared, mutable, nonreference and unknown observations
remain distinct. Reference descriptors and scalar referent locations are separate.
All reference roots may alias, including shared roots and self-pairs. Neither the
type spelling nor distinct parameter ordinals establish disjoint storage.

The independent source verifier checks actual source checksum, exact primitive
types, parameter/binding identities and spans, builtin dereference/compound
dispatch, complete direct-access inventory and original RHS bytes. Builtin compound
assignment evaluates RHS before target load/computation/store. The selected source
has four condition/index loads, then cur load/store and candidate load/store.
This is guarded lexical order, not a claim that every access executes.

Missing compiler evidence and budget exhaustion remain Unknown; unsupported
aliases, escapes, rebindings, reborrows, plain stores, macros and nested items
remain Unsupported. Limits precede alias cross-product allocation. Other mutable
referents retain existing unsupported interface observations. Historical records
omit the section and retain their wire form; shared-only functions do no new work.

The source-frozen native oracle independently checks maximal matching suffixes and
ordered bounds panics with distinct cursor objects. It does not prove captured
disjointness. A conservative aliased execution can fail after the first store;
the second compound load must not reuse a saved condition value.

Each dereference retains a separate lexical pointer read and scalar memory load;
each compound assignment retains RHS, pointer/load, checked arithmetic and store
in Rust evaluation order. The canonical root-only mutable token is distinct from
shared references and integers; nested mutable descriptors remain unavailable.
Only verified builtin scalar operations, blocks, while, direct shared byte reads
and compound scalar writes enter this family. Calls, local reference storage,
plain stores, reborrows, returned mutable references and owned cleanup stay closed.

The existing location interner gives mutable referents and shared referents
separate locations. Memory analysis computes RAW/WAR/WAW dependencies by a bounded
finite worklist. A write strongly replaces its exact root and weakly updates each
possible alias; possible aliasing is not transitively closed. Loop-carried flags
record whether a verified back edge lies between the definition/read and use.
Shared indexed reads keep their precise element selectors alongside coarse root
dependencies; root evidence does not replace element evidence.

Bounds and checked arithmetic branch to distinct terminal failure CFG exits before
a dependent store. Normal and failure snapshots retain every root's reaching
alternatives, including earlier stores and untouched entry values. Divergence
remains cyclic. This models bounded failure effects without claiming general
unwind cleanup or disjointness.

Strict replay checks source/operation correspondence, actual CFG branches, exact
memory effects, definitions and dependencies. A separate document verifier
reconstructs memory flow from serialized CFG/source evidence and compares the
complete paired root/alias/exit records; it does not invoke the lowerer/emitter.
Freshly resealed mutation tests preserve ordinary XXH3 transport checks while
attacking semantic evidence. Historical profiles receive no new permission.
Stateful helper export, borrowing locals, moves/drop and collection ownership
remain separate prerequisites for callers above this family.
