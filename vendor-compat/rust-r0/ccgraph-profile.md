# CCGraph strict matching profile

**ID:** `compat.ccgraph.strict` · **Version:** `0.1.0`  
**Interchange:** [PDG 0.1.0](pdg.schema.json), [CFG 0.2.0](cfg.schema.json).  
**Machine settings:** [ccgraph-profile.json](ccgraph-profile.json).  
**Status:** specified implementation target. No existing producer/detector is
certified, and no detection-quality or original-tool-reproduction claim is made.

MUST/MUST NOT are requirements. This document and its machine settings specialize
[the agreed decisions](ccgraph-decisions.md). Schema versions remain independent.
The schema is not a matching profile. Conflicting document/configuration values
are a contract error, not permission to select an implementation's preferred rule.

## 1. Boundary, versions and admitted evidence

Input is a set of uniquely keyed function PDGs plus validation context. Output is
accepted unordered clone pairs and stage evidence. Grouping, alignment, mining,
retrieval LSH, AST bypass and rewriting are outside this profile.

Validation context contains the paired CFG, exact source bytes/content identities,
producer/build identity, extraction-policy identity, and category/interface/binding
facts needed to independently check graph correspondence. It is supplied beside
PDG documents; do not add fields to closed interchange objects. Validated internal
views may avoid reparsing JSON but MUST preserve all contract evidence.

An extraction policy must be explicitly registered with its version, normal-flow
semantics and validation requirements in the run manifest. The first specified
policy is [compat.rust.hir-r0/0.1.0](rust-extraction-profile.md). A future Go policy
can use the same matcher without pretending to implement Rust extraction. Unknown
extraction policies are ineligible, not inferred from node shape or language name.

Each input requires:

- Structural schema validity plus semantic namespace/reference/span/provenance
  validity. Source byte identities and CFG/PDG correspondence must be checked.
- Verified support for applicable normal-flow node extraction, scalar reaching
  definitions, value joins, control/execution dependence, returns and parameters.
- Known complete function interface and a verified reference-count policy.
- Per-function evidence that unsupported constructs are absent, or a registered
  policy that explicitly supports them. Empty arrays alone do not establish absence.
- Known node/edge meanings under that policy. No unresolved memory or call effects
  may be discarded merely to claim a supported graph.

Claims of `conformant`/`supported` in producer JSON are inputs to validation, not
proof. `unknown`, `unsupported`, or `approximated` evidence for a required normal-
flow fact makes the function ineligible. `not_applicable` is acceptable only when
absence is checked. Exceptional-flow omissions may be admitted only under an
explicit bounded normal-flow policy: preserve partial CFG evidence, capability
limitations and diagnostics. Do not relabel omitted panic/unwinding flow complete.

The matcher does not prove behavioral equivalence, complete language semantics,
subgraph containment or safe extraction. Hand-authored stage fixtures may exercise
call labels, cycles and self-edges independently of production eligibility; that
does not authorize the R0 producer to accept those constructs.

## 2. Operation inventory and conservative projection

Validate the extracted graph before projecting it. Derive operation reachability
from the paired CFG's entry, not array order, constant folding, or source lines.
Retain the unmodified PDG and emit a separate projection with origin correspondence.

1. Keep all supported reachable operations, including declarations, reads,
   computations, literals, assignments, predicates and actual returns.
2. Retain every actual formal-input value entity, including unused parameters.
   They are not executable CFG operations and must not be deleted for lacking a
   CFG reachability vertex.
3. Retain required nonexecuting value entities/merges iff their validated value
   sites are reachable. Use the registered policy's explicit correspondence.
4. Remove entry-unreachable operations and entities belonging exclusively to
   unreachable sites. Diagnose malformed edges crossing this boundary; do not
   hide a missing reachable producer by deleting its consumer edge.
5. Remove `entry`/`exit` nodes only when validated as administrative boundaries
   with no binding definitions, reads/writes or value/data semantics. If they
   carry meaningful values, this projection is ineligible until an explicit
   semantic representation is supplied. Do not remove all synthetic nodes.
6. Drop edges incident on removed nodes. Never splice their neighbors. Preserve
   every retained edge occurrence, its direction and family. Validate dependency
   completeness again for the retained slice; do not lose required value sources.

Record every retained/removed node and edge with a reason and origin mapping.
`memory_summary` is not admitted by this initial projection. `other` requires a
registered, justified semantic meaning; it is not a fallback for unknown source.
No read sharing, transparent-binding collapse, expression flattening, algebraic,
iteration, constructor or branch canonicalization is performed.

R0 creates no formal-output graph nodes; output metadata and actual returns suffice.
Other registered policies must document any meaningful formal-output entities.
This mapping does not automatically authorize new extraction constructs.

## 3. Explicit category mapping and numerical features

