# End-of-input helpers: connected target

Select the complete default-feature debug regex-automata 0.4.18 library at
Data-v3 3.51 (`29100a2`), pinned nightly-2026-09-30, aarch64 Linux. Preserve
original declarations, derives, modules, imports and neighbors. Require composed
std/no_std development capture on the three installed targets; native CI remains
separate evidence. Frozen sixty identities/denominators remain unchanged.

| Original function in src/util/alphabet.rs | Byte range | SHA-256 |
| --- | --- | --- |
| Unit::eoi | 5523..5842 | abfcfdd0fe617b67e4c03728c19fbe1312cc45249b787801746054d3f53111e3 |
| ByteClasses::eoi | 12328..12526 | acaf1984025154fcb1aba0cc52af823cda7b142ecfeb86404360e3329b1f62e8 |

Both are expected in scope from source. Unit::as_usize, ByteClasses::empty,
alphabet_len and get_by_unit already admit in the unchanged natural unit.

The complete blocker chain is:

1. Standard assert with a literal diagnostic and an implicit builtin usize
   capture. Preserve condition evaluation, diagnostic dependencies and failure
   behavior; do not erase formatting or authorize custom Display/hidden effects.
   Current expansion leaves source/HIR/field inventories inconsistent.
2. Exact core TryFrom<usize> for u16 and its concrete integer Result. Unlike
   3.51's widening conversion, this operation can fail. Preserve success payload,
   error boundary and immediate unwrap's possible panic; no invented range proof.
   Current type capture and selected-implementation hook cover only u32→usize.
3. Exact usize::checked_sub → Option<usize> → unwrap, composing alphabet_len,
   Unit::eoi and the final tagged value. Option type capture already works;
   operation summaries/unwrap and their complete normal-flow composition do not.

Unknowns: conditional diagnostic operand correspondence, expanded formatting
operations/drop behavior, and helper panic/result composition after these fixes.
Inspect dependencies before extending capture. Reuse existing nodes, concrete
observations and summaries; no new provenance census or cache is justified.

Independent expectations: Unit::eoi(n) preserves n for 0..=256 and panics above
256, including values that fit u16. ByteClasses::eoi uses the last array byte
plus one for every 256-byte array; other entries do not affect the sentinel.
Check all last-byte values and independently varied neighboring entries, domain
edges, checked-sub underflow, narrowing boundaries and scalar saved/helper values.
Custom formatting, side-effecting diagnostics and unsupported Result composition
must remain excluded unless their complete semantics are explicitly supported.

Before implementation, fix development expectations from source and measure the
3.51 baseline. Existing original-crate tests already cover Unit::eoi's domain;
add an independent original ByteClasses oracle. Activation requires complete
Cargo capture, independent checking, strict admission/matching, mutation and
composition tests, existing regressions, measured unchanged natural/frozen
selections, bounded performance controls, and zero rstyle violations/candidates.
No general formatting, Result matching, `?`, iterator or rewrite-safety claim.


## Fixed development expectations, before evaluation

`tests/fixtures/function-families/end-of-input.rs` declares nine future positives:
Unit::eoi/as_usize, ByteClasses::empty/alphabet_len/eoi, direct, saved, mapped,
and checked. Three generated Clone methods and saved_result remain excluded.
Expected semantics include underflow panic for checked; do not assume arbitrary
inputs satisfy the natural caller's bounds. The fourteen-function historical
bitset corpus and all earlier selections remain unchanged.

The original-crate oracle checks every valid constructor input, panic messages
at invalid narrowing/domain boundaries, and all 256 last-byte values under five
neighbor configurations. It appends tests to the complete original crate; it
changes no selected function, declaration or build feature.


Source review corrected the draft's omitted original ByteClasses Copy derive
before the source-faithful baseline; the initial non-Copy diagnostic is retained
separately and does not justify widening owned construction.

## Baseline evidence

