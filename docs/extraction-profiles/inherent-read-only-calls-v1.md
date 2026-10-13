# Verified concrete inherent calls, version 1

Frozen before implementation at `f90b7c0`, with a clean working tree on
`ccgraph/read-only-api-coverage`. Selection: `hir-data-v3`.
Extraction identity: `rstyle.rust.hir-data/3.0.0`. Matching identity remains
`rstyle.ccgraph.data-calls/1.0.0`, with unchanged numerical/string gates,
directed WL, weights, depth, multiplicity and scoring. No threshold changes,
canonicalization, inlining, mining, grouping, proposals or rewriting.
Status at freeze: unavailable until the complete verified Cargo path exists.
Data-v1, Data-v2 and all earlier selections retain their semantics.

## Syntax, resolved interface and compiler evidence

Inherit the Data-v2 concrete/drop-free/shared-reference boundary and exact
source/HIR/build, nominal-unit, receiver and location evidence. Add resolved
same-compilation-unit inherent methods invoked through method syntax or direct
qualified/aliased associated paths. Initially require a nongeneric inherent
implementation and method, an ordinary shared-reference receiver, supported
concrete remaining arguments, and supported scalar/unit results. Type/const
substitutions and early-bound lifetime implementation parameters are excluded
until separately verified. Late-bound receiver lifetimes retain the existing
explicit Data treatment; they are not nominal type identity.

`rstyle.helper.inherent-resolution/1` must bind the compiler-resolved local
associated definition, containing inherent implementation and concrete owner
type to the captured callee body. Capture the callee's own definition/implementation
identities and exact nominal owner association before helper linking. The implementation
association inventory `rstyle.helper.inherent-unit/1` records those selected-unit
definitions in deterministic identity order, with exact source file/function and
implementation spans and exact nominal owner identities. Full owner field/type
trees remain in each callee's concrete interface, independently checked against
original implementation source; the
association table does not duplicate them or supply a substitute type proof.
A callee record must agree with its unique unit association;
duplicate definitions or contradictory implementation owners fail verification.
Compiler definition hashes are corroborated captured identities, not source type
serialization or independent proofs of effects. The original-source ownership,
concrete interface and verified body remain separate semantic obligations.
The implementation must have no trait; trait declarations/implementations, dynamic/unresolved dispatch,
external bodies and unsupported substitutions cannot acquire inherent treatment.
Display names or familiar method names never establish identity or effects.
All selected bodies precede acyclic linking; capture/enumeration order is not
authority. Cross-unit/package linking remains unavailable under this slice.

Method syntax evaluates the source receiver first, then explicit arguments once
in source order. Initially require the unadjusted receiver to have the exact
canonical shared type of the callee's first argument. The pinned-compiler chain
must contain one builtin shared-reference dereference followed by one shared
borrow, ending at the identical shared receiver type. An erased/empty chain is
not a provenance shortcut. Independently verify each
transition, endpoint, instantiated signature and exact source method invocation.
Reject mutable/raw/overloaded dereferences, pointer/unsizing coercions, repeated
dereferences, by-value receiver autoref, explicit turbofish and arbitrary macros.
UFCS uses its original direct-call source operands and exact declared signature;
it does not synthesize receiver evaluation.

## Effects, locations, CFG and dependence

Derive source-body effects through the existing independently verified callee
boundary, retaining exact definition/signature/build/source dependencies. Inherit
`rstyle.helper.read-only-body/1` for shared formal field reads with scalar/unit
results, `rstyle.helper.source-body/1` for memory-free scalar/unit bodies and the
existing narrowly verified whole-formal shared-return boundary where applicable.
Borrowed memory-reading results, construction/reference-result components,
by-value memory substitutions, unsupported callees and interior mutability remain
unavailable. Shared input types alone cannot establish immutable effects.

Keep each original method operation as one source-backed CALL with original kind
`expr_method_call`; keep UFCS original kind `expr_call`. Both have statically
resolved `call_form: direct`, within the unchanged closed schema vocabulary.
The source kind and original invocation evidence distinguish their syntax.
Never inline callee operations into the caller.
Substitute read locations from independently verified actual referent origins,
preserving exact nominal field identities, ordered paths, alternatives, alias
uncertainty and multiplicity. No writes or invented independence. Argument/value
edges include the receiver at operand zero; retain existing fixed-point scalar
dataflow and divergence-sensitive dependence. Summarized bodies must have complete
nonpanicking, loop-free, returning normal flow; recursive/unknown/panicking or
nonreturning callees remain explicit exclusions. No exceptional transfer or exit
edge is fabricated. Caller scalar loops retain their existing boundary.

## Verification, projection and compatibility

Independent serialized checks must verify source CALL membership, direct/method
form, original spans/navigation, resolved definition and containing implementation,
concrete owner, receiver adjustments/signature, ordered argument dependencies,
derived summaries and substituted read/location inventories. Verify the callee's
own containing implementation/type evidence against its captured original source
context and concrete interface; caller-provided inherent/effect flags cannot
authorize admission. Tampered/missing/duplicate artifacts, identities, types,
receivers, summaries or manifests fail closed. Paired documents must still
validate unchanged vendored schemas; authority gaps keep the capability unavailable.

