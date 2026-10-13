# Source-body read-only calls and Data-calls matching, version 1

Frozen before implementation at `da98cf5`. Opt-in selection:
`cargo rstyle clones --extraction-profile hir-data-v2 --format json`.
Extraction: `rstyle.rust.hir-data/2.0.0`.
Matching: `rstyle.ccgraph.data-calls/1.0.0`.
Status: profile registration and the existing verified Data paths are connected.
Memory-reading helper calls remain unavailable for admission until the independently
verified connected slices below are implemented. This is a local extension, not pinned compat conformance.
R0, expanded, helper and `hir-data-v1` identities and semantics remain unchanged.

## Syntax, types and evidence

Inherit the bounded concrete Data type/source/drop-free boundary and tested
primitive/metadata summaries. Initially support same-compilation-unit acyclic
direct calls to verified bodies, followed by resolved inherent shared-receiver
methods. Module paths and aliases use exact resolved definition identities.
Unbounded late-bound lifetime parameters may be resolved by the pinned compiler;
type/const/early-bound substitutions, traits and unresolved dispatch are excluded.
All selected bodies precede summary resolution, independent of enumeration order.
Selected-package calls remain unavailable until invocation-wide selected-unit
linking is implemented; external dependencies require registered tested summaries.

Each dependency retains exact frontend source/HIR/build capture, paired documents
and manifest, including transitive dependencies. Caller/callee compiler invocation,
selected unit, source ownership, instantiated signature and semantic build evidence
must agree. Changing source/build evidence invalidates dependent summaries. A
caller flag, name or successful compilation cannot prove immutable effects.

## Source-body read contracts

`rstyle.helper.read-only-body/1` records actual callee read operations with their
verified PDG operation identities, formal parameter positions, ordered resolved
location paths and value types. Keep operation multiplicity in the summary, even
when operations share one location. Derive reads from independently verified
callee READ/CALL access inventories; exclude place projections and pure reference
copies. No writes, interior mutability, unsupported drops, unsafe accesses,
allocation, unsupported panic behavior or nonreturning flow are admitted. Initially
summarized loop-containing bodies remain excluded under the returning-flow boundary.

For a shared formal, a read path begins at its shared referent. Substitute an
actual argument's verified referent origin for exactly that leading projection,
then append the remaining resolved projections. Never add or remove an extra
stored-reference dereference. For by-value formal aggregates, require exact
formal-place correspondence before substitution; until that proof exists the
call is unavailable. Every resulting location retains the caller formal root,
exact path and value type. Preserve all possible actual origins and aliases.
Unknown origins or mutation through a callee exclude the whole function.

The first memory-reading slice has scalar/unit results. Borrowed memory-reading
results require separate exact callee output-origin substitution, including
stored-reference loads and field-storage borrows; they remain unavailable until
that proof is implemented. Whole-formal memory-free shared outputs retain the
already registered `rstyle.helper.shared-return/1` boundary. Aggregate reference
result components require component-specific evidence and remain unavailable.

## CFG, dependence and serialized verification

Keep one source-backed CALL operation, without inlining or normalizing its body.
Arguments evaluate once in source order; the verified adjusted receiver is first
for methods. Retain actual argument value edges and the CALL's exact substituted
read-location accesses. Formal referent dependence follows the existing Data
field-read interpretation: verified actual roots plus exact immutable location
paths, not invented disjointness or generic memory nodes. The nested verified
callee return/value graph remains the evidence for argument/result dependence.
Normal continuation requires complete nonpanicking returning flow; no exceptional
transfer or executable exit is fabricated. Scalar fixed-point dataflow and
divergence-sensitive dependence policies remain unchanged.

Both documents retain the exact call summary inventory and evidence dependencies.
Independent verification checks actual CALL membership, source span, resolved
callee/signature, ordered operands, access multiplicity, location identities/types,
aliases, effects, returned origins and derived callee summaries. It reconstructs
expected access facts from source arguments and independently verified callee
documents, not caller re-emission. Extra, missing, duplicate or contradictory
locations/reads fail verification. All accesses refer to actual declared formal
locations and known input roots. Unchanged closed schemas must validate these
documents; otherwise the capability stays unavailable pending named authority work.

## Matching interpretation and compatibility

The separately named matching extension uses the existing Data interface tokens,
source CALL labels, strings, nine features, gates, directed WL, depth, weights,
multiplicity and exact scoring. Its only added projection interpretation counts
distinct verified read keys from reachable retained CALLs as well as READs.
Binding keys and exact formal-location keys remain distinct. The numerical
reference-count feature retains its position and meaning as this registered
distinct-key count. No `memory_summary` node is admitted, no callee graph is
projected into the caller, and no canonicalization or threshold change is added.

Pairs require equal extraction and matching identities, exact compatible Data
interfaces/nominal identities and compatible known semantic build settings.
Recursive native-width involvement requires equal known target widths. Imported
admission must verify the new registration rather than label these graphs as
pinned or earlier Data conformance. Enumerate every distinct unordered admitted
pair, including explicit incompatible/rejected decisions, without hidden caps.

## Bounds, exclusions and required validation

Retain depth-eight and eight-MiB per-call dependency budgets. Bound summarized read
occurrences by the concrete evidence entry budget; excess is explicit incomplete
execution, never truncation. Artifact, arithmetic, verification and resource
failures remain incomplete execution. Unsupported constructs remain exclusions.

Independent full Cargo families must cover field getters through wrappers,
nested reads, multiple aliased shared inputs, multi-level transitive calls,
qualified aliases, field reads composed with scalar/metadata calls and branching.
Then cover equivalent inherent-method families with exact receiver adjustment and
signature evidence. Negative tests include mutation/interior mutability/drop,
unsupported callee/result/argument origins, coercions without proof, recursion,
panicking/nonreturning callees, wrong nominal identities, read/location/summary
tampering, missing/corrupt dependencies and changed build settings. Require
source-relative/file-relative navigation, exhaustive pairs, deterministic scheduling,
trace invariance and transitive edit invalidation. Existing Data-v1 and historical
profile decisions, schemas, reports, style and patterns must remain unchanged.

This contract is coverage/correctness work. It does not register quality targets,
calibration, mining, grouping, proposals or rewriting. The broader original Data
goal, contemporary inventory and remaining capability bundles remain required.
