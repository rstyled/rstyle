# Native prefix readers: source-selected target

This declaration precedes admission assessment. Current Data-v3 is 3.43.0;
there is no new activation or admission claim. Keep the frozen 60-function
inventory, identities, denominators and historical outcomes unchanged.

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

`eval/coverage-development/native-prefix-readers.rs` fixes 11 function identities
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
python3 eval/coverage-development/native-prefix-readers-oracle.py \
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
