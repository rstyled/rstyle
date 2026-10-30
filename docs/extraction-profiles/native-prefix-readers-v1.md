# Native prefix readers: source-selected contract

This target was declared at Data-v3 3.43.0 before admission assessment and is
activated by Data-v3 3.44.0 after the validation below. The frozen 60-function
inventory, identities, denominators and historical outcomes remain unchanged.

The selected natural family is regex-automata 0.4.18's three native unsigned
prefix readers in `src/util/wire.rs`, in the original complete default-feature
Cargo library context on the pinned compiler. All three are expected in scope.
Do not remove neighboring error types, macros, unsafe items, derives or modules.

| Function | Original byte span | SHA256 |
| --- | --- | --- |
| util::wire::read_u16 | 29612..29761 | 852fd0e757528a8591eb6c9f3ddf1eb51665e7ad8aff97735f6d950e750d4d4c |
| util::wire::read_u32 | 30069..30218 | 302aaf96df8e3b521df5bcc491decff3b23f403f71457870b03a9f54ce6eb865 |
| util::wire::read_u128 | 30359..30513 | cd2740158fc9036a45584ac18e0dc3fa3d3fc0c2c45401d9d05915ad820a5797 |

## Independent behavior and scope

For widths 2, 4 and 16, a reader returns the native-endian unsigned integer
formed from exactly that many leading bytes. Bytes after the prefix do not
affect the value. Every shorter input panics; bounds failure and unwrap failure
must retain their normal-completion conditions. Do not certify the functions as
nonpanicking or infer that a range spelling discharges the Result check.

`tests/fixtures/function-families/native-prefix-readers.rs` fixes 11 function identities
before assessment. Also assess the unchanged file as an ordinary external
`reader` module in a separate `#![no_std]` crate; those names have the `reader::`
prefix. Do not use include expansion or rewritten module paths. Eight intended positives are read_u16/read_u32/read_u128,
native_u16, choose, selected_u16, saved_u16 and snapshot_u16. Saved views and a
base evaluated before endpoint rebinding must keep the old slice, while the
helper-selected base follows the chosen input. The three retained negatives are
suffix (outside this prefix-only target), signed (outside these unsigned native
summaries) and caller_u16 (partial/panicking helper-summary transport is not
established). A valid runtime result does not imply pipeline admission.

An independent oracle specifies values by positional byte weights, without
calling the conversion under test to compute expected results. It checks all
65536 two-byte words, single-bit patterns for 32/128-bit words, extra trailing
bytes, every short length, selected inputs and saved/rebound bases. The original
private function bodies are copied byte-for-byte from the recorded source into
a runtime-only harness; their Cargo/unit/source proof remains a separate gate.

## Full blocker chain to investigate

1. Prefix RangeTo syntax (`..end`) has no source lower operand. Represent the
   language-defined zero origin explicitly; absence of capture data must never
   silently become zero. Existing exclusive two-endpoint views cannot certify
   this by pretending a lower HIR expression exists. Check base-before-endpoint
   evaluation, original RangeTo/Index authority, unsized-place receiver
   adjustments, bounds and saved view origins together.
2. `size_of::<u16/u32/u128>()` is an original direct expression here, not the
   currently bounded function-local usize-size constant. Establish its actual
   callee/type/instantiation evidence and pinned result without fabricated
   literal source or general const-evaluation permission.
3. Concrete copied slice-array Result observations, TryInto implementation,
   unwrap and native conversion currently have bounded four-byte/borrowed-eight
   contracts. Verify complete type/receiver/source evidence for 2/4/16-byte
   copies and u16/u32/u128 native decodes; do not erase the intermediate Result,
   conditional reads, exceptional completion or array operand dependencies.
4. Confirm complete unit/source/import ownership and pinned std/core display
   paths. Audit all known source and graph blockers, not only the first
   eligibility rejection. Keep unsupported, unknown and incomplete distinct.

Reuse existing callee implementation records, source snapshots, byte-window
locations, summaries and document checks where sound. No new provenance table,
source buffer/cache or broad trait authority is justified. General open/inclusive
ranges, signed/float conversions, mutable views, general Result patterns/?,
looping or panicking helper summaries and unsafe extraction are not implied.

