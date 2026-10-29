# Shared source options in bundle version 4

The later [matching-build metadata change](matching-build-metadata.md) retires
the graph captured-build copies mentioned below. These historical measurements
remain associated with their stated baseline and implementation.

The version-4 internal handoff introduced the source sharing retained by
[`rstyle.shared-unit/7`](shared-unit-bundle.md). Its source table
serves function ranges, nominal census files and declaration-origin files.
Publication registers immutable shared file allocations once and writes their
bytes once. Import resolves all references through one checked table and shares
its `Arc<str>` buffers. Dense IDs describe this bundle only; they establish no
content identity or verification authority.

Census and origin frontend options use tagged wire records with `kind` and
`value`. Census files carry path, snapshot ID and the original digest claim;
origin declarations carry nominal evidence, source file, snapshot ID, original
file digest and declaration span. Explicit optional null presence is preserved
and checked for valid canonical paths, bounds and duplicates before rendering.
Malformed original options use the `inline` kind and retain their original JSON
and decoding error. Other options retain their existing grammar. Unknown fields,
wrong option kinds, dangling references and unused snapshots are rejected.

The adapter borrows build metadata for publication and moves it on import.
`BuildFacts` has a defaulted options type parameter for the wire adapter; ordinary
callers continue to use `BuildFacts` and `FrontendOptions`. Export does not render
whole census/origin JSON trees or populate their readable JSON memo. Readable
frontend JSON retains the previous values, including untrusted digest claims and
explicit nulls. No version 3 runtime compatibility adapter is retained.

This changes representation, not acceptance. Source digests, census context,
nominal association, range correspondence, evidence budgets and strict/generated
verification separation keep their existing meaning. The importer reconstructs
ordinary unvalidated evidence before the existing semantic validators run. A
source ID or matching stored digest does not certify that evidence.

Focused regressions cover readable byte parity, shared imported allocations,
malformed/tag-like inline records, incorrect digest claims and invalid source
references/null-presence records. CLI corruption fixtures retain independently
corrupt frontend evidence against the original graph tables; their wire adapter
remaps only actual references and removes fixture-created orphan snapshots.

## Measurements and validation

Baseline source is `a4666ab`; its previously isolated samples were retained as
`before` without rerunning them. Saved binaries are in
`/tmp/rstyle-before-source-options`. Candidate is this increment. The existing
`measure_library_resolution.py` and `measure_broad_pipeline.py` harnesses ran
`after target` against `/tmp/rstyle-fast-checksum-{measurement,broad}` after
builds and gates finished. Fixture paths, warm caches and analyzed check-profile
settings match. No builds, tests or report comparisons overlapped measurements.
The [312 raw samples](source-options-v4-samples.json) cover six fixtures,
debug/release analyzers, trace on/off and strict imported replay. Each command
setting has one warmup, three ordinary samples and a separate diagnostic run;
strict replay has three samples. Environment: aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30, locked dependencies. GNU time resolves 0.01 seconds; RSS is
a whole-command memory proxy, not per-allocation attribution.

Median ordinary no-trace seconds and representative debug bundle bytes:

| Workload | Debug before → after | Release before → after | Bundle bytes before → after |
| --- | --- | --- | --- |
| minimal | 0.23 → 0.22 | 0.21 → 0.21 | 53,002 → 53,002 |
| read | 1.11 → 1.14 | 0.30 → 0.32 | 2,023,176 → 1,939,416 |
| inherent | 1.67 → 1.65 | 0.39 → 0.34 | 3,037,160 → 2,937,494 |
| shared-fanout | 0.90 → 0.87 | 0.29 → 0.27 | 879,628 → 879,628 |
| many-pairs | 1.66 → 1.66 | 0.33 → 0.36 | 3,585,472 → 3,365,707 |
| larger-graphs | 3.64 → 3.66 | 0.69 → 0.69 | 2,140,798 → 2,140,798 |

Timings are mixed; this does not establish a uniform command speedup. In the
read, inherent and many-pair bundles, respectively, inline census/origin source
bytes fall from 250,560 to 167,040; 297,108 to 198,072; and 654,150 to 436,100.
The source table still contains one original snapshot per fixture (1,392, 1,572
and 2,225 bytes). The remaining inline copies are in PDG/CFG captured-build
metadata, outside this frontend adapter. This deliberately bounded increment
removes one third of those repeated bytes; it does not claim all graph source
records have been consolidated. Scalar fixtures do not carry these options.

All 120 complete matching reports compare exactly. All 2,160 frontend records
and complete frontend options match after normalizing run-directory paths in
compiler arguments. Source bytes/digests/spans, names, signatures, bindings,
bodies, diagnostics and completeness are unchanged. Harness admission, exact
score, decision and strict replay assertions pass. All 690 nextest tests passed,
zero skipped; workspace doctests, formatting and warning-free workspace/all-target
Clippy passed. Both quality gates are clean: zero style violations and zero
actionable candidates; 15 existing reviewed ignores remain unchanged. All 24
study tests and two measurement-harness tests pass. Logs are
`/tmp/rstyle-source-options-*`. The 33.924-second suite duration is observational,
not a controlled whole-suite performance claim.
