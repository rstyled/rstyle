# Immutable selected-unit context in CLI import

The CLI now constructs one private `SelectedContext` from the frozen selected
Cargo unit before importing its frontend functions. Construction validates the
manifest identity, navigation and transport budget. No mutable access is exposed.
Every function's supplied context is still decoded with the closed wire schema
and compared by complete structural equality with that checked owner. An equal
record necessarily has the same validated manifest bytes and digest; importing
another function no longer rehashes the manifest, reconstructs the planned
context, or serializes the context merely to repeat its budget check.

This is owner-local reuse, with no filesystem metadata cache, cross-run cache or
weaker association key. Missing, unexpected and malformed contexts still fail.
Changed manifest bytes, even with a newly consistent digest, cannot match the
original selected owner. Non-data units still reject any selected data context.
There is no format or provenance guarantee change.

Public package/context validation, standalone census checks and nominal binding
still validate their own independent boundaries. This increment does not claim
to consolidate those separate owners. Regression tests exercise repeated matches,
mutation after checked-owner creation, invalid initial manifests, changed selected
fields and unknown fields. Existing source-edit and handoff tests remain enabled.

## Measurements and validation

Baseline is source `9a9464c`, reusing the previous increment's retained isolated
samples as the matched before run. Candidate is this commit. The same six
fixtures, source paths, warm caches, ordinary check profile and debug/release
analyzers are used. [312 raw samples](selected-context-samples.json) include
trace off/on and strict replay. One warmup, three ordinary samples and a separate
diagnostic sample per setting; three strict replays per fixture/profile. Builds,
tests and report comparisons did not overlap timing. Aarch64, eight-CPU quota,
16 GiB, nightly-2026-09-30, locked dependencies. GNU time resolution is 0.01s;
RSS describes the whole command, not this owner alone.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.24 | 0.21 → 0.21 |
| read | 1.16 → 1.15 | 0.33 → 0.32 |
| inherent | 1.67 → 1.66 | 0.38 → 0.37 |
| shared-fanout | 0.88 → 0.88 | 0.31 → 0.31 |
| many-pairs | 1.67 → 1.69 | 0.33 → 0.37 |
| larger-graphs | 3.66 → 3.66 | 0.71 → 0.70 |

These mixed samples do not establish a command speedup. The structural change
reduces selected-context manifest hashing, planned-context cloning and budget
serialization from function count to unit count: 20 → 1 for read, 21 → 1 for
inherent and 49 → 1 for many-pairs. These are call-site counts, not allocator
events, and exclude other independent boundary validation. Complete context
decoding and comparison still occur for every function.

All 120 full matching reports compare exactly; harness admission, exact-score,
decision, completeness and strict-import assertions pass. All 680 nextest tests
passed, zero skipped, and workspace doctests passed. Formatting, warning-free
workspace/all-target Clippy, 24 study tests and two harness tests passed. Required
gates show zero style violations, zero actionable candidates and the same 15
reviewed ignores. Logs: `/tmp/rstyle-selected-context-*`. Measurements used the
existing `measure_library_resolution.py` and `measure_broad_pipeline.py` harnesses
with `after target`, using `/tmp/rstyle-fast-checksum-measurement` and
`/tmp/rstyle-fast-checksum-broad`.