The following mapping is an intentional profile interpretation of the paper's
five node classes, not a claim that the interchange enum inherently defines them:

| Validated PDG category | Initial WL label / node-count coordinate |
| --- | --- |
| `declaration` | `DECL` / NumOfDeclNodes |
| `assignment` | `ASSIGN` / NumOfAssignNodes |
| `control` | `CTRL` / NumOfCtrlNodes |
| `call` | `STATE` / NumOfStateNodes |
| `read`, `computation`, `formal_input`, `return`, `formal_output`, `merge`, `other` | `OTHER` / NumOfOtherNodes |

Calls resolve the paper's state/call terminology. Arithmetic computations are NOT
assignment nodes. Operators, literal values, original frontend kinds, names,
types, IDs, spans, branch outcomes, operand positions and optional annotations
are excluded from initial labels. They remain available as extraction evidence.
Every projected node belongs to exactly one of the five classes.

Extract this nonnegative integer vector, in exactly this order:

```text
[ NumOfCtrlEdges, NumOfExcEdges, NumOfDataEdges,
  NumOfDeclNodes, NumOfAssignNodes, NumOfCtrlNodes,
  NumOfStateNodes, NumOfOtherNodes, NumOfReference ]
```

Count projected edge occurrences by `control`, `execution`, `data`, preserving
parallel multiplicity and self-edges. Different families on identical endpoints
are separate occurrences. Do not deduplicate by endpoint pair. Count nodes after
projection; the five node counts must sum to projected size.

`NumOfReference` counts distinct local/parameter symbol IDs read by retained read
operations, including decomposed compound reads. Repeated reads count once;
shadowed bindings count separately. Writes, declarations, interface metadata,
implicit borrow syntax and literals alone do not count. Resolve each read access
to a known symbol. Recompute this count from retained reads; the producer's known
reference_count must agree under its recorded policy for the same reachable slice.
Unknown reference counts or policy mismatch block eligibility, never become zero.

Numerical similarity is cosine of these vectors. A nonempty correctly mapped
projection has a positive norm. Zero norm is a contract diagnostic, not an empty-
vector similarity convention. All nine coordinates are required.

## 4. Interface normalization and string features

Source interface arrays are ordered by contiguous zero-based position; validate
that order rather than silently repairing it. Require complete, known types,
roles and boolean `variadic` facts. Names may be explicitly not applicable because
parameter names are not used in these string features.

The initial normalization vocabulary is:

```text
bool, i8, i16, i32, i64, i128, isize,
u8, u16, u32, u64, u128, usize, unit
```

Resolve aliases to their primitive type under the extraction policy; normalize
Rust `()` to `unit`. Retain original type evidence. For R0, validate the known
`compat.rust.hir-r0:resolved-type` extension on each interface element/symbol and
`compat.rust.hir-r0:type` value-node annotations. The adapter supplies normalized
types from those facts, never by replacing alias names in raw type strings. Other
registered policies must specify equally explicit resolved-type transport.
Do not infer alias resolution
from source spelling, erase signedness/width, or map `isize`/`usize` to fixed widths.
Native-width types require known target pointer width; pairs using native-width
interfaces or operations on different target widths are ineligible for comparison
under this initial policy, with a diagnostic. All R0 builds still record targets.
Unsupported normalized types require another profile/version, not an opaque token.

Serialize each interface as compact JSON, no spaces, with Unicode characters
unescaped and standard JSON string escaping. Each element is a tuple:

```text
[role, normalized_type, variadic_boolean]
```

`IntroPara` is the input tuple array; include receiver/capture roles only under an
extraction policy that supports them. `ReturnPara` is the output tuple array.
R0 has one output of role `return`, including a unit result. A verified genuinely
empty interface serializes as `[]`, not an unknown/empty-string placeholder.

Examples:

```text
IntroPara:  [["parameter","i32",false],["parameter","bool",false]]
ReturnPara: [["return","i32",false]]
Unit result: [["return","unit",false]]
No inputs: []
```

`Name` is the known nonempty unqualified source function/method identifier. Rust
raw-identifier syntax `r#` is removed by the Rust producer; no case folding,
Unicode normalization, namespace prefix or spelling-based aliasing is performed.
Qualified identifiers do not substitute for missing name evidence.

Compute Jaro-Winkler separately for ReturnPara, IntroPara and Name, then take their
arithmetic mean. All fields contribute equally, including verified empty interface
arrays. Shared serialization punctuation affects similarity; document this bias
rather than treating the three fields as separately enforced compatibility gates.

### Exact Jaro-Winkler convention

Use Unicode scalar values, not UTF-8 bytes or grapheme clusters. Reject invalid
Unicode scalar sequences. To make repeated-character greedy matching symmetric,
orient strings by `(length, lexicographic scalar sequence)` before matching.

