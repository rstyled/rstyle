# Borrowed-result chain: source audit v1

Current: **Data3.69 connects the original borrowed-result chain**; integrated
validation is recorded in the root roadmap. The3.59 audit below is historical;
its nominal/callable prerequisites were delivered in3.67–3.68.

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

The3.66/3.67 processes hit1200-second limits, but subsequent streaming recovery
validated complete2582-record reports with capture/assessment complete:114→120
admissions, zero losses. Full-library matching remains incomplete.

## Data-v3 3.68: original scalar/Span getter composition

The [source selection](../../eval/coverage-development/input-owner-selection.json)
now also fixes original `get_span`, `start`, `end` and repeated-call `is_done`:
return the configured span/endpoints and exactly `start > end`, including empty
and terminated spans. They share the complete owner and selected Copy dependency
boundary with `get_earliest`; no stored-slice read is needed.

A dependency may use the narrower boundary only with exact complete declaration,
field/source resolution and pinned builtin Copy evidence, and only when every
captured occurrence stays inside a distinct shared struct owner. Direct values,
references, construction, field loads and callable uses of that dependency do
not qualify. Missing type evidence grants nothing. Selected attributes remain
checked; the actual function's owning scope and all ordinary operation, helper,
effect and origin checks remain closed. This proves no macro-generated method
semantics and gives no permission to execute neighboring implementations.
The `rust.passive-copy-dependencies` capability is independently required only
when this narrower proof is used; historical policies cannot certify it.

Fresh complete3.67 Cargo evidence recertified under3.68 admits all five original
getters: **120→132/2582**,12 gains/0 losses, identical source/build identities.
All132 roots strict-verify; no driver grammar changed. Incidental gains were
source-audited: sparse State range/accelerator, Transition::matches_byte and four
Match/HalfMatch getters. Independent original oracle:1128 states/11280 observations.
All10 selected comparisons complete; get_span/get_earliest unexpectedly passes
its predeclared cross-result negative (7/8 hold). Keep that detector limitation
separate from extraction support; no scoring/threshold changes.

Retained paths and exact build/source identities are in the selection; the root
roadmap records the frozen replay and integrated gates.
Stored-reference helper-result composition remains the next target.


## Data-v3 3.69: stored-slice helper composition

`rust.stored-slice-helper-results` verifies a shared slice of supported integer/bool
scalars from exact formal struct fields. Slot reads retain `[SharedReferent, fields...]`;
returned referents append a distinct final `SharedReferent`. Every finite result
alternative requires an independently derived exact slot read. Existing saved
actual-argument substitution preserves interior referents. No array-view panic
or partial-flow permission is inherited; owner, lifetime, drop and effect checks
remain required. Missing reads and historical-policy downgrades reject.

The [source selection](../../eval/coverage-development/borrowed-result-selection.json)
fixes the complete original `haystack → is_char_boundary → utf8::is_boundary`
chain, source/build identities and independent expectations. All three strictly
admit. The root roadmap records measured gains and validation; broader borrowed
aggregates and nested-reference helper results remain outside this slice.
