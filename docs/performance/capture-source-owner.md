# Capture-owned original compiler sources

The driver now creates one `Sources` owner per `after_analysis` callback and
shares it across function extraction, declaration origins, the nominal census
and conditional source inventory. A private `CompilerSource` handle uses rustc's
file start position within that one callback's source map. It is never serialized,
used as a persistent identity or shared between compiler invocations.

On first access, the owner recovers original UTF-8 bytes from rustc's immutable
normalized buffer and normalization positions. It restores the leading BOM and
CR in CRLF without reopening the file. Later accesses borrow the same snapshot.
Function `SourceText` records are checked ranges into that snapshot from capture
onward; the HIR extraction context shares the same immutable function range.
Declaration origins and the census share the original file's `Arc<str>` and one
memoized actual file SHA-256. Function-range identities retain their separate
meaning and memoized actual hashes. No supplied digest claim fills an identity
cache.

Bounds, UTF-8 boundaries, original offsets, selected package namespaces and
unexpected original-prefix rejection remain. Whole-file snapshots include a BOM;
compiler spans beginning after that BOM still produce the same exact ranges as
before. Public/import boundary verification remains independent. Source wire
formats, contract identities, policies, matching and completeness are unchanged.
This does not yet deduplicate repeated file/range bytes in bundle JSON.

Regressions use rustc's actual normalization machinery with BOM, CRLF and Unicode,
assert pointer and identity-result reuse within an owner, and require independent
owners and edited source records to remain separate. Checked range construction
covers invalid bounds, reversed spans and split UTF-8. Existing compiler-backed
source-offset, source-edit, nominal-binding and conditional inventory tests remain
enabled. Diagnostic stages `capture.source_restore` and `capture.source_hash`
count actual cache misses and bytes without adding content fingerprints for
instrumentation.

## Measurements and validation

Baseline source `6762c93` uses the previous increment's retained isolated samples;
pre-change binaries are saved in `/tmp/rstyle-before-capture-sources`. Candidate
is this commit. [312 raw samples](capture-source-samples.json) cover six fixtures,
debug/release analyzers, trace off/on and strict replay. The existing
`measure_library_resolution.py` and `measure_broad_pipeline.py` harnesses used
`after target` with `/tmp/rstyle-fast-checksum-{measurement,broad}`. Source paths,
warm cache conditions and ordinary check-profile fixture settings match. Each
setting has one warmup, three ordinary samples and one separate diagnostic sample,
plus three strict replays per case/profile. Builds/setup, tests and report
comparisons did not overlap timing. Aarch64, eight-CPU quota, 16 GiB, pinned
nightly-2026-09-30, locked dependencies. GNU time resolution is 0.01 seconds;
peak RSS remains a whole-command memory proxy.

Median ordinary no-trace wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.22 → 0.24 | 0.20 → 0.21 |
| read | 1.16 → 1.14 | 0.32 → 0.29 |
| inherent | 1.68 → 1.65 | 0.35 → 0.38 |
| shared-fanout | 0.89 → 0.89 | 0.31 → 0.27 |
| many-pairs | 1.64 → 1.66 | 0.33 → 0.36 |
| larger-graphs | 3.65 → 3.67 | 0.71 → 0.71 |

Samples are mixed; no general command or suite speedup is established. Diagnostic
runs count one successful original-file reconstruction per workload: normalized
bytes 59, 1,392, 1,572, 802, 2,225 and 1,719 respectively. Full-file identity hashing
occurs once for each data workload (read/inherent/many-pairs), with the same byte
counts, and zero times for the other three. Those counts concern this capture
owner only; independent strict-import validation still hashes its own input.
Before this change each function alone reconstructed its entire compiler file,
so those calls fall at least from 2/20/21/8/49/8 to one, before counting old
additional declaration, census, implementation-span and inventory requests.
No exact total old reconstruction/allocation count is claimed.

All 120 complete matching reports compare exactly. All 2,160 frontend source
records preserve exact source bytes/digests/spans, names, signatures, bindings,
bodies and source findings. Complete frontend options, including census and
origin evidence, also compare exactly after normalizing per-run output-directory
paths in compiler invocation arguments. An initial raw options comparison found
that expected run-path difference; no semantic field was normalized away.
Harness exact-score, admission, decision, completeness and strict-replay checks
pass. All 684 nextest tests passed, zero skipped, plus workspace doctests.
Formatting, warning-free workspace/all-target Clippy, 24 study tests and two
measurement-harness tests passed. Both gates show zero violations and zero
actionable candidates, with the same 15 reviewed ignores. Logs:
`/tmp/rstyle-capture-sources-*`.
