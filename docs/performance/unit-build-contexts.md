# Unit-owned build contexts

Base: `5bf2e4e`. The introducing commit replaces
`nominal_binding::build_digest`, which serialized and SHA-256 hashed the entire
`BuildFacts` of every concrete interface solely to label its report binding.
No graph-admission, source-correspondence or cache authority consumed that digest.

Each `UnitBindings` now owns a `BuildContexts` table. Before allocating a record,
interning compares exact producer revision, compiler version/commit, edition,
target, pointer width, layout, ordered cfg/features, overflow/debug settings,
panic strategy and optimization level. It also compares these frontend options:
`edition`, `target`, `invocation`, the explicit extraction selection, and the
selected nominal-unit context (including package and target association).
Missing and explicit null remain different. Equality borrows fields without
serializing them; a new context copies only these settings once. Full raw
frontend evidence remains attached to each function and independently checked.

Source identifiers/files, definition identities, concrete/declaration/field
observations, helper evidence, receiver records, library-resolution inventories
and source correspondence are function/unit observations, not build settings.
They are deliberately excluded from this build table. Unknown future frontend
options remain raw evidence and do not implicitly become v1 build-setting keys;
a future option affecting build equivalence must explicitly extend/version this
contract. This table is report bookkeeping, never a verification cache key or an
authority to accept those observations.

## Report/API changes

The nominal binding discriminator advances to
`rstyle.rust.nominal-unit-binding/2`; `build_digest` is removed and `build_id` is
a dense unit-local integer. The report's `build_contexts` array contains one
`unit_bundle` namespace and `builds` table per imported unit. Each table has
identity `rstyle.rust.unit-build-contexts/1` and ordered `contexts` records using
the existing build-facts setting representation. Resolve a binding's integer
only in the entry whose `unit_bundle` equals its function's
`unit_evidence.bundle`. Target/package names alone are not that namespace.
The public `bind_unit` returns `UnitBindings` (bindings plus owned table).

IDs are not persistent fingerprints; the same integer in independent units or
runs says nothing about build equality. Insertions/order/edits may change them.
There is no public bare-ID resolver or serialized proof capability. No value is
placed in an obsolete digest field. Manifest/source hashes and source-edit
checks are unchanged in this increment. Package checks still validate the
independently selected context before any bindings are created.

Retired guarantee: the report no longer offers a cryptographic digest of an
entire function's build-and-observation object. Consumers can inspect exact
settings and raw observations directly. This is separate from bundle transport
consistency and strict graph reconstruction, which are unchanged.

## Validation and measurement

Focused tests check sharing despite different function observations, retention
of the selected invocation, every top-level build field, every relevant frontend
option, and missing/null distinctions. CLI coverage resolves IDs through the
unit's exported table across four selected contexts and verifies changed
manifest/source/build observations after recapture. Existing corruption and
source-range tests remain enabled. Formatting, warning-free workspace/all-target Clippy, the full nextest/doctest
wrapper (649 tests passed, zero skipped), and 24 Python study tests passed.
Both style gates passed: zero violations, zero actionable candidates, and the
same 15 reviewed ignores. No suppressions or thresholds changed.

The prebuilt base and candidate ran sequentially using
`scripts/measure_library_resolution.py` (same method and machine as
[location interning](local-location-handles.md)): aarch64, eight CPU quota,
16 GiB limit, pinned nightly, locked dependencies, no concurrent builds/tests;
ordinary fixture Cargo check settings with both debug and release analyzers.
One warmup, three ordinary plain/trace samples, one separate diagnostic run per
mode, three strict imports per workload/profile; 156 total commands.
[Raw samples](unit-build-context-samples.json) preserve times, report sizes and
RSS. Before/after phases were not interleaved; machine drift and 0.01 s timer
resolution limit conclusions.

Ordinary wall medians, seconds before → after:

| Analyzer | Workload | Plain | Trace |
| --- | --- | --- | --- |
| Debug | Minimal (2 functions/1 pair) | 0.23 → 0.24 | 0.25 → 0.23 |
| Debug | Read (20/190) | 1.56 → 1.57 | 1.58 → 1.59 |
| Debug | Inherent (21/210) | 2.19 → 2.18 | 2.19 → 2.18 |
| Release | Minimal | 0.21 → 0.21 | 0.21 → 0.21 |
| Release | Read | 0.35 → 0.37 | 0.34 → 0.35 |
| Release | Inherent | 0.41 → 0.41 | 0.40 → 0.41 |

Strict import medians for minimal/read/inherent: debug 0.01/0.64/0.96 →
0.01/0.63/0.95 s; release 0.00/0.06/0.09 → 0.00/0.08/0.11 s. Zero is below
timer resolution. Strict imports do not execute nominal report binding; these
samples help expose ordinary run-to-run noise rather than a causal improvement.
Maximum RSS (all modes/iterations) for debug read/inherent: 114052/121192 →
114008/121328 KiB; release: 109464/116660 → 109412/116628 KiB. These samples
establish no consistent speed or memory gain. Every sampled admission, score,
stage outcome, pair decision and completeness flag matched across revisions,
trace settings and analyzer profiles; every strict replay completed.

Structural result: the former one whole-BuildFacts JSON byte-vector and SHA
operation per emitted nominal binding becomes zero. A unit with identical
settings owns one context, as checked by the CLI family. Counts are per-operation
code facts, not measured global allocation totals; harness fingerprint counters
refer to the already-retired library artifact work and are zero in both versions.
The full optimization goal remains active.


## Routine source consistency (subsequent Data3.92 increment)

New Data-v3 function-source claims and fresh manifest/file/declaration claims use
existing `xxh3-64` checksums. Each immutable source owner memoizes actual bytes,
with lazy legacy SHA computation only when a historical claim requests it.
Strict verification checks both supported schemes, malformed/unknown schemes,
missing bytes, exact original ranges and literal cross-record consistency.
Module expansion proofs retain their already verified census claim rather than
recomputing another identity. These checksums grant no source or call authority.
Historical function-source profiles, including r0, retain SHA conformance;
reviewed standard-library SHA pins and frozen selection identities stay unchanged.
No semantic capability, admission threshold or matching score changes.