Activation requires original Cargo capture, independent document and producer
verification, strict admission and matching for all three complete readers;
unchanged-selection new/lost counts, source/identity/type/operand/bounds
corruptions, historical regressions, an unchanged performance control, and zero
rstyle violations/pattern candidates. This target is not supported by separate
fixtures for prefix slicing, constant size or array conversion alone.

## Baseline audit (no activation)

Both separate development Cargo contexts capture/assess all **11 functions**,
with **1 admitted /10 excluded /0 unknown** and complete enumeration/evaluation
of their empty pair set. Only choose is admitted; the seven other intended positives and three retained
negatives remain excluded, and all function source identities are fixed. There are no
new or lost admissions from this audit and no production implementation change.
The source file was made compilable before its first assessment by giving the
saved rebind a common explicit lifetime; it is fixed from that assessment on.

Current 3.43 classification of the three source-identified functions in the
retained complete 3.42 original Cargo unit (2582 functions) rejects all three.
Their original spans/hashes match the table. This is a selected-function audit
of retained historical capture, **not** a fresh complete 3.43 original-library
CLI report or a new whole-unit admission count. Before activation, fresh
original Cargo capture and complete baseline/current roster comparison are
still required. The prior source census is retained; no neighbors were removed.

The full source/facts/code inspection establishes these blocker combinations:

| Reader | Confirmed connected blockers |
| --- | --- |
| u16 | RangeTo prefix representation/authority and bounds; core display paths; unresolved size_of call/instantiation; two-byte Result/unwrap/copy contract; native u16 summary. |
| u32 | RangeTo prefix representation/authority and bounds; core display paths; unresolved size_of call/instantiation. The four-byte Result/copy/unwrap/native machinery already exists. |
| u128 | RangeTo prefix representation/authority and bounds; core display paths; unresolved size_of call/instantiation; sixteen-byte Result/unwrap/copy contract; native u128 summary. |

The original crate is no_std internally even in its default-feature library
context. Captured callees/selected blanket implementation use core::convert,
core::result and core::ops paths with the same compiler/core definition identity
as the std display forms. The no_std development context independently
reproduces those extra summary failures. Do not normalize arbitrary strings or
relax crate/hash/definition checks to address this difference.

The current capture **already observes** copied slice-array Result lengths 2 and
16, their array payload types and method signatures. Add only their bounded
semantic contracts; do not introduce redundant type observation tables.
The nested size_of expression, by contrast, is an expr_call with usize output
but **no callee resolution or instantiated type-argument authority**. Its absence
is hidden by early rejection of RangeTo, so the u32 development reader's short
reason list does not prove size_of support. No missing-data-as-zero or
source-spelling-only intrinsic summary is justified. Determine the smallest
sound reuse of existing typed source/callee evidence before capture changes.

The four runtime oracle tests pass with the pinned compiler and overflow checks:
all 65536 two-byte words, every bit of 32/128-bit inputs, uniform boundary
patterns and trailing bytes, all short lengths, both helper choices and saved
bases after rebinding (including an empty replacement). The original private
bodies are verified byte-for-byte before building the temporary runtime harness.
Run from a shell with rustc on PATH:

```sh
python3 tests/fixtures/function-families/native-prefix-readers-oracle.py \
  /path/to/regex-automata-0.4.18/src/util/wire.rs
```

This establishes independent expectations and a complete known dependency
chain, with the exact generic-argument proof and composed lowering still to be
resolved. No family admission, rewritten source-prefix certificate, complete
exceptional-flow model, performance gain or detector-quality claim follows.

Audit validation: both real-Cargo development reports are complete; strict bundle
import, independent documents, producer verification and admission pass for their
choose controls. All four standalone runtime oracle tests pass; Python syntax,
workspace fmt and diff checks pass. Rstyle remains **2686 functions /0 violations**
and patterns **0 candidates**. The extraction engine, frozen selection, scoring,
thresholds, suppressions and ignore ledger are unchanged. A workspace test rerun
is not claimed for these evaluation-only additions; the preceding production
milestone's 846-test result remains recorded separately.

## Bounded implementation

The Data-v3 3.44.0 contract connects the complete chain rather than
certifying a supported prefix of an otherwise unsupported function:

- `..end` on a shared scalar slice has an explicit language-defined zero start,
  one actual endpoint operand and a retained `end <= length` normal-completion
  requirement. Base evaluation precedes endpoint evaluation. No fake lower-bound
  expression or literal is inserted, and suffix/inclusive/mutable/array views
  are not added by this contract.
