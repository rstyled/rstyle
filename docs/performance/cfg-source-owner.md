# CFG content borrows the function source owner

The later [matching-build metadata change](matching-build-metadata.md) retires
the graph captured-build copies mentioned below. These historical measurements
remain associated with their stated baseline and implementation.

`CfgSource::content` now holds `Option<SourceText>` instead of `Option<String>`.
Emission clones the checked function source range and its actual identity memo,
not the text. Bundle import first checks root/certificate/depth structure, then
replaces an equal CFG text range with the owning function's range. Exact byte
equality is required. Differing or missing content stays intact for the existing
semantic validators to assess. No path, ID, digest claim or namespace changes.

This restores shared immutable ownership after ordinary JSON decoding. It is not
zero-copy decoding and does not change bundle version 4 or standalone schemas:
the wire value remains the same string. Source correspondence, edited-source
invalidation and strict/generated verification remain independent. Raw graph
captured-build metadata still contains readable JSON copies of census/origin
records; this increment does not claim to remove those transport representations.

The Rust API field changes to the immutable range type; string construction uses
`.into()`. The focused regression establishes owner/memo sharing for equal text,
byte-for-byte JSON preservation with an incorrect digest claim and another path,
and no replacement of edited or missing content. Existing semantic corruption
and strict-import tests remain required.

## Measurements and validation

Baseline source `3ed20c5` uses retained isolated samples and saved tools in
`/tmp/rstyle-before-cfg-source-owner`. Candidate is this increment. Existing
pipeline harnesses ran `after target` against
`/tmp/rstyle-fast-checksum-{measurement,broad}` after builds and gates.
[312 raw samples](cfg-source-owner-samples.json) cover six fixtures, debug/release,
trace modes and strict replay. One warmup, three ordinary samples and separate
diagnostics preserve matched fixture paths, warm caches and analyzed check-profile
settings. Replay has three samples. No builds, tests or comparisons overlap
timing. Environment: aarch64, eight-CPU quota, 16 GiB, nightly-2026-09-30,
locked dependencies; GNU time resolution 0.01 seconds, whole-command RSS proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.24 | 0.21 → 0.20 |
| read | 1.14 → 1.13 | 0.32 → 0.31 |
| inherent | 1.66 → 1.68 | 0.35 → 0.37 |
| shared-fanout | 0.86 → 0.89 | 0.29 → 0.27 |
| many-pairs | 1.66 → 1.61 | 0.33 → 0.38 |
| larger-graphs | 3.64 → 3.64 | 0.72 → 0.70 |

Timings are mixed; no uniform speedup is established. Bundle byte counts remain
unchanged. Structural improvement is removal of CFG text copies during emission
and retained source copies after import, not zero-copy transport or an allocation
counter claim.

All 120 matching reports and 2,160 frontend records/options compare exactly after
normalizing run-directory paths in compiler arguments. Harness exact scores,
admission, completeness and strict replay assertions pass. All 693 nextest tests
passed, zero skipped, followed by workspace doctests. Formatting, warning-free
workspace/all-target Clippy, 24 study tests and two harness tests passed. Both
quality gates are clean: zero style violations, zero actionable candidates and
the same 15 reviewed ignores. Logs: `/tmp/rstyle-cfg-source-owner-*`. Suite
duration 33.891 seconds is observational, not a whole-suite speedup claim.
