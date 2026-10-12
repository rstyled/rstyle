# CCGraph implementation decisions

Status: agreed design record, specialized by the normative
[CCGraph strict matching profile](ccgraph-profile.md), its
[machine settings](ccgraph-profile.json), and the
[Rust HIR R0 extraction profile](rust-extraction-profile.md), all version `0.1.0`.
Those profiles are specified targets, not implemented capability or original-tool
reproduction claims. CFG/PDG schema versions remain unchanged. Existing inventory
records remain historical audit evidence.

## Agreed architecture and scope

- Target conformance to a documented, versioned interpretation of CCGraph, not
  reproduction of the original tool from the paper alone.
- Reuse the Cargo/rustc integration; replace graph construction and detector
  semantics. Park abstraction mining until clone detection is solid. Independent
  style checks need not depend on clone detection.
- Use source-facing HIR-first extraction and semantic-operation-oriented graphs,
  initially with operation-granularity CFGs.
- Construct PDGs from CFGs plus binding/definition/use facts. CFG successor edges
  are not automatically PDG dependencies.
- Start with a small, strict supported subset. Missing required evidence makes a
  function ineligible with an actionable diagnostic; never replace it with zero,
  empty strings, or an assumed absence of effects.
- Keep extraction and matching profiles separately versioned. Retain source
  evidence and build a traceable, separate CCGraph projection.
- Implement imported-graph matching with independently specified fixtures before
  broad frontend integration. Use exhaustive unordered pairs, no self/reverse
  duplicates, authoritative pair output, and intermediate stage traces.
- Do not add retrieval LSH, AST bypass, abstraction canonicalization, rewrite
  execution, or rewrite-safety claims to the initial base.

## Agreed first Rust frontend milestone

Support primitive integer, boolean and unit parameters/results; local declarations,
reads and shadowing; direct local assignments and compound assignments; built-in
scalar computations/comparisons; blocks and tail expressions; `if`/`else`;
short-circuit `&&`/`||`; and explicit/early returns under the declared normal-flow
policy. This is an agreed implementation target, not current conformance evidence.

Defer arbitrary calls, references/dereferences and memory-backed writes. Encountered
unsupported constructs make the function ineligible for conformant matching; do
not erase them and analyze the remainder as complete. Loops and a bounded `match`
subset are the next milestone. Stage `for` separately because iterator/trait-call
semantics require additional policies. Add summary-backed calls before broad opaque
call support. Imported-graph matcher fixtures may include call nodes before the
Rust producer supports them.

The [Rust capability backlog](rust-capabilities.md) assigns stable planning IDs and
acceptance-fixture requirements. All new-profile implementation statuses remain
unimplemented until verified; existing legacy extraction is not conformance.

## Dependency-analysis expansion path

1. Local bindings and sets of reaching definitions, including supported
   reassignment, branches and loops.
2. Abstract memory locations and conservative call effects.
3. More precise alias analysis and call summaries within explicitly stated
   supported boundaries, not a promise of universally exact aliasing.

Maintain a capability backlog with stable identifiers, status, scope, blockers,
required facts, and positive/negative acceptance fixtures. Implementation
capability checks and diagnostics should reference those identifiers. This record
establishes the roadmap; no such code checks are implemented by this document.

Additional capability families to stage explicitly include field/index access,
mutable-reference effects, closures/captures, macros, async lowering,
unwinding/cleanup, nonreturning calls, and divergence handling. Their detailed
expansion-profile boundaries remain to specify.

## Agreed flow policies

- Honor language-defined evaluation order.
- Represent short-circuit evaluation with explicit CFG branches.
- Reject encountered unspecified evaluation order until a tested all-orders
  overapproximation is supported.
- Preserve unreachable source operations in extraction; exclude entry-unreachable
  operations from the initial matching projection under a documented policy.
- Begin with normal-flow analysis, disclose exceptional-flow omissions, and
  exclude functions when required evidence is missing.
- Preserve divergence as cycles. Never fabricate executable edges to exits.
- Derive control dependence using a specified CFG/postdominance policy, not merely
  syntactic nesting. Multiple-exit and nontermination details remain to specify.
- Execution dependencies represent mandatory sequencing, not copied CFG
  successors. For the initial reachable, acyclic normal-flow CFG, record the
  nearest dominating semantic operation as an operation's execution prerequisite.
  Synthetic entry is an analysis boundary, not a matching operation. The exact
  CFG-operation correspondence and boundary handling require explicit fixtures.
- Do not emit the full dominance transitive closure. Prerequisites follow
  transitively through nearest-dominator edges. At branch joins, possible branch
  predecessors are not individually mandatory prerequisites; use the nearest
  dominating operation instead.
- This dominance-based execution policy is a selected interpretation, not a
  uniquely specified CCGraph rule. Unsupported/missing flow blocks conformance.
  Extension to loops/repeated dynamic executions requires a separate policy and
  fixtures; do not apply the acyclic rule automatically.
