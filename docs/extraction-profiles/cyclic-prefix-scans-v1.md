# Cyclic prefix scans: selected F10 development target

This is a declared development target, not an active support contract.

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


## Verified baseline and implementation dependency audit

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
and oracles live in eval/coverage-development/cyclic-prefix-scans.rs and
cyclic-prefix-oracle.rs. Original-package evidence remains required for each
natural function before claiming any admission gain.


## Candidate finite freshness design (unverified)

Existing CFG transfer evidence already exposes structured loop backedges, and
while lowering targets the start of condition evaluation, before nested logical
operators. Prefer deriving reducible natural-loop regions independently from CFG
reachability/dominance rather than trusting a new producer flag. On traversal of
a verified backedge, a temporary kill/gen analysis can invalidate evaluation and
guard facts belonging to that loop region before evaluating the next condition.
Inner-loop iteration must preserve values evaluated in the surrounding outer
iteration; an outer backedge must invalidate both regions. Controllers skipped
by the new condition then have no surviving stale outcome.

This is a proposed proof direction, not implemented support. It must independently
establish reducibility/backedge coverage, exact kill regions, current alternative
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
