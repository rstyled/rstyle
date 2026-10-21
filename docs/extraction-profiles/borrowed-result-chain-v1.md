# Borrowed-result chain: source audit v1

Status: **blocked; not activated**. Audit 2026-10-21 against the retained Data-v3
3.59 roster and current implementation. P06.C depends on bounded P05 nominal and
callable ownership proof; this document does not promise an immediate gain.

## Original selection and build

Select the smaller complete original chain `Input::haystack` →
`Input::is_char_boundary` → `util::utf8::is_boundary`, rather than coupling the
first increment to by-value `Span` helper results. The middle function is a
scalar wrapper/consumer of the single borrowed result. It returns no reference.
No original function is rewritten, and no new frozen-inventory member is added.

Package: `path+file:///home/dev/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/regex-automata-0.4.18#regex-automata@0.4.18`.
Unit: `regex_automata:test=false`, `src/lib.rs`, edition 2021, default features;
nightly-2026-09-30 / rustc `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`,
aarch64-unknown-linux-gnu, debug/unoptimized, overflow checks, panic=unwind.
The exact expanded features/cfg/invocation remain in the retained selected facts
`/tmp/rstyle-helper-index-natural-selected.json` (same owning compilation unit).

Whole-file SHA-256:

- `src/util/search.rs`: `74157bd77eb6f80e8ab5952ef21e8fb2d3452c1817ec671a7a3574fbddae1ae7`
- `src/util/utf8.rs`: `a02a559f0ec4013aa8bbc1a2d8717cfa0c82ed81577a4366fbbd8ef8660264fc`
- `Cargo.toml`: `25d528e0bd6dd67c78462d3c7a426698b5d6e702052b25daf7599c0a211315b6`

| Original function | Source byte span | Function SHA-256 | Recorded 3.59 |
| --- | --- | --- | --- |
| `util::search::Input::<'h>::haystack` | 24270..24334 | `9c29bc68d97eff53c40572a6f600a1f421c83f314403784b48c8e724e3889bb8` | excluded |
| `util::search::Input::<'h>::is_char_boundary` | 29204..29316 | `0f83c1f1788b7fe1b5a04b8c67da890aa5239dbbf1cf3a5a9326887348f74e9d` | excluded |
| `util::utf8::is_boundary` | 3947..4747 | `e676b73906d493177219bcc72dbbb29fce630d0c07570e1ec1b7c3378b10c826` | admitted |

## Complete known dependency frontier

1. **P05 callable ownership:** `supported_inherent_member` rejects all enclosing
   impl generics, including `impl<'h> Input<'h>`. A lifetime-only exception must
   prove the exact owner/substitution; removing the generic check is insufficient.
2. **P05 complete borrowed owner:** `Input<'h>` contains `&'h [u8]`, `Span`,
   `Anchored`, and bool. Selected declaration checks reject nominal lifetimes
   and shared-reference fields. The borrowed-owner permission only accepts
   scalars, scalar arrays, and separately checked Copy struct children.
   Preserve the unused fields and derive/neighboring-item proof.
3. **Selected enum child:** `Anchored::{No,Yes,Pattern(PatternID)}` combines unit
   and nominal payload variants. This exceeds the existing scalar enum contract;
   3.60 fieldless flag enums alone do not supply it. `PatternID(SmallIndex)` and
   `SmallIndex(u32)` are ordinary transparent tuple structs with builtin Copy
   derives. Nearby `index_type_impls!` invocations generate methods, **not these
   declarations**; general macro support is not yet a demonstrated prerequisite
   for reading the untouched owner. Exact unused-child proof must be decided.
4. **P06 stored-reference load:** returning `self.haystack` reads the stored
   reference field and obtains that field's referent, distinct from the address
   of the reference slot. Existing `helper_views::verify_origin` permits only
   scalar-array field → slice views. It cannot certify this result.
5. **P06 substitution/composition:** callee result origin must substitute the
   saved actual receiver, including nested calls, while read summaries retain
   the slot read separately. Reuse verified unit links and existing projections;
   do not invent whole-owner origin or merge read effects into result origins.
6. **Consumer:** `utf8::is_boundary` is already admitted through checked get,
   payload binding and byte comparisons. Wrapper calls must preserve both its
   conditional byte read and the getter's reference-slot read. Checked access
   does not imply an unconditional byte read or a panic for out-of-range offsets.