- Distinguish execution and control dependencies even when they connect the same
  operations. Their counts and edge labels remain separate.

## Agreed direction and depth policy

WL neighborhoods distinguish incoming/outgoing dependencies and dependency types.
Initial base matching excludes operand-position labels; preserve operand facts in
extraction. Refinement is synchronous and uses equality-preserving common sequence
interning across compared graphs.

Define `max-finite-directed-distance` on the projected PDG:

- Follow dependency edges in their recorded direction.
- Count path lengths in edges.
- Take the maximum shortest-path distance over reachable, distinct ordered pairs.
- Unreachable pairs do not contribute. With no reachable distinct pair, return 0.
- Parallel edges do not change distance; self-edges do not add a distinct pair.

This is a named finite-distance convention, not the textbook diameter that may be
infinite for a non-strongly-connected directed graph.

For a pair, use the diameter of the graph with fewer projected nodes. On equal
node counts, use the smaller diameter. Perform exactly `h` refinements and retain
initial labels as round zero. At `h = 0`, compare round zero only.

No legacy refinement cap, graph-size fallback or silently truncated score is
permitted. Resource limits may terminate evaluation with an explicit diagnostic.
Round-zero inclusion and linearly decreasing weights are confirmed below.

## Agreed round weighting

Score all rounds `r = 0 .. h`, including initial labels at round zero. Use positive
integer weights `w_r = h + 1 - r`:

- `h = 0`: weights `[1]`.
- `h = 1`: weights `[2, 1]`.
- `h = 3`: weights `[4, 3, 2, 1]`.

For bounded overlap, average match counts by dividing the weighted sum by the
sum of weights, then divide by the smaller projected node count. For experimental
cosine, use the same pair-selected rounds and weights in the cross-kernel and
both self-kernels; do not add node-count normalization.

This is a documented extension of the paper's lower-round weighting preference,
not a uniquely specified paper formula. Round zero does not encode connectivity;
include hard negatives that test whether its contribution masks structural
mismatch. The weighting policy does not introduce a refinement cap.

## Agreed acceptance gates and thresholds

Use sequential AND gating with inclusive comparisons. After size eligibility,
every following gate must pass:

- Numerical nine-feature cosine: `>= 0.9`.
- Equal-weight mean of three string similarities: `>= 0.5`.
- One final selected WL similarity: `>= 0.9`.

Similarity thresholds are configurable within `[0, 1]` and must be finite.
They are starting settings, not calibrated quality claims. Compare unrounded
scores and report the effective profile, scoring variant and configuration.
Numerical failures require diagnostics. Do not silently convert missing evidence
or arithmetic failures into ordinary scores. Mark stages skipped after an earlier
rejection as not evaluated, rather than assigning zero scores.

Use one final selected WL acceptance score, not an additional preliminary WL gate
or second normalization. Hold other gates fixed for controlled scoring comparisons;
evaluate numerical/string filter losses as separate ablations.

## Agreed conservative matching projection

Retain supported semantic operations and meaningful analysis entities: declarations,
reads, assignments, computations, predicates, returns, actual parameter/value-input
nodes, and any explicitly required merge entities. Keep the extracted PDG intact;
projection produces a separate view with transformation provenance.

Remove only positively identified administrative nodes under explicit rules, such
as entry/exit bookkeeping with no value semantics. Exclude entry-unreachable
operations as already agreed. Synthetic origin alone is never a removal criterion:
value-input/merge entities may be synthetic and semantically necessary.

Do not collapse value plumbing or apply abstraction/algebraic/iteration/branching
canonicalization in the initial base. Each removal must specify incident-edge
handling; do not silently reconnect neighbors. Preserve retained edge types and
multiplicity and trace retained/removed nodes and any edge transformations.

Compute node/edge features, matching size and directed depth from the projected
view. Reference-count correspondence must also remain available. Category mapping,
administrative-node criteria and expression-valued branch representation are now
specified in the normative profiles and independent fixture expectations.
Reaching-definition sets do not by themselves require SSA-style merge nodes.

## Agreed feature direction

- Interpret the paper's state-node count as call-node count.
- Count nodes and edges in the projected CCGraph view; count edge multiplicity.
- Interpret reference count as distinct local/parameter bindings read in the
  projected function, not read occurrences or reference-typed bindings. Specify
  how projection retains the necessary binding correspondence.
- Serialize inputs as ordered normalized type descriptions including receiver
  role; serialize outputs as ordered normalized output-type descriptions.
- Use the unqualified source function/method name for the name field, not
  navigation IDs or module paths.
- Aggregate the three string similarities with an equal-weight mean.
- Missing required features make the function ineligible, with reasons.

These are selected interpretations, not uniquely determined paper definitions.
The matching profile specifies type normalization, exact string serialization,
Jaro-Winkler edge cases and zero-norm diagnostics.

## Switchable scoring: confirmed default and experimental alternative

