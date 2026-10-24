# Checksum-state roundtrips v1

Data-v3 **3.43.0** activates this source-selected family under
`rust.scalar-from-conversions` (55 capabilities). The frozen 60-function
inventory, identities and denominators remain fixed.

The selected natural family is adler2 2.0.1's read-only state roundtrip, in its
original library Cargo context on the pinned compiler. Assess default `std` and
`--no-default-features` separately. Source-selected functions in `src/lib.rs`:

| Function | Original byte span | SHA256 |
| --- | --- | --- |
| Adler32::from_checksum | 3368..3510 | f74ded98b21241c4fdb3406a15b5ab729200b070b96f83d126f87b85488f621d |
| Adler32::checksum | 3593..3684 | 353616f8226f0055ff141d0c2339f8587a25ae092e9d5f113d019f0d5343a8e0 |

These identities and expectations are fixed from published source before
admission assessment. Rustc-captured identities must be checked independently.
The complete unit includes Debug/Copy/Clone derives, Default/Hasher trait impls,
the external algo module, mutable methods, local constants and numeric trait
impls. The no_std context aliases `core` as `std`. Do not remove these neighbors
or infer purity for them to admit the selected methods.

## Independent scope and behavior

Both selected methods are intended in scope, along with their complete
constructor-result to getter composition. For every u32 input `sum`,
`Adler32::from_checksum(sum).checksum()` equals `sum`, including values that are
not reduced Adler components. The constructor preserves two distinct u16 halves;
the getter widens them without sign extension or truncation and combines them.
This contract does not validate an Adler checksum or certify checksum updates.

`eval/coverage-development/checksum-state.rs` fixes a separate development
selection. The five intended positives are State::from_checksum, State::checksum,
roundtrip, saved_input and widen. A saved constructed owner must survive later
input rebinding. The custom trait implementation/caller and floating conversion
are outside this bounded target; a custom From implementation must not acquire
the core scalar conversion's nonpanicking/effect-free summary. Preserve unknown,
unsupported and incomplete outcomes separately.

The independent runtime oracle checks 262144 whole-word inputs (all high halves,
four low-half boundaries), both original public methods and the development
composition, plus every u16 input to widening. Its expected roundtrip result is
the original input, not a reconstruction using the extraction implementation.

## Full blocker audit before evaluation

- Constructor casts, shifts and named Copy literals already have bounded
  contracts. Confirm their complete S09/source/import/derive boundary in this
  actual mixed unit before changing it; an otherwise supported body alone does
  not establish admission.
- The getter requires concrete u16-to-u32 core From calls. Existing primitive
  summaries cover a different closed method set; spelling `u32::from` alone is
  not a semantic or implementation identity. Inspect capture of the actual
  selected implementation and instantiated operand/result types before design.
- The existing direct-callee record already has an optional implementation
  identity, used for checked byte conversions. Reuse it if required; no new
  provenance table, source buffer or cache is justified.
- Getter field reads, distinct field paths, shared receiver adjustment and
  constructor/helper-result snapshots must compose with that call summary.
  Independently verify graph operands, memory locations, effects and matching.
- Unknowns before assessment are the exact source-owner/inline/derive/import
  gates in each build context and whether direct trait-call capture supplies the
  implementation evidence. Confirm those dependencies before redesigning S09.

Default/new and Hasher::finish are neighboring APIs, not selected positives:
they additionally require trait-method definition/call support and, for finish,
another conversion. write_slice/compute/adler32_slice add mutation and loop/SIMD
work. General traits, From implementations, floats, fallible/narrowing conversion,
aliases and checksum mutation remain outside this first roundtrip contract.

At activation, require independent numeric expectations, custom-implementation
and signature/identity/operand corruptions, composed real Cargo capture,
independent verification, strict admission and matching, unchanged-selection
new/lost admission measurement, historical regressions, and zero rstyle
violations/patterns candidates. Do not call this family supported from isolated
cast, field or conversion fixtures.

## Confirmed blockers and bounded implementation

The baseline 3.42 constructor fails exact lexical ownership because selected
function attributes reject builtin `#[inline]`. The getter has that same gate
and two missing `From` summaries. The constructor's casts/shift/literal, verified
Copy derive, mixed module/import census, getter field locations and existing
owned helper-result receiver snapshots already compose; no unit redesign is
needed. Permit only the builtin inline grammar: bare inline, inline(always),
inline(never). Keep every other source, macro, derive and drop gate.

Capture the selected implementation only for the instantiated u16-to-u32 direct
From call, reusing the existing optional callee implementation record. The
summary requires the pinned compiler, target, core crate hash, trait and numeric
implementation definition hashes, exact u16 operand/u32 result and original
primitive invocation with its ordered operand span. std and core display paths
bind the same target-specific core definitions. Bounded source forms are
`u32::from`, `core::primitive::u32::from` and `std::primitive::u32::from`.
UFCS, aliases, generic arguments, other numeric signatures and custom traits do
not receive this summary.

