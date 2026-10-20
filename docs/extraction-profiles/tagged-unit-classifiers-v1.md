# Tagged byte-unit classifiers: source-first target

This is the next connected target after Data-v3 3.47. It does not activate a
capability or change admission. Use the complete default-feature Cargo library
of regex-automata 0.4.18, retaining all modules, imports, derives and neighbors.
The frozen 60-function inventory and the previous eleven-function development
corpus remain unchanged.

## Selected originals and independent semantics

Both functions live in original `src/util/alphabet.rs`:

| Function | Bytes | SHA-256 |
| --- | --- | --- |
| `Unit::as_usize` | 6688..6858 | `4dfbbd07c91e502bca13dca320768d302d9d1d8e3bd6ea859e567788862afb5f` |
| `Unit::is_eoi` | 7216..7283 | `f3eace4f01d5d7b096884d6f8c08e4de0d15abec00cfa825e738e76b66064fff` |

For all 256 byte payloads, as_usize returns the mathematical byte value and
is_eoi returns false. For all 65536 EOI payloads, as_usize returns the unchanged
payload value and is_eoi returns true. Zero payloads remain distinct tags.
Do not infer a 0..=256 sentinel bound from the public checked constructor.
The original-source oracle checks these expectations using casts and known
construction tags, independently of extraction operations.

## Complete known blocker chain and bounded contract

The 3.47 natural roster excludes both methods. Existing enum declaration,
positive Copy, variant/payload and scalar Option helper evidence remains required.

1. as_usize already captures the selected u8-to-usize From implementation.
   Its u16-to-usize call captures only the trait definition, without the selected
   implementation. Extend the existing scalar conversion capture and exact
   pinned implementation registry for the three supported compiler/target
   contexts; do not infer a contract from the From spelling or trait identity.
   Verify source correspondence and mathematical widening independently.
2. is_eoi calls the now-supported as_eoi helper, then core Option::is_some.
   Existing method-receiver capture already records the owned Option<u16>
   temporary, its shared autoref, and the exact &Option<u16>-to-bool signature.
   Reuse that evidence. A registered core source/semantic contract and source
   correspondence permission are still absent. A similarly named custom method
   must not acquire library purity.
3. The tag test must compose with the helper result and complete constructor
   callers, including existing value snapshots and positive Copy proof. It
   tests only the Option tag, without loading a payload or assuming ABI layout.
   Initial scope is owned concrete scalar Option values/temporaries with exact
   receiver evidence. Reference, generic and non-Copy payloads stay outside it.
4. Independent serialized verification must establish exact conversion/library
   authority, receiver adjustment, operands, result type and complete operation
   inventory. No parallel provenance table, duplicated source census or cache
   is justified by the observed facts.

Unknowns still requiring implementation evidence: exact selected u16 conversion
identity on all three pinned targets, required tag-test document shape and helper
snapshot composition, unchanged-profile behavior, and practical timing/RSS on
unchanged controls. Runtime execution on one host does not certify other targets.

Neighboring Unit::eoi additionally has source/HIR observation mismatch,
assertion-format macro deferral and TryFrom/Result/unwrap authority/effect gaps.
is_byte requires Option map_or and closure capture/body effects; is_word_byte
requires callback identity/body and a missing binding read. These complete
chains remain deferred; enabling one conversion does not support them.

## Fixed development extension and baseline

Append `eval/coverage-development/tagged-unit-classifiers.rs` to the unchanged
`tagged-units.rs`. Specify six new positives before admission: Unit::code,
Unit::is_end, byte_code, end_code, byte_is_end and end_is_end. The four closed
callers require constructor-to-helper-to-conversion/tag-test composition.

Four named negatives are unregistered_widen (unregistered u32-to-u64 From),
narrowing (TryFrom/Result/unwrap), reference_tag (Option reference payload), and
custom_tag (custom is_some calling unsupported black_box). The counterfeit
method itself and generated Clone remain excluded. Existing six positives and
five exclusions must retain their classifications.

Real Cargo 3.47 baseline assesses all 23 functions: six admitted, seventeen
excluded. The permanent baseline regression passes, independently checks admitted
documents, and requires complete capture, assessment, enumeration and evaluation.
The intended connected result is twelve admitted and eleven excluded, without
changing the original corpus or selecting examples by observed admission.

Activation requires original-source semantic expectations, complete composed
Cargo positives and independent document verification, negative/mutated evidence,
matching, unchanged natural selection deltas against 69/2582 admissions, frozen
replay, historical regressions, rstyle zero violations and patterns zero candidates.
Measure bounded performance against exact a236541; do not claim a general
performance guarantee from small controls. Extraction coverage remains separate
from rewrite safety, abstraction activation and held-out detector quality.

Audit validation: all four original-crate runtime oracle tests pass, including
exhaustive classifier payloads. The Cargo baseline regression passes. Workspace
formatting and whitespace checks pass; rstyle reports 2785 functions and zero
violations, and patterns reports zero candidates. Production Rust and the 3.47
policy remain unchanged; this audit does not replace full workspace activation
validation or establish a new natural admission/performance result.
