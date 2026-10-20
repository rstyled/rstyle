# Bitset cardinality: source-first target

This contract selects a complete cardinality/emptiness chain against Data-v3
3.50 (`ec97611`) before activating support. Preserve the frozen sixty identities,
denominators and historical results, and all existing development selections.

## Original source and build context

Select regex-automata 0.4.18's complete default-feature debug library, pinned
nightly-2026-09-30 / rustc 5c543b0b8c73c7b72bc8284ced4fb22ead15734d,
aarch64-unknown-linux-gnu. Require composed capture/verification/admission/matching
on installed x86_64 Linux and aarch64 macOS targets too; cross-target capture is
not native runtime validation. Retain the original modules, imports and neighbors.

Both methods are in `src/util/look.rs`:

| Function | Byte range | SHA-256 |
| --- | --- | --- |
| LookSet::len | 13114..13328 | 8c28b6c9656796472b49e0ad77a5b8e44ffd919bad31c4ecc41db4b55339890c |
| LookSet::is_empty | 13403..13464 | 6cb522a32ff9b04eeaa82bcd249d5d6af393bbb8707c26236ca1c3ee317554f3 |

The original struct derives Clone, Copy, Default, Eq and PartialEq, and contains
one public u32 field. Its empty/full constructors and set operations already
admit in this exact unit. Source gives `len` as
`usize::try_from(self.bits.count_ones()).unwrap()` and `is_empty` as
`self.len() == 0`. All u32 inputs matter, including high bits beyond currently
assigned look-around flags; do not assume only ten low bits are reachable.

## Complete known blocker chain

1. `u32::count_ones` needs exact pinned primitive dispatch, receiver/result types,
   bit-width semantics and range 0..=32. A similarly named user method supplies
   no authority. Inspect captured builtin adjustments before defining its summary.
2. `TryFrom<u32> for usize` is a different trait operation from existing From
   summaries. Confirm core implementation identity, source spelling and resolved
   argument/result types. On the selected 64-bit targets every u32 fits, but the
   Result's actual error type and representation still need evidence. Do not
   turn an unknown Result into a scalar or grant success merely from its name.
3. Concrete `Result<usize, TryFromIntError>::unwrap` must compose with the checked
   conversion and retain evaluation once, payload dependence, panic/drop facts
   and any explicit limitations. Current byte-array TryFromSliceError support
   does not authorize this integer Result. Do not infer arbitrary error purity,
   cleanup or general Result support from a special-case conversion.
4. Compose all three operations into the original owned-Copy getter, then its
   is_empty helper and callers using saved/selected values. Confirm source/HIR
   inventories remain complete and helper summaries admit the resulting chain.
   The observed first rejection is not a complete blocker inventory.

Unknowns to resolve from source/capture: whether the compiler uses a direct
TryFrom impl or blanket delegation; exact integer error and unwrap identities;
whether Result type normalization or body/source inventories need extension;
and whether existing helper return/normal-flow checks reject the composition.
Reuse current concrete types, library resolution, call summaries and normal-flow
proofs. No extra source census, provenance registry or cache is justified.

## Independent expectations and negative boundaries

Before admission evaluation, the fixed development file
`eval/coverage-development/bitset-cardinality.rs` declares eight positives:
LookSet::empty, full, len, is_empty, and count, empty, saved, selected.
The six negatives are generated Clone, mutate, Custom::count_ones, custom,
narrowing and propagate. Non-Copy owned custom receivers, mutable owners,
unregistered narrowing conversions and general Result/`?` propagation are not
covered by this bounded target. Do not remove these functions to gain admission.

For any u32, len equals the sum of its 32 binary digits and is_empty is equivalent
to bits == 0. All selected contexts return a usize in 0..=32 without a conversion
failure. Saved values and branch selection retain the chosen object's bits;
input alias independence is not needed for these owned builtin-Copy values.
Independent runtime tests append only tests to the complete original crate,
verify both source identities and compute expectations by repeated division and
remainder, without count_ones or TryFrom. They cover every byte in every lane
and its complement, every single bit and complement, extreme/alternating words,
and 4096 mixed words. This is finite boundary coverage, not exhaustive u32 testing.

Activation requires full composed Cargo capture, independent serialized checking,
strict owning-unit import and matching; semantic negative/mutation checks;
existing assertion/subslice/callback/getter regressions; unchanged natural and
frozen selections with measured gains/losses; alternating performance controls
against exact ec97611; rstyle zero violations and patterns zero candidates.
No threshold/scoring changes, general macros/iterators, rewrite-safety or held-out
detector-quality claim follows. Accel::contains additionally requires original
iter().position() and a callback; as_accel_tys requires aliases, formatted
assertions and slice-to-array conversion. Neither is reduced to this scalar chain.

## Baseline evidence and resolved dependencies

At unchanged 3.50 the fixed fourteen functions have **two admitted controls /
twelve excluded**, complete capture/assessment/enumeration/matching, and strict
independent checks of the controls. The eight future positives and six negatives
above were specified before evaluating this baseline. Both original-crate oracle
tests pass, including high bits and all listed mixed-word cases. This audit adds
no production capability and claims no natural admission gain.

Native captured HIR identifies count_ones as `core::num::<impl u32>::count_ones`,
with u32 receiver/result; TryFrom as the core trait's try_from, with u32 argument
and `Result<usize, TryFromIntError>` result; and the core Result unwrap with that
exact receiver and usize result. The intermediate Result is currently not
normalized. The trait path alone is not an impl-selection proof.

Pinned core `num/uint_macros.rs` defines count_ones via ctpop. The 64-bit section
of `convert/num.rs` expands the reverse unbounded usize/u32 conversion: it returns
`Ok(value as Self)` for every u32. `num/error.rs` declares TryFromIntError with
an IntErrorKind field and builtin Copy derive; it is not a unit error and its
representation must not be invented or conflated with TryFromSliceError.
These findings resolve the concrete conversion/error-source questions, while
cross-target capture, normalized Result evidence, independently checked summary
composition and helper restrictions remain implementation work.

The existing Callee record already supports a selected `implementation` alongside
its trait identity; scalar From capture uses it today. Extend that bounded capture
hook for the declared TryFrom instantiation rather than add another inventory.
The concrete observation grammar currently has only the specialized
SliceArrayResult. Integer Result support must retain its own checked payload and
core error identity, preserving historical serialized shapes; adding a scalar
summary alone would leave the intermediate value unsupported.

Validation for this source-only audit: focused CLI baseline, both new original
oracle tests and all six existing tagged-unit oracle tests pass. Workspace
Clippy with warnings denied, formatting, Python parsing and whitespace checks
pass. Fresh rstyle checks 2848 functions /zero violations; patterns reports zero
candidates. The preceding 887-test full run validates unchanged production code;
this audit adds no production changes and does not claim a new full-suite run.
