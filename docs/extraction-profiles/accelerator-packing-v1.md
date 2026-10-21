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
