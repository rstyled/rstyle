# Cyclic prefix scans: bounded F10 support contract

Active in `hir-data-v3`, `rstyle.rust.hir-data/3.41.0`, through
`rust.cyclic-value-merges`. The declaration and baseline below record selection
before activation; this contract remains bounded to verified finite scalar/data
compositions.

The selected natural family consists of two read-only prefix scanners in ordinary
published default-feature library builds on the pinned compiler:

- regex-automata-0.4.18 `src/util/wire.rs::skip_initial_padding`, source SHA256 `417a2fc4062c81ac7f4fe8312ad7660b3aa31329e9176a17a5e7e7aa021642af`.
- data-encoding-2.11.1 `src/lib.rs::skip_ignore`, source SHA256 `dee0ab652e01b1247c29b6eb6d98b6043107c84088858e8c06ee82126d439ee6`.

Selection follows original source inspection before admission assessment. Keep
the frozen 60-function selection and its denominator unchanged; these methods
form a separate development selection. Preserve whole-package Cargo contexts,
including neighbors, generated functions and unsupported callers.

Independent expectations: skip_initial_padding returns the number of leading
zero bytes capped at seven and at the input length. Empty input yields zero;
seven leading zeros yield seven regardless of the eighth byte. No indexed byte
read occurs after either bound fails. skip_ignore returns the first index at or
after its input position whose byte's translation entry differs from 129, or
input.len() when all remaining entries equal 129. If the initial position exceeds
the length, return that original position without a read. Translation lookup uses
the actual byte (including 0 and 255) and all 256 entries remain significant.
Neither function writes input or invents independence between shared formals.

Known source requirements are repeated short-circuit condition merges (F10),
current-iteration scalar snapshots, scalar local updates/reaching definitions,
slice length summaries, builtin array/slice reads with receiver/index order and
bounds evidence, plus the exact local IGNORE constant for the table scan.
Nested conjunctions in the bounded scan must not reuse a right-hand controller
outcome or value from an earlier iteration when a later bound short-circuits.
All operations and skipped-read paths must remain represented and verified.
Original Cargo capture will establish additional blockers; do not assume that
fixing the first rejection completes the family.

The intended support contract is finite verification of repeated merge values
using existing serialized CFG/PDG operations, guards, value sites and frontiers.
Do not add unbounded iteration histories, epoch provenance, source copies or a
runtime cache; do not unroll a chosen number of iterations as a proof. Source
eligibility and independent serialized checking must change together. Existing
outside-loop merging and no-merge cyclic cases must retain their results.
No loop-helper summary, general iterator, mutation, unsafe, Result/error parser
or full deserialization caller support is implied by this prefix-scanner target.

Positive composition expectations include empty/all-matching/early-mismatch
inputs, final-iteration short-circuit exits, maximum starting indices, nested
conjunctions, and loop-local branch values consumed before the next iteration.
Independent runtime oracles must compare against a direct mathematical scan,
including all byte values and explicit table entries. Adversarial document cases
must reject stale controller outcomes, stale alternative snapshots, a previous
iteration's selected value, missing reads/bounds, swapped operands and changed
completion frontiers. Retain negative source cases for unsupported writes,
iterator protocols and value/labelled exits. Correctly record divergence and
partial normal-flow effects instead of manufacturing a return or totality proof.

Natural callers remain outside this bounded family: regex-automata's dense DFA
deserializer also needs returned-reference parsing, Result/payload flow and
unsafe/source boundaries; data-encoding's decoder needs generic static parameters,
mutation, buffer state and error/payload propagation. Inspect these dependencies
before declaring their caller chains supported.


## Preimplementation baseline and dependency audit

Data-v3 3.40.0 completes original data-encoding capture/assessment for all 127
functions. The exact skip_ignore source identity above is excluded by the
cyclic-value policy. A separate six-function development corpus captures and
assesses completely: both prefix scanners, alternating loop-body values and
last_condition are excluded by that same source policy; write and value_break
retain their independent mutation/exit restrictions. Four positive expectations
and two negative expectations are fixed before feature activation. The arithmetic
composition uses ordinary addition, retaining existing checked-overflow/partial
normal-flow semantics rather than requiring a new wrapping-method summary.

Source audit establishes additional independent gates beyond that first rejection:

- eligibility::check_no_loop_merges rejects short-circuit and conditional values
  inside the loop body or condition.
- producer::profile records the current outside-cycle value availability policy;
  a new registered support boundary must describe the actual checker semantics.
- value_sites::cyclic::check_nonrepeating rejects repeated controllers and repeated
  concrete alternative snapshots. Its current must-evaluate analysis is cumulative
  and does not establish freshness since the latest controller evaluation.
- cyclic_values delegates Merge operands to that value-site proof; merely removing
  the nonrepetition rejection would leave stale-iteration operands uncertified.

The existing scalar reaching-definition and source/dispatch/type/read/bounds
checks remain required. A candidate finite kill/gen proof must invalidate relevant
availability when a controller is reevaluated, and account for nested controllers
that are skipped on a later iteration. It must independently prove alternatives,
completion frontiers and actual consumers for that evaluation, without treating
"executed at least once" as current availability. Whether the existing temporary
state abstraction suffices remains an explicit implementation unknown. No checker
or source gate has been relaxed by this audit.