- Identical strings score 1, including two known-empty strings.
- One empty string scores 0. Missing evidence is not an empty string.
- Window radius is `max(0, floor(max(len(a),len(b))/2) - 1)`.
- Scan the oriented first string left-to-right. For each character, choose the
  lowest-index unmatched equal character in the inclusive window in the other.
- With no matches, score 0. Otherwise build both matched-character sequences in
  their own source order. Let `m` be matches and `t` half their disagreement count
  (not integer-rounded).
- `J = (m/len(a) + m/len(b) + (m-t)/m) / 3`.
- Let `L` be the common prefix length of the original strings, capped at 4.
  `JW = J + L * 0.1 * (1-J)`, with the prefix boost always applied; no extra
  `J > 0.7` condition. This follows the printed boost expression as our convention.

Use exact mathematical values for decisions. Hashes/library-specific variants
must not silently alter these rules. Unicode, repeated-character, empty and
prefix-boost fixtures are required.

## 5. Directed WL encoding and compression

Use the five labels above for round zero. At each subsequent round, form the
structural tuple for vertex `v`:

```text
( previous_label(v),
  sort([(edge_family, previous_label(source)) for each incoming edge]),
  sort([(edge_family, previous_label(target)) for each outgoing edge]) )
```

Incoming and outgoing lists are separate multisets. Family order is
`data < control < execution`; label ordering must be consistent within the common
interning context. Retain repeated entries. Self-edges contribute to BOTH lists.
Subkinds, IDs, operand positions, outcomes and annotations do not enter refinement.

Construct every tuple from the previous round, then update all vertices together.
Intern complete tuples in a shared equality-preserving context across compared
graphs: equal tuples iff equal labels. Hash-only equality is insufficient; collisions
must be checked against full tuples if hashing is used. Numeric label IDs are not
cross-implementation results; compare equality partitions under node correspondence.
Renaming navigation IDs or reordering unordered collections must not change scores.

## 6. Directed depth and weights

For projected graph `G`, `D(G)` is the maximum finite shortest-path distance over
reachable DISTINCT ordered vertex pairs, following all dependency families in
recorded direction. Count edges. Ignore unreachable pairs. If none exist, return 0.
Parallel edges do not change distances. Self-edges alone do not make `D` positive.

Choose `h` from the graph with fewer projected vertices; on equal vertex counts,
use `min(D(A),D(B))`. Perform exactly `h` synchronous refinements, retaining rounds
`0 .. h`. Use positive integer weights `w_r = h + 1 - r`. At `h=0`, weight is 1.
No four-round cap, large-graph fallback or early-stop shortcut changes this result.

There is a known limitation: a zero-depth pair scores only initial labels, even
if a graph contains self-edges. Capture this in fixtures/traces, not an undocumented
extra refinement. Larger depth/size may require explicit resource limits: fail
with diagnostics, never silently truncate results or claim a complete pair set.

## 7. Default and experimental scores

Let `a_r(c)` and `b_r(c)` be round label-count histograms and `nA,nB > 0`.

### Default: `smaller_graph_overlap`

```text
M_r = sum_c min(a_r(c), b_r(c))
W = sum_{r=0..h} w_r
M = sum_r w_r * M_r / W
S = M / min(nA,nB)
```

The score is bounded in `[0,1]`. Record both `M/min(nA,nB)` and `M/max(nA,nB)`
for interpretation; only the first controls final default acceptance. This is
label-count overlap, not structural node alignment or verified containment.

### Experimental: `cross_product_cosine`

```text
K_r(A,B) = sum_c a_r(c) * b_r(c)
KAB = sum_r w_r * K_r(A,B)
KAA = sum_r w_r * K_r(A,A)
KBB = sum_r w_r * K_r(B,B)
S = KAB / sqrt(KAA*KBB)
```

ALL three kernels use the SAME pair-selected rounds and weights. Do not use
independently selected self-kernel depths. Do not divide again by node count or
use a sum of separately normalized round cosines. Positive self-kernels follow
from nonempty graphs; arithmetic failure is diagnostic, never a fabricated score.

Scoring is an explicit configuration choice recorded in outputs. It changes the
kernel and normalization, not just a denominator. Neither variant is calibrated.

## 8. Gates, exact boundaries and configuration

Enumerate all unordered distinct function-key pairs. Reject duplicate function
keys before enumeration; do not merge different functions by display name. Order
keys by Unicode scalar lexicographic order, then emit pairs lexicographically.
Keys identify navigation/output only and never affect matching. No self/reverse
pairs, heuristic pair caps, grouping, ranking or display truncation alter accepted
pair evidence.

After function validation/projection, apply these gates in order:

