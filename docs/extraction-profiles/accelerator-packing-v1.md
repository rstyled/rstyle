# Borrowed accelerator packing: source-first contract

At Data-v3 3.52 (`ac6b59b`), select original regex-automata 0.4.18
`dfa::accel::Accel::as_accel_tys` in the complete default-feature debug Cargo
library, pinned nightly-2026-09-30. Source: `src/dfa/accel.rs`, bytes
19419..19773, SHA256
487f6cae717b28c4b77d15ccd4957f9d85586d734fea683c532a3583d4652c44.
Preserve modules, cfg guards, imports, neighbors and the original Clone-only
owner. Keep all frozen sixty identities/denominators and historical reports.

Expected behavior from source: return two native-endian u32 values formed from
bytes 0..4 and 4..8, including header and padding. Every stored byte contributes;
do not substitute needles(), restrict the first byte to a valid length, or
replace the non-Copy owner. Existing len/needles getters are controls.

## Full blocker chain

1. Prove `type AccelTy = u32` from its active, same-module original declaration,
   matching the captured scalar in the return array and local annotations.
   Reuse the closed module environment; no new alias census/cache. Initially
   bound support to nongeneric direct builtin scalar aliases. Qualified,
   imported, chained, local-item and aggregate aliases need separate evidence.
2. Capture exact core assert_eq!/debug_assert_eq! invocations with two builtin
   bool/integer operands. Preserve left-to-right once-only scalar snapshots,
   comparison, debug activation and possible panic. Custom PartialEq/Debug,
   explicit diagnostic arguments and other comparison macros remain excluded.
3. Compose original array-field range places with implicit shared method
   receivers, pinned TryInto -> copied Result -> immediate/saved unwrap -> native
   u32 conversion, and return a two-word array. Existing individual summaries
   are present; verify full operand/origin/conditional-read composition.
4. Complete getter, saved/selected owner and array-result caller transport.
   Current helper boundaries explicitly reject byte conversion/unwrap and
   non-view subslice places. Inspect verified read locations, checked Copy
   temporaries, drop and panic evidence before refining these restrictions.

Unknowns: additional source/HIR mismatches after comparison collapse, implicit
slice receiver lowering, conditional byte-read transport and local array-result
composition. Investigate these before redesign. Do not manufacture nonpanic,
Copy, purity, independence or supported-prefix evidence.

## Fixed expectations before admission evaluation

`eval/coverage-development/accelerator-packing.rs` fixes nineteen functions.
Thirteen future positives: Accel::len/as_accel_tys, getter, first, saved, selected,
scalar_alias, array_alias, compare_order, debug_order, saved_result, raw and
raw_caller.
Six negatives: generated Clone, new, owned, mutable, formatted and
counterfeit::compare. The owner stays Clone-only. Assess std/no_std across
three installed targets; require complete capture/assessment/matching and
independent strict checks. Retain bounds/unwrap panic limitations.

Independent runtime expectations use byte weights rather than the decoder to
compute expected words; vary every byte through 0..255 with varied neighbors,
all single-bit inputs and high metadata. Comparison tests must expose snapshot
order, debug removal and overflow/panic boundaries. Include alias declaration
and build/graph mutations, custom comparisons, dropping values, existing
regressions, unchanged natural/frozen rosters, bounded performance controls,
and zero rstyle violations/pattern candidates. Extraction support does not
activate abstraction mining or establish rewrite safety/detector quality.


Initial baseline capture aborts in byte-copy location reconstruction for
saved_result's implicit array-field receiver. Keep that failed diagnostic
separate from any successful baseline. The producer currently accepts the
signature but expects a shared reference when constructing physical read
locations. Reject that unimplemented shape before emission, then establish a
complete baseline. This is a capture/classification repair, not admission gain.

Source audit corrected the draft's saved_result negative before implementation:
3.32 already supports annotated/inferred byte Result locals. Integer/Option
saved unwrap limitations must not be applied to this older concrete byte Result.

## Capture repair boundary

The successful 3.52 baseline retains **19 functions: 2 admitted /17 excluded**
in std/no_std across all three installed CI targets. Capture, assessment and
matching complete; both controls pass independent strict verification. The
unsupported implicit array-place copy now excludes before emission instead of
aborting the owning unit. No capability or admission gain is claimed for this
repair; the full packing chain above remains pending.