The reviewed pinned `core/src/convert/num.rs` digest is
`8dbf6c64681e2dee556e0844eef3e53d62a40df0280d25f469352870a401e034`.
Its numeric impl casts the u16 input to u32; the debug assertions compare constant
representable ranges and cannot fail for this instantiation. Thus this call is
lossless, returning, nonpanicking and memory-free; effects/partial returns of its
operand remain separate. The digest is tested without adding it to historical
serialized length-library evidence. No provenance table, extra source snapshot,
cache or production dependency is introduced. adler2 is a dev-only dependency
for the independent original-public-API oracle.

Independent document verification inventories every conversion, checks its
original operation/type/call form, exact summary/build/source/operands, absence
of invented memory effects or bindings, and the registered policy. Corruption
checks reject missing/wrong/nested implementation identities, wrong signatures,
changed callee/type/operands and policy downgrade. Alias/custom/float/bool calls
remain excluded. Source grammar checks reject unknown/malformed inline forms.

## Measured complete selections

| Unchanged selection | Captured functions | Baseline 3.42 admitted | Current 3.43 admitted | Lost |
| --- | ---: | ---: | ---: | ---: |
| Development corpus | 11 | 1 | 5 | 0 |
| Original adler2, std | 17 | 0 | 2 | 0 |
| Original adler2, no_std | 16 | 0 | 2 | 0 |

Complete roster comparisons preserve names, source spans/digests and build
contexts. The two natural source functions gain admission in **each** context;
this is two source functions and four context-specific admissions. Neighbors
and generated methods stay in the census and retain exclusions. Every capture,
assessment, pair enumeration and pair evaluation flag is complete. The
five development positives evaluate all ten comparisons. Strict bundle import,
independent document and producer verification and strict admission succeed for
the original methods; each context separately evaluates all six comparisons
against the development constructor/getter, with corresponding pairs scoring 1.
Contexts are evaluated separately because identical source graph keys across
std/no_std must not be fed to one matching universe as distinct functions.

The runtime oracle passes against the original API with both default std and
no-default-features: 262144 u32 words preserve the input and all 65536 u16
widenings agree with builtin cast expectations. A custom From deliberately
panics on zero and remains excluded. Real Cargo integration additionally checks
all three installed pinned targets with std/no_std, partial operand returns,
saved input values, qualified primitive calls and unsupported aliases/bool
conversions; unsupported signatures retain no selected-implementation record.

Remaining chains are unchanged: new/default and Hasher::finish need trait
method definitions/calls (finish also needs u32-to-u64 conversion); mutable
updates need mutation, looping and SIMD semantics. This family establishes
read-only state construction/getter composition only. Matching results do not
establish rewrite safety, abstraction activation or held-out detector quality.

## Performance check

A quiescent real-Cargo run alternates baseline **1ee3554** and current CLI/driver
pairs, discards one warmup pair and retains seven pairs for each unchanged
source selection. The already-supported six-function cyclic-prefix control
retains all source identities and outcomes: median elapsed **0.66 / 0.63 s**
(baseline/current ranges **0.61–0.66 / 0.61–0.65 s**) and median per-run peak RSS
**111548 / 111796 KiB**. These overlapping bounded measurements show no material
regression on the unchanged control; they are not a general speedup claim.

The unchanged checksum corpus takes **0.36 / 0.41 s** (ranges **0.35–0.38 /
0.39–0.43 s**) and **118552 / 119964 KiB** median per-run peak RSS. Current does
more work: five admitted functions and ten evaluated pairs versus one admission
and no pair comparisons. This is a measured activation cost, not an equal-work
performance regression or whole-library performance guarantee. All capture,
assessment and matching-completion flags and source identities are checked on
every run. Raw alternating records are retained in
`/tmp/rstyle-checksum-state-performance-results.json` for this development run.

## Frozen replay and wider-unit observations

Canonical replay `run-dfcyj99b` preserves all 60 selected identities, outcomes,
context counts and denominators against `run-lqdkmx_1`: **20 admitted / 33 excluded
/ 7 inactive**. The three complete captured unit rosters also retain all
identities and semantic build contexts: lz4_flex **150, 7→7 admitted**; time-core
**226, 8→8**; regex-syntax **956, 14→19**, with no losses. The five additional
regex-syntax admissions are `hir::LookSet::{empty, full, subtract, union,
intersect}`, previously blocked by exact inline ownership. These are incidental
wider-unit observations, not a retrospectively selected target or a claim of a
complete LookSet API family. All neighbors remain captured. Canonical replay
independently verifies **68 documents** across the full units (14/38/16), while
the frozen denominator remains 60. Full roster delta details are retained in
`/tmp/rstyle-checksum-state-full-frozen-delta.json` for this run.

## Final validation

The final workspace nextest CI run passes **846 tests**, with five existing
skips; workspace doctests pass. All-target workspace clippy with warnings denied,
fmt and diff checks pass. `cargo rstyle --format json` reports **2686 functions,
zero violations**; `cargo rstyle patterns --format json --fail-on-candidates`
reports **zero candidates**, without new suppressions, ignores or threshold
changes. Fresh final Cargo captures and strict natural/development comparisons
repeat the measurements above after capture was narrowed to the supported
numeric signature. Cross-target tests compile/capture/verify macOS evidence on
Linux; they do not execute macOS binaries. The full Rust roadmap remains open.