Reuse the existing Data-calls interface/projection interpretation: nominal and
referent distinctions remain exact, and distinct binding/location read keys on
READs and CALLs retain the existing ninth feature interpretation. Method CALL
labels remain original resolved source operations. No new graph canonicalization
or memory-summary node is introduced. Pairs require equal extraction identity,
registered matching identity, exact compatible interfaces and known semantic
build compatibility, including transitive native-width requirements. Earlier
Data versions remain incompatible with this extraction version. Exhaustively
enumerate every unordered distinct admitted-function pair, without hidden caps.

## Bounds and required connected validation

Inherit depth-eight, eight-MiB per-call and 4096 read-occurrence/receiver inventories.
Exhaustion, arithmetic, artifact and verification failures remain explicit incomplete
execution, not ordinary rejection or truncated complete success.

Require real Cargo families combining field getter methods, wrappers and multi-level
methods, direct helpers and method calls, UFCS/aliases, shared-field receivers,
aliased inputs, unit methods, constants/casts, registered metadata operations,
branching shared actual origins and caller scalar loops. Independently specify
CALL form, receiver operand and projection/read/location expectations. Verify
callee edits invalidate transitive evidence, trace on/off invariance and order
independence. Negative cases include trait/shadowed method resolution, same-named
methods on different nominal types, mutable/by-value/coerced receivers, generic
implementations, panic/divergence/recursion, interior mutability/drop/unsafe,
borrowed memory-reading results and receiver/owner/implementation/summary/document
tampering. Full workspace, producer, serialized/imported, real-command, inventory,
schema, style/patterns, fmt and Clippy gates remain required.

This is an incremental coverage/correctness slice. The broader original data
goal and contemporary applicability inventory remain required; no quality
calibration or holdout claim is authorized.

## Lossless dependency transport prerequisite

The 21-function preregistered connected family failed the inherited eight-MiB
per-call budget on 2026-10-13 (two smaller tests passed). Removing duplicated
nominal metadata did not solve recursive JSON-string escaping. The failed run
is retained in `target/inherent-calls-real-latest.log`; no coverage success is
claimed from it.

Before transport implementation, register `rstyle.helper.json-transport/1`
for Data-v3 dependencies only. This is reversible structural encoding of embedded
JSON strings, not graph normalization, compression of semantic evidence or a
cache. Retain the complete dependency including original frontend facts, exact
PDG/CFG bytes and manifest. Encode an embedded string structurally only when
parsing and standard compact or pretty serialization reproduce its bytes exactly;
otherwise retain the literal string. Record the formatting choice and restore
exact bytes before existing independent semantic/document verification. Use explicitly tagged value trees (including object keys) so arbitrary source
objects cannot collide with transport metadata. Reject malformed tags, duplicate
object keys, unsupported formats and exhausted
byte/depth budgets. Old profiles retain their original inline encoding. Keep
both encoded and restored dependency byte limits at eight MiB and call depth at
eight; encode/decode tree depth is explicitly bounded at 256. The original CALL
operation and matching projection remain unchanged. Require independent literal
round trips, malformed/tag-like source-object envelope cases, unchanged legacy bytes and the
unchanged connected-family admission target before claiming success.

The initial tagged-tree transport also failed the unchanged connected-family
budget (`target/inherent-calls-real-transport.log`, two passed, one failed).
Register the same pre-release transport as a per-call closed node table before
further implementation: intern only byte-identical tagged transport nodes, with
child indices strictly preceding parents and an explicit root. This shares exact
evidence payloads within the current dependency, never across invocations or
functions through an evidence cache. It does not alter graph nodes, interfaces,
projection or semantic evidence. Every referenced node must exist; every table
node must be reachable; cycles, forward indices, duplicate object keys and
unsupported tags fail closed. Restore exact JSON strings before independent
callee document/manifest/type/effect verification. Keep encoded and restored
byte budgets unchanged; bounded streaming serialization must stop restoration
before allocating an over-budget string. Fix this prerequisite before claiming
connected inherent-call coverage.

Exact document field order is part of manifest bytes. For JSON strings that
cannot round-trip through sorted compact/pretty object serialization, the table
may store a lexical sequence preserving every byte: unchanged punctuation,
whitespace and noncanonical tokens remain literal fragments; a JSON string token
may reference its decoded payload only when standard string escaping restores
that token exactly. Record a quoted-token tag separately from concatenated-text
fragments. Restore with bounded writes and verify complete byte equality before
encoding. Object ordering is never normalized. Non-JSON source strings stay
literal. Test unsorted keys, escaped quotes/backslashes, noncanonical Unicode
escapes, BOM/CRLF and malformed/over-budget token tables independently.