The independent runtime oracle passes three tests: bounded zero-prefix lengths,
all 256 translation-byte values with explicit mismatching entries and maximal
starting positions, and 1025 alternating/final-condition cases. Those runtime
checks establish semantic expectations, not strict pipeline support. The fixtures
and oracles live in tests/fixtures/function-families/cyclic-prefix-scans.rs and
cyclic-prefix-oracle.rs. Original-package evidence remains required for each
natural function before claiming any admission gain.


## Preimplementation candidate design (historical)

Existing CFG transfer evidence already exposes structured loop backedges, and
while lowering targets the start of condition evaluation, before nested logical
operators. Prefer deriving reducible natural-loop regions independently from CFG
reachability/dominance rather than trusting a new producer flag. On traversal of
a verified backedge, a temporary kill/gen analysis can invalidate evaluation and
guard facts belonging to that loop region before evaluating the next condition.
Inner-loop iteration must preserve values evaluated in the surrounding outer
iteration; an outer backedge must invalidate both regions. Controllers skipped
by the new condition then have no surviving stale outcome.

At declaration this was a proposed proof direction, not implemented support.
Activation required independently establishing
reducibility/backedge coverage, exact kill regions, current alternative
and successor availability, and merge consumers. Existing cumulative must-evaluate
and guard-state logic cannot merely be renamed. Retain the current no-merge path
and nonrepeating-site path to avoid adding cost to unrelated supported functions.
Unknowns include irreducible serialized graphs, nested loop/merge frontiers,
continue/break paths and state-space cost. A prototype must resolve these with
independent finite-graph/semantic tests and real Cargo composition before any
capability or support claim is activated. No bounded unrolling or serialized
iteration epochs are part of this proposal.


## Original Cargo baseline completed

Fresh Data-v3 3.40.0 default-feature regex-automata capture and assessment complete
all 2582 functions: 7 admitted /2575 excluded, no unknown outcomes. All retained
names, digests and spans match the previous complete natural inventory. The
selected exact skip_initial_padding digest is excluded by the cyclic merge gate.
Data-encoding completes all 127 functions: 3 admitted /124 excluded, no unknown
outcomes; skip_ignore is excluded by the same gate. Both selections are therefore
0/2 admitted before activation. Their expected two source hashes were specified
before assessment and match the compiler-captured identities.

Both commands intentionally use max-pairs zero: full-library matching is
incomplete, while source capture and assessment are complete. No admission gains
are claimed. rstyle checks 2622 functions with zero violations; patterns has zero
candidates. The three runtime oracle tests pass. Production code, thresholds,
scoring, suppressions and historical artifacts are unchanged.


## Implemented finite freshness boundary

The verifier derives dominance, backedges and natural-loop regions from the
serialized reachable CFG. Removing the dominating backedges must leave an
acyclic graph; two-entry irreducible cycles are rejected. All natural regions
sharing a header are united, so a continue edge invalidates facts from sibling
paths too. Inner backedges kill only the inner region; outer backedges also kill
nested regions. Self-loops kill their own region. No new serialized provenance,
epochs, source snapshots or persistent cache is introduced.

On each backedge the finite guard-state network forgets controller outcomes in
that region. Its independent must-evaluation fixed point intersects predecessor
availability after generating the executed source and killing the region. This
establishes availability since the relevant iteration boundary, rather than
execution sometime in an earlier iteration. Existing exact site/frontier,
alternative, consumer, source/type/dispatch/read/bounds and scalar reaching-
definition checks still apply. No-merge graphs retain their existing path;
nonrepeating sites retain the cumulative proof without constructing loop regions.
An empty reset map performs no per-edge lookup allocation.

Independent graph tests cover nested loops, sibling continue paths, self-loops,
acyclic flow and irreducible rejection. Dominance expectations use vertex-deletion
reachability, independently of the production fixed point. A separate finite
execution-state oracle enumerates location/history-bit combinations to saturation
and checks reset-aware must-evaluation over small cyclic graphs and kill masks;
it does not choose an unrolling bound. Existing simple-path tests remain intact.

Real Cargo tests establish four positive/two negative development outcomes,
complete all six comparisons, and independently verify each admitted document.
They reject altered guards, frontiers, controllers, successors, unavailable
alternative snapshots, missing scanner reads/bounds and swapped index operands.
Additional compositions cover preloop captured values, nested loops, continue
and break. Data-v2 remains excluded. The three runtime oracle tests retain the
source-first scan and final-iteration expectations.

Both natural selected functions independently verify and strictly admit from
their original whole-package Cargo bundles. Matching them with their declared
development counterparts completes all six comparisons without diagnostics;
both corresponding pairs are Accepted at score 1. This is extraction support,
not a detector-quality, equivalence, abstraction activation or rewrite-safety
claim. Mutation, value-carrying/labelled exits, general iterator protocols and the
broader natural parser/decoder caller chains remain outside this contract.

The final original-package comparison preserves all 2,709 function identities:
regex-automata improves 7 to 8 admissions, data-encoding improves 3 to 4, exactly
the two declared scanners, with zero losses and zero unknown outcomes. Capture
and assessment complete; full-library matching is intentionally incomplete with
max-pairs zero. The separate six-function selection improves 0 to 4 admissions,
retaining both negative exclusions and completing its six comparisons. Frozen
replay run-rncfhp4y keeps all 60 identities/denominators/outcomes unchanged at
20 admitted /33 excluded /7 inactive.
