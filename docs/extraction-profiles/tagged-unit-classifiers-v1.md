# Tagged byte-unit classifiers

Data-v3 3.48 connects the next selected family after Data-v3 3.47. Use the complete
default-feature Cargo library of regex-automata 0.4.18, retaining all modules,
imports, derives and neighbors.
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

The natural selection uses nightly-2026-09-30 (rustc commit
5c543b0b8c73c7b72bc8284ced4fb22ead15734d), aarch64-unknown-linux-gnu,
and the default debug Cargo library context. Primitive conversion/tag authority
is also checked on x86_64-unknown-linux-gnu and aarch64-apple-darwin under the same
pinned compiler and 64-bit width. The original-source runtime oracle executes on
ARM Linux; cross-target capture is not native runtime evidence.

## Complete known blocker chain and bounded contract

The source-first 3.47 natural roster excluded both methods. Its simultaneous
blockers defined the following contract. Existing enum declaration, positive Copy,
variant/payload and scalar Option helper evidence remains required.

1. as_usize already captured the selected u8-to-usize From implementation.
   Its u16-to-usize call captured only the trait definition, without the selected
   implementation. Extend the existing scalar conversion capture and exact
   pinned implementation registry for the three supported compiler/target
   contexts; do not infer a contract from the From spelling or trait identity.
   Verify source correspondence and mathematical widening independently.
2. is_eoi calls the now-supported as_eoi helper, then core Option::is_some.
   Existing method-receiver capture already records the owned Option<u16>
   temporary, its shared autoref, and the exact &Option<u16>-to-bool signature.
   Reuse that evidence. A registered core source/semantic contract and source
   correspondence permission were absent. A similarly named custom method
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

Baseline unknowns, resolved by the connected implementation below: exact selected
u16 conversion identity on all three pinned targets, required tag-test document shape and helper
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

Real Cargo 3.47 baseline assessed all 23 functions: six admitted, seventeen
excluded. The source-first regression recorded that baseline, independently checked
admitted documents, and required complete capture, assessment, enumeration and
evaluation. The declared connected expectation was twelve admitted and eleven
excluded, without
changing the original corpus or selecting examples by observed admission.

Activation requires original-source semantic expectations, complete composed
Cargo positives and independent document verification, negative/mutated evidence,
matching, unchanged natural selection deltas against 69/2582 admissions, frozen
replay, historical regressions, rstyle zero violations and patterns zero candidates.
Measure bounded performance against exact a236541; do not claim a general
performance guarantee from small controls. Extraction coverage remains separate
from rewrite safety, abstraction activation and held-out detector quality.

Source-first audit validation passed all four original-crate runtime oracle tests,
including exhaustive classifier payloads, and the Cargo baseline regression.
Formatting and whitespace passed; rstyle reported 2785 functions and zero
violations, and patterns zero candidates. Production Rust and the 3.47 policy
were unchanged at that boundary; no new natural gain was claimed by that audit.

## Connected implementation and validation

The implementation extends the existing selected From capture and
three-target core definition registries for u16-to-usize. The existing library
summary, receiver and document-verification path handles is_some, requiring
an owned scalar Option and exactly one shared autoref to its checked signature.
It tests the tag through the registered `rstyle.data.option.is-some/1` summary;
no payload load, ABI representation, new provenance table or cache is introduced.
Both core and std compiler display paths retain the exact resolved core hash.
The Data-v3 3.48 policy gates both operations; old document policy
labels cannot authorize them.

The focused Cargo regressions pass: all twelve intended development
positives admit, all eleven expected exclusions remain, and independently checked
complete documents compose through constructor/helper callers and matching.
Primitive conversion and tag-test admission also pass on all three installed
pinned targets. Six mutations of summary behavior, value operands and document
policy are rejected by independent verification and strict admission. An exact
pinned option.rs digest and exhaustive u16 tag/widening runtime test supplement
the complete-original-crate oracle.

Validation passes all 880 workspace nextest tests with five existing
skips, workspace all-target Clippy with warnings denied, doctests, formatting and
whitespace. Rstyle reports zero violations and patterns zero candidates.

Against isolated a236541, one warmup and seven alternating quiescent pairs per
workload retain all source identities. Unchanged byte-window controls keep eleven
admissions and exactly equal matching: baseline/current median .87/.87s (both
ranges .85–.90), median RSS 123504/123508 KiB. Cyclic controls keep four admissions
and exactly equal matching: .64/.64s (ranges .61–.66/.63–.67), RSS 112016/112020 KiB.
The fixed 23-function combined corpus gains six admissions (six to twelve) with complete
matching: .73/.84s (ranges .71–.77/.81–.86), RSS 123780/125504 KiB. Extra classifier
work includes producing, independently verifying and comparing newly admitted
functions. These bounded observations establish neither broad performance
invariance nor detector quality.

## Additional measured natural gain

The unchanged full 2582-function roster gains ByteClasses::get_by_unit as well as
the two declared targets. This extra function was discovered by the full delta;
it was not part of the predeclared two-method selection. Its original identity is
`src/util/alphabet.rs` bytes 11998..12190, SHA-256
`c692de960c7c46267c5f6f7562f85dc065bcac35375dbd15a11a5d7d70509cf2`.
Its full source matches UnitKind: byte payloads call the existing ByteClasses::get
helper and widen the retrieved class, while EOI payloads widen unchanged. Thus the
new conversion closes a real enum-match/getter/array-read composition too.

The independently specified runtime expectation is mapping[byte] as a mathematical
usize for byte tags, and the unchanged full u16 payload for EOI tags, with no array
read in the sentinel arm. The complete-original-crate oracle now binds this exact
identity and checks all bytes and all 65536 sentinel payloads against constant-zero
and reversed-byte maps. All five oracle tests pass. No original method or
surrounding declaration is replaced. Independent verification passes for all 72
natural admissions and twelve development admissions. All 153 comparisons among six original and twelve
development functions complete with unchanged matching defaults, no incomplete
evaluations and no diagnostics.

## Unchanged natural selection results

The complete default-feature regex-automata library retains all 2582 exact source
identities and its build context, including compiler/target, features and cfgs.
Admissions increase **69 to 72**, with exactly **three gains and zero losses**:
Unit::as_usize, Unit::is_eoi and ByteClasses::get_by_unit. Capture and assessment
complete with no unknown/failed outcomes. Full pair enumeration is explicitly
budgeted out (2556 possible pairs); it is distinct from the completed 153 selected
comparisons above. Historical reports are preserved.

The wider Unit/ByteClasses family remains incomplete. Unit::eoi still combines
owned source/HIR observation disagreement, assertion-format macro deferral,
TryFrom/Result/unwrap authority/types/effects and helper-link gaps. is_byte still
combines map_or authority with closure ownership/body/captures; is_word_byte needs
map_or callback identity/body and a retained binding read. ByteClasses::eoi also
needs checked_sub/Option unwrap and the unsupported Unit::eoi helper chain. These
functions remain excluded in the new full roster. The new scalar conversion or
tag summary grants none of those contracts. No abstraction activation, rewrite
safety or held-out quality claim follows from these extraction results.

Frozen replay retains all 60 exact selected identities, denominators and outcomes:
20 admitted /33 excluded /7 inactive, with 70 verified documents. Complete package
source identities, build contexts and outcomes are unchanged against 3.47:
lz4_flex 150 functions /7 admitted, regex-syntax 956 /20, time-core 226 /8.
No historical report or frozen selection is modified.