Recorded first rejections are receiver ownership/primitive interface (getter),
plus unsupported reference-valued operation/helper summary (wrapper). These are
not an exhaustive measurement of downstream activation failures. Unknowns:
exact current HIR stored-reference path/adjustment, lifetime substitution transport,
selected non-scalar enum child validation, and whether existing builtin dereference
load origins already suffice. Audit these before changing the shared vocabulary.
No new capture or readiness claim was made in this audit.

Alternatives inspected: `regex-syntax::Utf8Sequence::as_slice → len` adds mixed
payload enum/ref-pattern, aggregate slice element and `slice::from_ref` contracts;
`regex-automata::dfa::sparse::Transitions::sparse → memory_usage` adds generic
AsRef dispatch. Neither is a cheaper ready P06-only family. Input's smaller
haystack chain remains the explicit downstream acceptance target.

## Bounded contract and independent expectations

Read-only same-unit, lifetime-only owner; one saved shared slice result from one
stored field. No owned Input construction/consumption/drop, mutable receivers,
local escaping borrows, aggregate reference results, unknown dispatch, recursive
helpers or new index scopes. Constructors may be used by the runtime oracle;
that does not imply constructor extraction support.

Before admission measurement, fix expectations: getter preserves pointer and
length; changing span, anchored mode or earliest does not change its haystack.
For valid UTF-8, wrapper result equals the language string boundary predicate:
empty offsets 0/1 → true/false; `☃` offsets 0..4 → true,false,false,true,false;
ASCII and mixed multibyte text follow exact codepoint starts/end. For all offsets
past end, including usize::MAX, return false without panic. For arbitrary bytes,
separately test the implementation's byte classifier formula without claiming
UTF-8 validity guarantees. Two distinct same-length haystacks must retain distinct
origins even when content agrees. Saved receiver/actual rebindings must not retarget
returned origins. Tampered field identity, omitted slot read, substituted referent,
forged lifetime/build owner or historical policy downgrade must fail verification.
Require unchanged original Cargo capture, strict replay and complete selected
matching; a diagnostic copied fixture is insufficient natural acceptance.

## Proposed ownership and readiness gate

C exclusively owns `helper_references.rs`, `helper_views.rs`, a dedicated
`variant_capture/borrowed_result_chain.rs` test leaf and runtime oracle **after
lead approval**. Lead owns P05 selected declaration/lifetime impl changes, shared
origin/field location seams, eligibility, policy/admission and test registration.
No production patch is ready until those prerequisites and exact stored-reference
path semantics are agreed. No sequence-location, payload or 3.59 indexed-helper
scope changes. Estimated uncertainty is high in owner proof, moderate in the
bounded helper leaf; the natural capture costs the existing full owning-unit run.

## Data-v3 3.67 owner foundation

The separate [Input selection](../../eval/coverage-development/input-owner-selection.json)
targets original `get_earliest` before borrowed results. Mandatory mixed-enum
`Unit | SingleTuple` shapes retain all Anchored variants and nested Copy wrappers;
their exact transparent repr, source, derives and fields are checked. Complete
borrowed owners retain stored scalar slices. Lifetime-only impl/callee ownership
requires exact lifetime mapping, no method generics and shared receivers.

The complete diagnostic owner admits its bool getter and both inferred call
forms across three targets × std/no_std, with strict verification and matching.
Owned use, mixed-enum operations, specialized lifetimes, unknown effects and
stored-slice reads in these new mixed owners remain closed. Historical Holder
reads and lifetime families remain supported. The diagnostic substitutes a builtin
Debug derive for PatternID's generated Debug impl; it is not original acceptance.
The independent original-library oracle passes 1128 states /5640 observations.

Original admission remains pending. The 3.66 full-package baseline compiled, then
hit its 1200-second wall-clock limit during assessment; no complete roster or delta
was produced. Retain `/tmp/rstyle-input-owner-baseline-target/rstyle/clones/run-KZh2xL`
as incomplete evidence. The source audit identifies neighboring `index_type_impls!`
invocations in the selected Copy wrapper's module as a remaining lexical boundary.
Next establish precise dependency/effect proof there, then measure the complete
original getter and callers before extending stored-slot/result-origin composition.