The confirmed default is bounded label-count overlap with smaller-graph
normalization. Cross-product/cosine scoring is a selectable experimental
alternative. Empirical quality of neither variant has been established here.
Scoring selection must be explicit, versioned,
and included in output/configuration evidence; it must not silently change base
results. Keep other pipeline policies fixed for controlled scoring comparisons.

### Cross-product with self-normalization: experimental alternative

For round `r`, with label-count histograms `a_r` and `b_r`:

```text
K_r(A, B) = sum_c a_r(c) * b_r(c)
K(A, B) = sum_r w_r * K_r(A, B)
S_cosine(A, B) = K(A, B) / sqrt(K(A, A) * K(B, B))
```

Use identical pair-selected rounds and positive weights for all three kernels.
This preserves the literal equal-label cross-product reading but selects a
self-normalization not specified by the paper's final smaller-node denominator.
Nonempty correctly mapped graphs have positive self-kernels. The matching profile
requires diagnostics for arithmetic/contract failures; no fabricated zero-norm
score or second division by node count is applied.

### Bounded smaller-graph overlap: confirmed default

Simply dividing the raw cross-product kernel by `min(node counts)` is not a
bounded similarity. Ten same-label nodes in each graph give a round kernel of 100;
dividing by 10 yields 10, not 1. Weight normalization alone does not fix this.

The selected bounded interpretation is label-count overlap:

```text
M_r(A, B) = sum_c min(a_r(c), b_r(c))
M(A, B) = sum_r w_r * M_r(A, B) / sum_r w_r
S_min(A, B) = M(A, B) / min(node_count(A), node_count(B))
```

This changes the match-count definition, not just the denominator. It is not a
verified node alignment or subgraph match. Empty projected graphs are excluded
under the agreed eligibility policy below.

Because this score can reach 1 when only the smaller graph's label counts are
covered, the initial whole-function-oriented detector also uses a size-ratio gate.
This gate does not establish semantic equivalence or verified containment.

For either variant, use the agreed single final acceptance gate in place of the
paper's conflicting preliminary/final score presentations. Do not add redundant
gates or normalize twice.

## Agreed size eligibility

- Size is the number of nodes in the CCGraph matching projection, not source
  lines, CFG blocks or the unprojected extraction graph.
- Keep every nonempty projected graph initially: minimum size is one node.
- Exclude empty projected graphs explicitly before pair scoring.
- Require `min(size(A), size(B)) / max(size(A), size(B)) >= 0.8` by default.
- The ratio threshold is configurable; validate it and record the effective
  value in results. The matching profile permits finite decimal values in `[0,1]`;
    zero disables ratio rejection only, not nonempty eligibility.
- The comparison is inclusive: exactly 0.8 passes.
- The 0.8 default is an engineering starting point, not a paper-prescribed or
  empirically calibrated value.
- Distinguish size-policy exclusions from unsupported/incomplete extraction.
- Apply identical size eligibility in controlled comparisons of scoring variants.

## Agreed validation direction

Before unpausing abstraction mining, require both exact profile conformance and a
bounded, independently labeled quality evaluation within the supported subset.
Broad Rust coverage and large-scale optimization are later milestones.

Conformance requires independently specified graph-validation, extraction,
projection and stage fixtures, including positive, negative and boundary cases.
Passing structural schema validation or implementation-generated snapshots alone
is insufficient.

Quality evaluation must include defined edited-copy positives, independently
labeled hard negatives, small/unequal-size cases, controlled overlap-versus-cosine
comparisons, separate filter ablations and held-out examples. Report pair-level
precision/recall, inspected false positives/negatives, size buckets, runtime/memory,
and extraction exclusions. Separate eligible-input quality from whole-corpus
coverage. Ground truth must not be derived from detector acceptance; behavioral
difference alone does not settle whether code is a clone.

Define the corpus and labeling rules first. Use development examples to select
settings; agree numerical quality targets and freeze profile/configuration before
using held-out results for acceptance. Exact targets, corpus and labeling protocol
remain to specify. This decision does not claim either validation gate has passed.

## Profile checkpoint and next implementation work

- Matching/extraction version `0.1.0` is specified with machine settings and
  independently authored extraction/stage fixtures under `fixtures/`.
- `test_ccgraph_contract.py` checks static settings, hand-worked arithmetic and
  graph-oracle consistency. It is not a producer/detector conformance harness.
- Implementation support, full source-to-interchange validation and detection
  quality remain unverified. No miner activation is authorized by this checkpoint.

Next: materialize schema-conformant fixture inputs and implement the imported-
graph validation/matching harness in bounded tasks; implement Rust R0 extraction
independently and maintain capability status. Complete the required positive,
negative, boundary and extension-isolation families before certification.
Define the independent labeled quality corpus, labeling protocol and held-out
acceptance targets before calibration claims. Add new language/construct profiles
explicitly rather than silently broadening this version.