- Original direct `size_of::<u16/u32/u128>()` calls require the pinned core callee,
  actual instantiated primitive argument, zero-input usize signature and exact
  original syntax. Unrenamed `size_of` imports and explicit core/std mem paths are supported;
  aliases for the type argument, custom callees, aggregate/general size calls
  and generic constant evaluation remain outside the contract. The call stays a
  call with a checked size annotation; it is not replaced by fabricated source.
- The existing concrete slice-array Result observation supplies lengths 2/4/16;
  the existing selected blanket TryInto implementation, receiver adjustments,
  type checks and conditional byte-window reads remain mandatory. Both the
  location payload type and read-window length are the actual array length.
  Unwrap still requires Ok for normal completion and still omits exceptional
  flow; conversion and prefix syntax do not prove it infallible.
- Native u16/u32/u128 conversion retains exact array dependencies and unsigned
  positional-byte significance on the three pinned little-endian CI targets.
  For byte conversions, only the three known core/std display alternatives are
  mapped to their exact existing authority keys; compiler, target, crate and definition hashes and the
  selected implementation must still agree. No general string normalization or
  trait purity inference is introduced.

The only new capture datum is the actual scalar size argument nested with the
existing callee record. Existing source storage, type observations, return-origin
transport, summaries and location keys are reused. There is no new provenance
index, source snapshot copy or cache. Historical policies reject new prefix,
size and extended native constructs independently of the generated documents'
other constructs.

Source review found an initially retained four-byte location payload label for
new two/sixteen-byte copies despite correct window lengths. It is corrected, and
independent fixed-length/type assertions for all three readers now cover this
composition. Altering the location payload to four bytes is explicitly rejected
by document verification, producer verification and strict admission.

Both unchanged 11-function development contexts now have **8 admitted /3
excluded /0 unknown**, versus **1 /10 /0** at baseline: seven new admissions and
no losses in each separate context. All eight positives compose through actual
Cargo capture and strict verification, with all **28 comparisons evaluated** in
each context on each installed pinned target. The signed, suffix and panicking
helper negatives remain excluded. Original-library, frozen-inventory and performance validation are recorded
separately below.

### Fresh complete baseline

The static 68c300c /Data-v3 3.43.0 binaries have now captured the original
regex-automata 0.4.18 default-feature library again, preserving all **2582
functions**: **9 admitted /2573 excluded /0 unknown**. All three selected reader
spans/hashes match the declaration and remain excluded. This supersedes the need
to infer a baseline from the older selected-function audit, without changing or
reinterpreting its historical results.

The complete saved `run-373MDW/report.json` was streamed to a compact roster and
validated through its closing record. Capture and assessment are complete;
matching is intentionally incomplete under the explicit zero-pair budget (36
possible pairs), not silently empty or successful. After that validation, the
redundant JSON stdout rendering was interrupted to avoid retaining a second
8 GB report string. The process's exit 130 is recorded as interrupted, not a
normal successful CLI exit. Its wall time is not used as a performance comparison.

### Complete natural-unit and selected-family evidence

The fresh original-library current report `run-aE1dkq` completes capture and
assessment of the same **2582 functions**, with identical names, source paths,
spans, hashes, compilation context, selected features, cfgs, target and compiler:
**14 admitted /2568 excluded /0 unknown /0 failed**, versus **9 /2573 /0 /0**.
There are **five new admissions and no losses**. The three declared readers move
from excluded to admitted with their original source hashes unchanged. The two
incidental gains are `util::wire::write_endianness_check_len` and
`util::wire::write_version_len`; each is a complete memory-free wrapper returning
`size_of::<u32>()`. They were identified by the complete roster delta, not added
to the declared target to improve its success rate.

The current CLI uses text rendering and exits 2 solely for the explicit zero-pair
inventory budget: its complete saved JSON report records 91 possible comparisons
and incomplete matching. No whole-library detector run is claimed. A separate
strict import of the three natural graphs and their three counterparts in each
of the std/no_std development units independently verifies documents, producer
semantics and admission. All nine graphs then complete all **36 comparisons**
with unchanged defaults; the six same-width natural/development comparisons
accept with score 1. All cross-width comparisons also accept under the existing
structural similarity policy. This is extraction/composition evidence, not
semantic equivalence, rewrite safety or a detector-quality improvement claim.