At unchanged 3.51 the thirteen-function corpus has **three admitted controls /
ten excluded**. Std/no_std Cargo capture on all three installed targets has
complete assessment, enumeration and matching; admitted controls pass independent
strict import/checking. The two original-crate runtime oracle tests pass.
No production capability is activated and no natural/frozen admission gain is
claimed. Existing frozen identities and all prior development corpora are intact.

Audit checks: focused cross-target CLI baseline and original oracle tests pass;
workspace Clippy, formatting and Python parsing pass; rstyle checks 2865 functions
with zero violations and patterns finds zero candidates. The preceding production
milestone's native CI is green on all three runners; no new full-suite run is
claimed for this source/test-only audit.

## Connected 3.52 contract

Reuse the integer-operation checker for selected usize→u16 TryFrom,
usize::checked_sub and their immediate unwraps. Pin the core definitions,
selected conversion implementation, receiver/signature, source operands and
known 64-bit build. Narrowing and subtraction unwraps retain possible panic;
no range proof, saved Result/Option or general payload matching is assumed.

Standard assertions additionally accept a literal message with distinct implicit
builtin scalar Display captures. Preserve the exact decoded message, local
identities and condition-before-diagnostic snapshots. Explicit arguments,
format flags, repeated/escaped captures and custom Display remain excluded.

The shared helper boundary accepts verified acyclic normal flow with a
reference-free checked-Copy result, checked-Copy parameters/locals, and existing
verified read-only locations. Nominal Copy comes from the closed original source
environment. Propagate may_panic=true/always_returns=false; reject forged
summaries. Existing loop, unknown-index, mutation, source and drop restrictions
remain. This connects the complete constructor/getter/caller family without a
new provenance inventory or cache. Strict admission remains a bounded
normal-flow claim; exceptional cleanup and rewrite safety are separate.

## Measured result

Against 3.51, unchanged selections and build contexts:

| Selection | Before → after admitted | Losses |
| --- | --- | --- |
| regex-automata, 2582 functions | 78 → 81 | 0 |
| frozen sixty | 20 → 23 | 0 |
| fixed thirteen-function development corpus | 3 → 9 | 0 |

Natural gains are Unit::eoi, ByteClasses::eoi and the incidental
ByteClasses::is_singleton caller. Frozen gains are time-core's
range_validated::days_in_year/days_in_month and util::days_in_month. Every frozen
identity, denominator and build context is retained; outcomes are 23 admitted /
30 excluded /seven inactive. Full package rosters: lz4_flex 7/150 and regex-syntax
22/956 unchanged; time-core 8→11/226. Historical reports remain unchanged.

The original-crate oracle additionally checks is_singleton against last-byte
255 under all neighbor configurations. A separate pinned original time-core
oracle checks all years -9999..=9999, all valid months, extreme full-range years
and invalid month/year panic boundaries against independent Gregorian rules.
Neither oracle replaces a selected body or declaration.

Validation: 897 nextest tests pass /five skipped; std/no_std development capture
passes on all three installed targets. Graph/message/operand/panic-policy and
compiler/width/definition mutations reject. Original runtime oracles, numeric
boundary laws, existing regressions, Clippy, doctests and formatting pass.
Rstyle checks 2884 functions with zero violations; patterns finds zero candidates.
All 81 natural and nine development admissions pass independent document,
producer and strict import checks; all 55 selected comparisons complete.
Frozen replay verifies 80 schema documents; all 40 package admissions additionally
pass independent producer/document verification and strict import.

Quiescent warmup plus seven alternating runs against exact 29100a2 give Copy
medians 0.89/0.89s (ranges 0.86–0.93/0.87–0.90), cyclic 0.64/0.62s
(0.59–0.66/0.60–0.64), development 0.50/0.59s (0.47–0.53/0.56–0.62).
The development corpus performs proof/matching for three versus nine admissions;
these controls do not establish a general performance or detector-quality claim.
