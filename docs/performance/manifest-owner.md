# Immutable manifest content identity

`PackageContext::manifest_bytes` now uses `SourceSnapshot`. Repeated validation
of an immutable context or its clones compares the stored manifest digest claim
with the actual identity memo on that owner. It no longer hashes manifest bytes
on each call. The selected context still checks package identity, path, size and
all existing association constraints. Equal content alone does not authorize
cache reuse or merge package namespaces.

The readable and bundle encodings remain strings. Imports create fresh owners;
independently decoded contexts may still have independent memos. This is not a
process-global cache or a claim that every equivalent serialized context is
interned. Caller-supplied claims never initialize the actual digest. Existing
manifest edit, incorrect-digest, over-budget and selected-unit association tests
retain their assertions; edits explicitly construct a new immutable owner.

The Rust API field changes from `String` to `SourceSnapshot`, with `.into()`
construction from strings. No checksum algorithm, digest field, schema, semantic
budget, graph admission or strict/generated policy changes. The source-snapshot
regressions establish fresh-import/edit and shared-clone identity behavior.

## Measurements and validation

Production baseline is `330b7d2`; intervening `3444d4d` only adds the diameter
benchmark and documentation. Retained baseline samples and tools in
`/tmp/rstyle-before-manifest-owner` precede this change. Candidate is this
increment. Existing pipeline harnesses ran `after target` with
`/tmp/rstyle-fast-checksum-{measurement,broad}` after builds and gates finished.
[312 raw samples](manifest-owner-samples.json) cover six fixtures, debug/release,
trace on/off and strict replay. One warmup and three ordinary samples per
setting are separate from diagnostic runs; strict replay has three samples.
Fixture paths, warm caches and analyzed check-profile settings match. No builds,
tests or comparisons overlap timing. Environment: aarch64, eight-CPU quota,
16 GiB, nightly-2026-09-30, locked dependencies; GNU time resolution 0.01 seconds.
RSS remains a whole-command proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.24 → 0.23 | 0.20 → 0.21 |
| read | 1.13 → 1.14 | 0.33 → 0.32 |
| inherent | 1.64 → 1.66 | 0.36 → 0.35 |
| shared-fanout | 0.90 → 0.86 | 0.26 → 0.29 |
| many-pairs | 1.67 → 1.66 | 0.37 → 0.33 |
| larger-graphs | 3.67 → 3.64 | 0.70 → 0.72 |

Results are mixed; no uniform runtime improvement is established. Representative
bundle byte counts remain unchanged. This change removes repeated hashing within
an owner, not all decoding or independent-owner work.

All 120 matching reports and 2,160 frontend records/options compare exactly after
normalizing run-directory paths in compiler arguments. Harness admission, exact
score, completeness and strict replay assertions pass. All 692 nextest tests
passed with zero skipped; workspace doctests passed. Formatting, warning-free
workspace/all-target Clippy, 24 study tests and two harness tests passed. Both
quality gates are clean with zero style violations/actionable pattern candidates
and the same 15 reviewed ignores. Logs: `/tmp/rstyle-manifest-owner-*`.
The 34.303-second suite duration is observational, not a controlled suite claim.
