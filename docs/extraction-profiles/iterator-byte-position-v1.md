# Fresh shared byte-iterator position

Source-selected next family after Data-v3 3.53: original regex-automata 0.4.18
`dfa::accel::Accel::contains`, default-feature debug Cargo library, pinned
nightly-2026-09-30. `src/dfa/accel.rs` bytes 19207..19316, SHA256
adbe93883fc3aab3b6a79d41284c676d620fa9b73c78526ba923dad80ade759a.
Retain the complete crate, cfg(dfa-build), Clone-only owner and original body:

```rust
self.needles().iter().position(|&b| b == byte).is_some()
```

Behavior: search active payload bytes in order and return whether the queried
byte occurs. Header/padding do not count. Empty payload returns false without
an element read; metadata above seven still panics in needles(). Keep the
frozen sixty identities/denominators and all historical results unchanged.

## Connected blockers and bounded contract

- S09 and helper-return source/origin proof already support original len and
  needles. Reuse them; do not substitute a Copy owner or rebuild module capture.
- Fresh shared slice iterators need exact pinned slice::iter and the concrete
  Iter<u8> implementation of Iterator::position. The pinned slice implementation
  **overrides** the trait default in slice/iter/macros.rs. Its ascending loop
  stops after the first true callback and returns that relative index; exhaustion
  returns None. Shared byte slices bound the counter and exclude element Drop.
- Existing closure capture is limited to immediate Option::map_or. Position
  needs complete `|&b|` pattern/body and compiler environment observations, one
  shared captured u8, builtin byte equality and no hidden calls/writes/drop.
  Environment creation reads no byte contents; each callback copies exactly
  the current shared element. Preserve once-only receiver/closure evaluation.
- Position needs conditional, repeated read evidence for the actual traversed
  prefix, including zero iterations. Do not replace these reads with an
  unconditional whole-slice read or declare potential origins independent.
- Existing concrete scalar Option and is_some contracts already cover usize.
  Connect the produced Option<usize>, saved results and supported helper
  transport, including possible panic from the receiver-producing helper.
  Model the fresh temporary's internal cursor mutation without authorizing
  mutation of caller-owned iterators or mutable input references.

The original 3.53 capture confirms absent concrete Iter/signature observations,
absent dispatched implementation identity and an opaque closure with no retained
body/environment. General closure reference patterns are also unsupported.
Original len/needles and scalar Option/is_some evidence are present. Remaining
unknown: helper restrictions on the completed iteration graph. No new census/cache
is justified by this target; extend existing callee/environment/type evidence.

## Expectations fixed before evaluating admission

`tests/fixtures/function-families/iterator-byte-position.rs` fixes seventeen functions.
Eleven positives: Accel::len/needles/contains, slice_contains, caller, saved,
selected, window, first_position, saved_position and position_caller. Six
negatives: generated Clone, owned, mutable, effectful, saved_iterator and
move_capture. The initial contract permits only a fresh immediately consumed
shared byte iterator and an unmodified shared-capture equality closure.
Stored/partly consumed iterator state, mutable/value captures, arbitrary
predicates, iterator adapters, generic/trait-object dispatch and other element
types remain separate work.

Independent expectations use membership bitsets and first-index tables,
including every byte, duplicates, empty/full views, ignored padding, invalid
metadata, subslice offsets and saved/selected origins. Trace the pinned runtime
callback visits to validate short-circuit order; source inspection and later
graph checks must establish primitive-load behavior. Require std/no_std across
the three installed targets, independent strict verification, complete selected
matching, old regressions and forged read/order/callee/environment/policy cases.
Measure unchanged natural/frozen rosters and bounded performance controls before
claiming delivery. No activation, rewrite-safety or detector-quality claim yet.

## Verified 3.53 baseline

The fixed corpus captures **17 functions: 2 admitted /15 excluded** in all six
std/no_std contexts across the three installed CI targets. Only len and needles
admit, both independently verified through complete selected matching. All nine
remaining intended positives are blocked; no unsupported prefix is certified.
The complete original library also excludes contains while admitting its two
dependencies. Its source, build context and outcomes come from the unchanged
3.53 natural inventory (82/2582 admitted); no new natural gain is claimed.