1. Both projections are nonempty (minimum one node, no source-line floor).
2. `min(nA,nB)/max(nA,nB) >= minimum_size_ratio` (default 0.8).
3. Numerical cosine `>= numerical_threshold` (default 0.9).
4. Mean string JW `>= string_threshold` (default 0.5).
5. Selected WL score `>= final_threshold` (default 0.9).

All gates must pass. One final WL gate replaces the paper's conflicting
preliminary/final presentations. Later stages after rejection are `not_evaluated`.
No required unknown fact becomes zero. Graph ineligibility, pair incompatibility,
ordinary gate rejection and resource/arithmetic failure are distinct outcomes.

Allowed overrides are only minimum_size_ratio, numerical_threshold,
string_threshold, final_threshold and scoring. Thresholds are finite decimal values
in `[0,1]`; zero size ratio disables only ratio rejection, not nonempty eligibility.
Unknown keys/variants or invalid values are configuration errors. Minimum node
count, labeling, depth, weights and projection are fixed by this profile version.

Treat decimal configuration lexemes as exact rationals. Compare overlap, size and
JW mean as rationals. Compare nonnegative cosine against `t` by
`dot^2 >= t^2 * normA_squared * normB_squared`, avoiding square-root boundary drift.
For experimental WL cosine, use the analogous KAB/KAA/KBB comparison. Approximate
floating-point implementations are permitted only if boundary-equivalent; rounded
printed scores must NEVER control decisions. No tolerance band changes thresholds.
Detect overflow or use sufficient/exact arithmetic; never wrap/saturate silently.

## 9. Required traces and reproducibility

Record profile/schema/producer/extraction-policy versions, source/graph identities,
target and build evidence, effective configuration, selected scoring and validation
results. For each function include projection provenance, category counts, nine
features, exact three strings, reference-count justification and diameter.

For each evaluated pair include ordered keys, size ratio, gate states/reasons,
numerical dot/norms, per-field JW components/mean, selected depth, weights, label-
equality partitions/histograms per round, overlap counts or cross/self-kernels,
exact score components, displayed score and decision. Record ignored label inputs
as evidence only. Report ineligible functions separately from rejected pairs.

Trace storage may be opt-in for routine runs, but all quantities must be obtainable
in conformance mode. Enabling traces/optional annotations or later consumers cannot
change base results. Partial runs must disclose incomplete pair enumeration.

## 10. Fixtures, quality and implementation gate

[fixtures/ccgraph-matching.json](fixtures/ccgraph-matching.json) contains hand-worked
stage expectations, not schema-conformant producer graphs or analyzer snapshots.
[fixtures/rust-r0-extraction.json](fixtures/rust-r0-extraction.json) separately pins
source-to-graph expectations. A future harness must materialize interchange
records and compare exact graphs, evidence and stage traces. Static fixture
consistency tests do not establish producer/detector conformance.

Required coverage includes namespace/span/evidence failures; known-zero versus
unknown; administrative/unreachable projection; all nine coordinates; interface
serialization/JW; directed and typed edges, multiplicity, self-edges and zero depth;
weights; default/experimental scoring; inclusive boundaries; permutation/ID
invariance; pair uniqueness and extension isolation. Extraction fixtures also cover
scope, reassignment, branch joins, snapshots, compound RHS-first evaluation and
strict unsupported-source rejection.

Before unpausing the miner, also require the agreed bounded labeled quality study:
define clone labels independently, use development and held-out examples, fix
acceptance targets before held-out decisions, compare variants and filter ablations,
report pair precision/recall, exclusions/coverage, size buckets and runtime/memory.
The quality corpus/targets are follow-up acceptance artifacts, not invented results
or prerequisites to specifying this deterministic matching contract.

## 11. Interpretation ledger and change control

This is a documented interpretation/adaptation, not original CCGraph reproduction:

- Sequential AND and inclusive thresholds select the prose-oriented filter path.
- State means call; references mean distinct retained read bindings.
- Normal-flow Rust operations, actual formal inputs and value merges are preserved;
  no attempt is made to mimic undocumented Frama-C/TinyPDG bookkeeping.
- Five initial categories and directed family-aware WL are explicit choices.
- Finite directed depth, tie handling, round zero and integer linear weights resolve
  unspecified details without legacy caps.
- Bounded overlap changes the literal equal-label cross-product count; cosine is
  an explicitly experimental self-normalized alternative. Neither pretends to
  satisfy every inconsistent printed normalization simultaneously.
- Size ratio 0.8 is provisional engineering policy; one-node eligibility is not
  the paper's reported six-source-line setting.
- Type strings, three-field mean and exact JW conventions are selected policies.

A change to these semantics requires a new profile version, distinguishing fixtures
and recorded migration. Threshold/scoring overrides remain explicit run settings.
Changing schemas, broadening extraction or registering another language policy does
not silently authorize a changed matcher. Existing implementations remain
noncertified until their independent conformance gates pass.