The source census retains neighboring enums, derives, macros, unsafe code,
imports and complete modules. The original crate remains internally no_std in
its default-feature library context; neither the original source nor its build
selection was rewritten to obtain these gains. Suffix/inclusive/array/mutable
views, signed conversions and partial-flow helper transport remain excluded.
The neighboring `try_read_*` family additionally requires concrete Result/tuple
payloads, error values, generic length-check helpers and `?`; supporting these
three low-level readers does not certify those callers.


## Frozen replay and incidental composition

Canonical replay **run-sd5dhs2u** reproduces all 60 frozen identities, outcomes,
context activity, record counts, denominators and source-blocker inventory from
run-dfcyj99b: **20 admitted /33 excluded /7 inactive**. There are no new or lost
admissions in that selection. All **70 serialized documents** in the captured
full units pass the canonical schema checks.

The full unchanged rosters preserve lz4_flex's **150 functions /7 admitted** and
time-core's **226 /8**. regex-syntax preserves all **956 identities** and moves
**19 →20 admitted**, without losses: the incidental gain is
`hir::LookSet::read_repr`, whose original complete body connects a four-byte
prefix, native u32 conversion and `LookSet { bits }` construction. Its exact
original-unit documents, producer semantics and strict admission independently
verify. It was found by the whole-roster comparison, not selected retrospectively
as part of the declared three-reader target.

All **14** current regex-automata admissions also independently pass document,
producer and admission verification after loading the complete original bundle,
including the two incidental scalar-size wrappers and the nine old admissions.
The total observed natural gain is five functions in regex-automata plus one in
regex-syntax, with no losses in any of the four complete compared unit rosters.
This does not change the frozen 60-function denominator or its historical count.

## Performance validation

Quiescent measurements compare static 68c300c /3.43.0 binaries with the current
implementation, using one warmup and seven alternating baseline/current runs per
workload. Every run checks unchanged source identities, complete assessment and
matching, and expected admission counts. The unchanged copied-four-byte control
is the exact existing `byte_windows::workspace` source (SHA256
`6ee50adcc4ab557360e126371535383346bb9eebe39627c1a98c5fd85f995145`), not a newly
simplified replacement. The other unchanged control is `cyclic-prefix-scans.rs`.

| Workload | Baseline median (range), seconds | Current median (range), seconds | Median peak RSS, baseline/current KiB |
| --- | --- | --- | --- |
| Existing copied byte windows, 16 functions /11 admitted /55 comparisons | 0.86 (0.83–0.90) | 0.89 (0.87–0.92) | 121952 /121916 |
| Existing cyclic scans, 6 functions /4 admitted /6 comparisons | 0.63 (0.59–0.66) | 0.63 (0.62–0.64) | 110960 /110516 |
| Newly activated prefix corpus, 11 functions | 0.34 (0.31–0.37) | 0.67 (0.65–0.72) | 118280 /120728 |

The copied control's measured median increases 0.03 seconds (3.5%); its ranges
overlap and peak memory is unchanged. The cyclic median is unchanged. Both
controls' complete matching reports are identical in every retained comparison.
This bounded check finds no material control regression but does not rule out a
smaller shift or establish whole-library performance. The activated workload
performs more work: eight admissions and 28 evaluated comparisons, versus one
admission and no pairs. Its additional runtime is recorded, not described as a
speedup. No new cache or provenance/source buffer is used to change these results.

## Validation and remaining boundary

Workspace nextest passes **852 tests**, with five existing skips. The final
prefix-reader and cross-target regression suite, workspace doctests, all-target
clippy with warnings denied, formatting and diff checks pass. Rstyle reports
**2703 functions /0 violations**, and patterns reports **0 candidates**. No
threshold, scoring, suppression or ignore-ledger change is made. The four
independent numeric/runtime oracle tests pass against exact original bodies.
Cross-target Cargo verification covers the three installed pinned targets;
native runtime execution is on ARM Linux, not native macOS/x86 execution.

This completes the declared three-reader family and its eight positive
compositions. It does not complete the whole Rust roadmap. Remaining connected
work includes the retained partial-flow caller and the richer Result/error/tuple
and `?` chain in `try_read_*`; general Option payload patterns, helper effects,
reference-bearing results, mutable/array/range views and unsafe extraction must
not be inferred from this activation. Keep further target selection based on
source and full blocker combinations before admission assessment.