Three independent tests pass inside a disposable copy of the complete original
crate: bitset membership for every query/seed/valid length and duplicates, panic
for all invalid headers, and first-index/short-circuit callback visits. The
runner verifies original len/is_empty/needles/contains source identities first:
`tagged-units-oracle.py /path/to/regex-automata-0.4.18 --family iterator-byte-position`.
This source/test milestone changes no production code or extraction policy;
the next delivery must connect the entire declared family.

Validation: **906 workspace tests pass /5 skipped**, Clippy, doctests and
formatting pass; rstyle **0 violations /2900 functions**, patterns **0 candidates**.

## Connected 3.54 contract

Data-v3 3.54 verifies the pinned slice implementation and one fresh
shared-byte iterator directly in the existing callee/receiver evidence. It adds
no general iterator type, capture census or cache. Original source, compiler
definition hashes for each registered target, shared-u8 instantiation, callback
parameter/equality and complete shared capture storage must all agree.

Receiver evaluation and iterator/environment creation occur once, in order.
A nonempty selector guards the captured-byte load and one repeated ascending
prefix-read operation. The prefix ends at the first equal byte, inclusive, or
at the end of the view when absent; indices are relative to that view. Empty
input selects None without loading the capture or any element. This composite
operation summarizes the reviewed drop-free loop, preserving its read semantics
and internal cursor behavior without admitting arbitrary iterator protocols.
The shared u8 capture is stable for the immediate call; its conditional saved
load relies on that proved scalar borrow/storage contract, not general callback
purity or permission to reorder effectful predicates.

Formal locations retain every possible backing origin and the evaluated view
operand. Repeated searches retain separate operation occurrences. Existing
checked helper transport retains the callee graph, conditional read occurrences
and possible receiver panic. The independent verifier checks operation inventory,
types/effects, ordered operands, both CFG guards and exact result alternatives;
it does not rely on producer regeneration.

The fixed eleven positives compose across std/no_std and all three installed
targets. Additional composition covers explicit shared array-field views,
reversed equality, endpoint mutations, saved-base snapshots and repeated
searches. Implicit array-field slice places remain excluded before emission;
they need explicit shared-view evidence. Stored iterators, other element types,
inequality/arbitrary callbacks, move/mutable captures and hidden calls remain
excluded. Runtime bitsets/first-index tables and original-crate tests specify
expectations independently; forged authority, receiver, callback, read, operand,
guard, result and policy evidence must fail strict bundle import/admission.

## Measured delivery

| Unchanged selection | 3.53 admitted | 3.54 admitted |
| --- | ---: | ---: |
| Development /17 | 2 | 11 |
| regex-automata /2582 | 82 | 83 |
| lz4_flex /150 | 8 | 8 |
| regex-syntax /956 | 22 | 22 |
| time-core /226 | 11 | 11 |
| Frozen /60 | 23 | 23 |

Zero losses. Original Accel::contains is the sole natural change; its source,
build context and all 2582 identities match the prior roster. Frozen outcomes
remain 23 admitted /30 excluded /7 inactive with identical denominators. All
135 admissions across these selections pass independent strict verification;
all 91 selected comparisons complete without diagnostics. Full natural matching
remains explicitly budget-limited.

Validation: 914 workspace tests pass /5 skipped, all six target/std contexts,
the original-crate runtime oracle, adversarial bundle/graph tests, Clippy,
doctests and formatting. Rstyle: 0 violations /2937 functions; patterns: 0
candidates. No scoring/threshold, source census, cache or admission relaxation.

Quiescent alternating measurements against exact 72ba438, seven samples after
one warmup per label (seconds, median [range]):

| Workload | Before | After |
| --- | --- | --- |
| Copy control, unchanged 12 admissions | 0.62 [0.61–0.66] | 0.62 [0.59–0.65] |
| Cyclic control, unchanged 4 admissions | 0.61 [0.60–0.64] | 0.61 [0.60–0.66] |
| Position family, 2 → 11 admissions | 0.68 [0.67–0.71] | 1.09 [1.06–1.13] |

Control matching is identical. The position corpus evaluates 1 → 55 pairs;
its median peak RSS grows 112.7 → 116.4 MiB. These are bounded controls, not
general throughput or held-out detector-quality claims. For large captures,
prefer text summaries and consume the persisted report/bundle; retain complete
evidence without repeating all function facts in progress reports.
