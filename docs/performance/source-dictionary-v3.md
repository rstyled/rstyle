# Function source dictionary in bundle version 3

The internal handoff now uses `rstyle.shared-unit/3`. Its `sources` array owns
original UTF-8 snapshots. Each function's `source` contains a numeric `snapshot`
handle, a half-open byte `range` within that snapshot, and the unchanged actual
function-content `digest` claim. Function navigation `fn_span` remains separate:
standalone inputs can own only function bytes while their navigation span refers
to a larger original file. Capture-owned functions normally reference the full
compiler file with their original offsets.

Source handles are local to the bundle and its namespace; no bare handle carries
cross-store identity, source verification or package authority. Export assigns
handles in function traversal order by the identity of already-owned immutable
snapshot allocations. It performs no content hashing or JSON-based identity
construction. Live borrowed owners prevent address reuse during assignment.
Independent owners with equal bytes may receive different handles; they are
never merged using a digest or spelling. Pointer values are not serialized.

Import allocates each source string once, checks handles and UTF-8 ranges, and
restores `SourceText` views before selected-unit inspection or evidence validation.
It rejects dangling handles, unused snapshots, unknown fields and invalid bounds.
The digest is preserved as an untrusted claim, never installed in the actual
identity memo. Ordinary source/graph correspondence, package and build checks,
strict/generated authority separation, completeness and evidence budgets still
apply to the reconstructed frontend. There is no new trust flag or cache policy.

Version-2 bundles are deliberately unsupported; re-capture them with the matching
driver. Frontend function references retain their existing namespace contract.
Readable reports and standalone frontend JSON still serialize source bytes and
digests as before. The report's `unit_bundle_schema` now advertises version 3.
The streaming transport seal and exact imported byte-snapshot check are unchanged.

This is the first wire-dictionary increment. Census, declaration-origin and graph
source records still carry their existing source fields; they are not silently
rewritten or granted authority through a function-source handle. Their repeated
wire bytes remain for subsequent consolidation. A whole original snapshot may
also contain non-function text, so a function-only dictionary is not guaranteed
to reduce every bundle's size.

## Measurements and validation

Baseline source `caa1cd9` uses the preceding increment's retained isolated samples;
saved tools are in `/tmp/rstyle-before-source-dictionary`. Candidate is this
commit. [312 raw samples](source-dictionary-v3-samples.json) cover six fixtures,
debug/release analyzers, trace modes and strict imports. The existing
`measure_library_resolution.py` and `measure_broad_pipeline.py` harnesses used
`after target` with `/tmp/rstyle-fast-checksum-{measurement,broad}`. Source paths,
warm caches and ordinary check-profile fixture settings match. Builds/setup,
tests and report comparisons did not overlap timings. Each setting has one
warmup, three ordinary samples and a separate diagnostic sample; strict replay
has three samples per fixture/profile. Aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30, locked dependencies. GNU time resolution is 0.01 seconds;
RSS is a whole-command memory proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after | Function source strings → snapshots |
| --- | --- | --- | --- |
| minimal | 0.24 → 0.23 | 0.21 → 0.21 | 2 → 1 |
| read | 1.14 → 1.11 | 0.29 → 0.30 | 20 → 1 |
| inherent | 1.65 → 1.67 | 0.38 → 0.39 | 21 → 1 |
| shared-fanout | 0.89 → 0.90 | 0.27 → 0.29 | 8 → 1 |
| many-pairs | 1.66 → 1.66 | 0.36 → 0.33 | 49 → 1 |
| larger-graphs | 3.67 → 3.64 | 0.71 → 0.69 | 8 → 1 |

Timing changes are mixed; no uniform speedup is established. Snapshot counts
cover frontend function-source storage only, not census/origin bytes or all
allocator activity. Representative debug bundle sizes increase by 82, 960,
1,052, 297, 1,865 and 306 bytes respectively. The complete original snapshots
contain slightly more bytes than all function substrings, and references add
metadata. This step establishes shared ownership across transport; it does not
claim compression. Exact representative sizes and source-byte counts are in the
raw artifact. Remaining repeated census/origin records still dominate those
bundles and remain required follow-up work.

All 120 complete matching reports compare exactly. All 2,160 frontend records
preserve exact source bytes/digests/spans, names, signatures, bindings, bodies,
source diagnostics and scan-completeness. Complete frontend options also match
after normalizing per-run output-directory paths in compiler arguments. Harness
exact-score, admission, decision, completeness and strict-replay assertions pass.
All 687 nextest tests passed, zero skipped; workspace doctests passed. Formatting,
warning-free workspace/all-target Clippy, 24 study tests and two harness tests
passed. Both required gates show zero style violations and zero actionable
candidates; the same 15 reviewed ignores remain. Logs:
`/tmp/rstyle-source-dictionary-*`.
