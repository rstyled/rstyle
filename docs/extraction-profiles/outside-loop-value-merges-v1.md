# Values across scalar loops: Data-v3 3.35.0

`rust.outside-loop-value-merges` completes the declared production `frac::gcd`
chain together with the [scalar tuple-assignment contract](scalar-tuple-assignment-v1.md).
The original complete rstyle-ccgraph library capture admits that function; the
separate development corpus is not a replacement natural context. Matching stays
`rstyle.ccgraph.data-calls/1.0.0`, with unchanged scoring and thresholds.

## Support boundary

Previously any nonunit conditional/match or short-circuit expression anywhere in
a loop-containing function was rejected. The new policy permits existing
supported value merges before or after scalar loops, nested outside-loop choices,
short circuits, supported scalar matches, and loop-containing alternatives.
Existing source/type/call/memory contracts still apply to every component.

Value merges syntactically inside a loop body or condition remain excluded.
Independently, a serialized merge controller or concrete alternative snapshot that
can repeat in a CFG cycle is rejected: must-have-executed evidence alone cannot
prove the correct iteration's value. This is a partial F10 contract, not general
iteration-specific value merges, value-carrying break, iterators or memory writes.

## Independent finite proof

The existing ordinary cyclic verifier retains its independent finite
reaching-definition and strict-dominance checks. Functions without merges use
that existing path. For new merge cases, the source eligibility rule and the
serialized verifier both change; syntax acceptance alone is insufficient.

The new checker reads only paired CFG/PDG documents and their existing operation
map, guards and value-site records. It builds finite states consisting of a CFG
vertex and the latest observed outcomes of controllers referenced by merge guards.
Irrelevant predicates do not split states. Boolean transfer edges update their
controller's outcome, including repeated true tests followed by a false exit or
a break. Traversal reaches a fixed finite state graph; it never unwinds a loop a
chosen number of times or fabricates executable exits for divergence.

A separate monotone worklist intersects which CFG operations must have executed
before each reachable state. Merge checks independently validate controller and
snapshot nonrepetition, guard validity and partition, corresponding alternative
edges, exact observed completion frontiers, and the selected snapshot's availability.
Merge operands also require the site's actual successor to have been reached
before their consumer, or to be that consumer. Nested merge availability applies
these same checks recursively. An earlier execution inside a loop is not accepted
as the current iteration's conditional value.

A proven unreachable successor has zero executable alternative edges. Its original
source, operations and complete static site metadata remain represented and
checked; absence is not guessed from missing evidence. In particular a value
expression after an infinite loop adds no normal exit. Malformed/mismatched site
records, invented values and unreachable executable dependencies remain errors.

All proof state is temporary and local to verification. No new serialized
provenance, snapshots or caches are introduced. The normal no-merge cyclic path
is retained. Cost depends on the finite combinations of relevant guard outcomes;
this is not a constant-cost or broad performance claim. The worklist does not
enumerate execution paths, and unrelated acyclic functions keep their prior path.
Historical extraction versions retain their original cyclic policy registration.

## Evidence and limits

The original seven-function development selection advances from 2 admitted / 5
excluded to **5 admitted / 2 excluded**: natural gcd, after_loop and before_loop
are gained; sequential and tuple_step remain admitted. cyclic_merge and
memory_target stay excluded. The natural selected gcd keeps its exact source
identity and library context and advances from excluded to admitted. Its %
operation retains the normal-flow/possible-panic disclosure; admission does not
invent totality or helper-call transport. `Frac::new` still needs its separately
identified closure, conversions, concrete Option/Result and `?` chain.

Independent runtime expectations enumerate divisors for 1024 pairs, wide u128
cases, the normalized zero pair and simultaneous RHS evaluation. The must-evaluate
fixed point is compared with an independent exhaustive simple-path intersection
oracle on all three-state graphs without entry predecessors, including states
that share a CFG vertex. Real Cargo composition additionally covers nested choices,
short circuits, matches, changing loop guards, break exits, loops in alternatives,
and a merge after divergence. Tests mutate paired frontier, guard, snapshot and
controller records; guard/snapshot alternative edges are changed consistently so
that failure must go beyond disagreement between duplicate records. Verification
and strict admission reject these mutations.

The retained whole-library natural bundle is independently loaded, document-verified
and strictly admitted, then matched against the source-selected development gcd:
Accepted, score 1. This is bounded extraction/composition evidence, not a held-out
detector-quality, abstraction activation, purity or rewrite-safety claim.