Validation: **898 tests pass /5 skipped**, doctests, Clippy and formatting pass;
rstyle and patterns report **zero violations/candidates**. The pinned original
packing oracle passes. Natural/frozen selections were not replayed for this
reject-only repair; their last measured results remain unchanged.

## Connected 3.53 contract

Reuse the checked module environment for direct scalar aliases; no alias census
or cache. Standard scalar equality assertions preserve two once-only operands,
left-to-right snapshots, builtin equality/Debug and debug activation. Comparison
panic remains possible. With field proof, only verified standard assertions
whose operands are plain scalar paths/literals bypass the coarse macro veto;
macro operands with writes or field accesses still require further proof.

Implicit array-field byte-copy receivers lower as places, with physical reads
on the copy CALL only. Preserve each bounds check, exact byte window,
reads_on=ok, saved Result and unwrap panic. Extend existing checked-Copy helper
proof to the pinned copied-byte Result and transport each verified read
occurrence. These summaries describe possible callee reads, not unconditional
reads: their source operation retains its conditions and endpoint snapshots.
Do not infer disjoint windows, owner Copy or omitted cleanup.

The fixed nineteen functions meet the original **13 admitted /6 excluded**
expectations, including packing/getter/saved/selected composition. Existing
copied-window and u16 prefix callers also admit with explicit Partial evidence.

## Measured delivery

| Unchanged selection | Before → after admitted | Losses |
| --- | --- | --- |
| Development family, 19 functions | 2 → 13 | 0 |
| regex-automata 0.4.18, 2,582 functions | 81 → 82 | 0 |
| lz4_flex 0.14.0, 150 functions | 7 → 8 | 0 |
| regex-syntax 0.8.11, 956 functions | 22 → 22 | 0 |
| time-core 0.1.9, 226 functions | 11 → 11 | 0 |
| Frozen sixty | 23 → 23 | 0 |

Natural gains are original `Accel::as_accel_tys` and lz4_flex's
`HashTable4KU16::get_hash_at`: its safe-encode `get_batch` performs the copied
slice/native conversion before `hash` and the usize cast. Complete package
identities/build contexts match their baselines. Frozen results remain
23 admitted /30 excluded /7 inactive, with unchanged identities/denominators.

All 82 natural and 13 development admissions pass independent source/document
verification and strict admission; the original packing function plus the
thirteen development positives complete all **91 selected comparisons** without
diagnostics. Full natural enumeration remains deliberately incomplete under
pair-budget zero; no full-package matching or detector-quality claim follows.
All 41 admissions across the three frozen-inventory packages also pass
independent source/document verification and strict admission.

Validation: **905 tests pass /5 skipped**, plus doctests, Clippy and formatting;
rstyle checks **2,900 functions /0 violations**, patterns **0 candidates**.
Tests cover std/no_std on all three installed targets, release debug removal,
runtime byte-weight/snapshot/panic laws, and rejected mutations of alias source,
comparison operands/order/operator, bounds, conditional reads, helper summaries
and document policy. The independent oracle also passes in the original crate.

Quiescent controls against `1887e11`: one warmup and seven alternating samples
per version, complete capture/assessment/matching, identical source selections.

| Workload | Admissions before → after | Median seconds before → after |
| --- | --- | --- |
| Copied-byte helpers | 11 → 12 | 0.87 → 0.65 |
| Cyclic prefix scans | 4 → 4 | 0.63 → 0.64 |
| Accelerator packing | 2 → 13 | 0.56 → 0.75 |
| Borrowed accelerator views | 10 → 10 | 0.76 → 0.75 |

Unchanged controls retain identical matching and overlapping timing ranges.
Packing now evaluates 78 pairs instead of one; median peak RSS rises about
3.4 MiB. These small controls do not establish large-crate performance.

Remaining chains: `Accel::contains` needs immutable iterator/position callback
semantics and Option<usize>/is_some composition; `Unit::is_word_byte` needs a
named callback and its original const-built indexed table. General saved
Option/Result payload control flow and `?`, mutable/owned non-Copy receivers,
broader aliases and formatted/custom comparisons remain outside this contract.
