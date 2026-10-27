# Saved checked consumption: first-wave source audit

Status: **blocked before activation**, 2026-10-21. P08.B source audit only;
no production edits, capture, admission measurement or support claim.

The existing immediate `usize::checked_sub(...).unwrap()` contract does not
justify arbitrary saved Option unwraps. `bitset_cardinality::resolve` explicitly
requires the unwrap child itself to be the checked producer. Existing saved
slice-get and constructed scalar Options already work and are not this task.
`end-of-input.rs::saved_result` is a saved **conversion Result**, so it belongs
to the subsequent Result increment, not an Option acceptance target.

## Natural candidate and readiness decision

A bounded source search of the four current natural packages found immediate
checked-sub unwraps, direct match/if-let consumption and unrelated complicated
callers, but no tractable saved checked-sub local followed by unwrap. This is
not a claim that no such Rust function exists. A bounded local-registry search
located a genuine saved producer in zerovec **0.11.8**:

- `src/varzerovec/components.rs`,
  `VarZeroVecComponents::parse_bytes_with_length`, line 266, UTF-8 byte range
  `[9859,11325)` starting at `pub fn` and ending after the body.
- Source file SHA-256:
  `1b2e859ab718341471f58b2ffee05d396cbf0154ab9d521f4cbc8fa452e12f47`.
- Function SHA-256:
  `71bbaa1f60bed25002ef670ec04b574ac5cfa49748c5226f7f4eef7c56434949`.
- Normalized registry `Cargo.toml` SHA-256:
  `11216eeba19169e8bec8d703c0c452367747e20e1ca319fc48feda7f0b1e8fba`.
- Original context: registry package `zerovec@0.11.8`, edition 2021, library,
  non-test; no default feature is declared. Candidate future capture: pinned
  nightly-2026-09-30, aarch64-unknown-linux-gnu, Cargo debug, no enabled package
  features. This is a source-selected proposed context, **not a captured build
  identity**. Exact Cargo unit/instance identity remains unknown until capture.

The source saves `let len_minus_one = len.checked_sub(1);` then consumes it with
`let Some(len_minus_one) = len_minus_one else { return Ok(...); };`.
It is **u32**, not the currently contracted usize operation, and the complete
original is generic over `T: VarULE + ?Sized` and `F: VarZeroVecFormat`.
It is unsuitable for the bounded bitset-cardinality-only increment.

## Complete known blocker graph

Source/module/derive ownership and generic impl eligibility
→ concrete T/F instance and associated `F::Index::SIZE`/trait resolution
→ exact u32 checked-sub authority
→ saved Option reaching value and let-else tag/payload control
→ Result construction/early return and guarded residual `?`
→ pinned u32/usize conversions, Option::ok/and_then/ok_or and checked_mul
→ closure target/capture semantics
→ scalar slice RangeTo/RangeFrom checked access and saved view origins
→ reference-bearing generic aggregate construction/return
→ `check_indices_and_things` helper and its entire transitive validator chain.

The validator chain itself is not frozen or audited here: its concrete-instance
selection and transitive requirements are explicitly unknown. No claim of a
complete *resolved* dependency graph follows from this source audit. The known
hard dependencies already exceed P08.B's allowed write set. The neighboring
unsafe constructor has the same saved Option but adds unsafe contracts and is
not a cheaper substitute. regex-automata `util/empty.rs` directly matches a
checked-sub but also requires mutable Input, callbacks, loops and Result `?`;
it is neither saved-local consumption nor a suitable replacement.

## Independent semantic expectations and future bounded contract

Before measuring any future original instance, independently specify:

- Checked subtraction has Some payload `n-amount` iff `n >= amount`, otherwise
  None; test zero, equality, one-below, maxima and wide boundary values.
- Saving the result must preserve its original operands despite rebinding;
  aliases, shadowing, branch joins and helper transport cannot substitute later
  values. Unwrap on None remains possible panic, never invented totality.
- For this original, len=0 returns an empty successful view independently of
  input bytes. Positive nonzero buffers must satisfy the documented index and
  element validity contract for a **fixed concrete T/F**, not a fabricated
  generic guarantee. Invalid index tables, metadata truncation and invalid
  element encodings must return the declared error. Runtime expectations require
  a separately written encoding oracle once T/F are selected.
- Historical document policies must reject new saved-producer consumption;
  wrong source/type/authority, stale producer values and removed payload guards
  must fail independent verification after ordinary integrity seals are rebuilt.

A future narrow saved-Option increment may reuse existing Option value and
reaching-definition proof with exact usize checked-sub/unwrap authority, Copy
locals, saved snapshots and partial normal flow. It must first find an unchanged
natural original whose **whole** blocker chain fits that contract. Do not activate
it merely because the constructed `saved_option` regression can be admitted.

## Ownership and dependency frontier

No implementation is ready; no shared interface change is requested now.
Proposed exclusive future write set, conditional on readiness:
`crates/rstyle-pdg/src/bitset_cardinality.rs`, its existing subtree plus a dedicated
saved-consumption leaf, and dedicated CLI tests. Lead owns registration, policy,
shared Option/eligibility/producer seams and all commits. If generic/let-else or
Result proof becomes the selected target, assign those seams separately before
this worker writes anything. Replan P08.B toward a source-ready concrete Option
if-let or Result family instead of stacking an unmeasured foundation here.
