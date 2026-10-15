# Checked declaration-file inventories

Baseline source: `b0d239cbb5d8dc0a73578891d1941329e0566076`. Declaration origins
previously rehashed and parsed the whole original file for every declaration,
including repeated declarations across functions in a single nominal binding.

Each standalone origin verification now owns one checked file inventory. A
selected-unit nominal binding owns that inventory across its functions, together
with the selected package, exact build-context table and source association map.
The first successful origin checks the complete source digest and parses the
original file, retaining immutable bytes and exact struct item spans. Subsequent
origins for that same path must match complete bytes and the checked digest
claim before reusing those spans. Each declaration still checks its exact byte
range, unique parsed struct span, nominal identity and complete interface
inventory. Package/root, build, function-range and digest checks remain in place.

No caller-provided flag, hash-only lookup or serialized proof constructs a checked
file. New standalone calls get a fresh inventory. Edited snapshots are rejected
within the old inventory even when a caller refreshes their digest claims; a
new independent call validates the new consistent snapshot. A failed first
origin publishes no file entry. Binding failures discard the entire binding
context; no semantic proof or policy cache escapes this operation.

One intentional strengthening is explicit: standalone origin inventories now
reject contradictory full-file snapshots under the same path, even if each
individual snapshot has a correct digest. Selected-unit binding already rejected
that contradiction through its source association map. This implements the
required one-path/one-snapshot relationship, rather than treating independent
correct checksums as sufficient association.

Origin wire size budgeting counts the same exact Serde JSON encoding through a
counting writer rather than allocating a temporary byte vector. The byte limit,
escaping and UTF-8 accounting are unchanged. This still visits the wire encoding
for the budget; it does not claim that all budget serialization has disappeared.
The origin identity/schema, SHA-256 meanings and serialized output are unchanged.

## Coverage and limits

New regressions cover repeated use of one parsed span inventory and one original
buffer, consistent edits in fresh owners, edited bytes with refreshed digests,
contradictory snapshots in one inventory, corrupt digest claims, UTF-8/range
failures, and struct-shaped text in comments. Existing interface/nominal, source
correspondence, source-edit, package, strict-import and policy tests stay enabled.
An exact encoding-size regression covers BOM, Unicode and JSON escapes.

This inventory is scoped to declaration verification or nominal binding. It does
not yet unify independent compiler census, capture and strict-import owners.
Capture-time original-file reconstruction, declaration/manifest identity work,
source wire dictionaries and the remaining internal unit/function identities
remain part of the full task. No global content cache or weaker cache authority
is introduced.

## Matched measurements

Linux aarch64, eight CPU quota, 16 GiB, pinned nightly-2026-09-30 and locked
dependencies. Baseline source was `b0d239c`; candidate is the implementation in
this commit. Baseline captured producer metadata was `53f71233251ec7a6581ce093002629dd96bd5cb9+dirty`;
candidate metadata was `b0d239cbb5d8dc0a73578891d1941329e0566076+dirty`.
Those build labels are distinguished from the exact recorded source revision.
The preserved untracked user file remained present.

Commands: `python3 scripts/measure_library_resolution.py before|after BIN_ROOT OUTPUT`
and `python3 scripts/measure_broad_pipeline.py before|after BIN_ROOT OUTPUT`.
Baseline binaries: `/tmp/rstyle-before-origin-inventory/{debug,release}`.
Candidate binaries: `target/{debug,release}`. Outputs:
`/tmp/rstyle-origin-inventory-measurement` and `/tmp/rstyle-origin-inventory-broad`.
Build/setup time is excluded; no builds or tests ran during measurements.
Each profile/workload/trace setting has one warmup, three ordinary samples and
one separate diagnostic run; each case also has three strict imports. Both
suites and revisions total 312 commands. Phases were sequential, not interleaved.
The wall clock has 0.01-second resolution; this is a small sample, without a
statistical-significance claim. Fixture compilation settings did not change.

All before/after checks passed for complete capture/assessment, admitted functions,
exhaustive pairs, named decisions, displayed scores, exact score inputs and stage
outcomes, with trace both on and off. Every strict replay completed. Exact
preliminary scores are compared within matched profile/workload roots; nominal
type strings differ across debug/release fixture paths. Cross-profile decisions,
displayed scores and stage outcomes still match.

Median seconds before → after:

| Workload / analyzer | Ordinary | Trace | Strict import |
| --- | --- | --- | --- |
| minimal / debug | 0.23 → 0.23 | 0.23 → 0.23 | 0.00 → 0.01 |
| minimal / release | 0.21 → 0.20 | 0.20 → 0.21 | 0.00 → 0.00 |
| read / debug | 1.31 → 1.24 | 1.35 → 1.28 | 0.57 → 0.56 |
| read / release | 0.32 → 0.32 | 0.30 → 0.33 | 0.06 → 0.05 |
| inherent / debug | 1.89 → 1.79 | 1.92 → 1.82 | 0.87 → 0.88 |
| inherent / release | 0.36 → 0.35 | 0.36 → 0.36 | 0.08 → 0.09 |
| shared-fanout / debug | 0.88 → 0.91 | 0.93 → 0.95 | 0.70 → 0.70 |
| shared-fanout / release | 0.27 → 0.29 | 0.29 → 0.32 | 0.06 → 0.06 |
| many-pairs / debug | 1.95 → 1.83 | 2.13 → 2.02 | 0.63 → 0.64 |
| many-pairs / release | 0.39 → 0.39 | 0.41 → 0.40 | 0.07 → 0.07 |
| larger-graphs / debug | 3.80 → 3.79 | 4.58 → 4.64 | 4.18 → 4.15 |
| larger-graphs / release | 0.73 → 0.71 | 0.78 → 0.78 | 0.40 → 0.42 |

The observed debug read/inherent improvements apply to these fixtures only.
Release read was unchanged and release inherent differed by 0.01 seconds.
Scalar controls and larger graphs do not demonstrate an improvement from this
source-specific change. No suite-wide speedup is inferred.

Separate diagnostic `unit.inspect` milliseconds (one plain sample each):

| Analyzer / family | Before | After |
| --- | --- | --- |
| debug / read | 65.150 | 10.469 |
| debug / inherent | 87.161 | 32.358 |
| release / read | 7.448 | 1.296 |
| release / inherent | 10.511 | 7.758 |

These samples attribute the nominal-binding check; they are not ordinary
medians and are not summed with nested stages. Peak RSS, report sizes and all
ordinary samples are retained in `origin-file-inventory-samples.json`.

Structural counts for the selected-unit nominal-binding call: read has 40
declaration occurrences in one 1,392-byte file, inherent has 42 in one 1,572-byte
file, and many-pairs has 49 in one 2,225-byte file. Full-file hashes and parses
therefore fall from 40/42/49 to one per binding call, reducing whole-file hash/
parse byte visits from 55,680/66,024/109,025 to 1,392/1,572/2,225 respectively.
These are scoped counts from the concrete evidence inventories and code path,
not counts for the entire pipeline. Each independent driver/public verification
still has its own boundary inventory. Repeated complete-byte equality remains
necessary when separately supplied records have different allocations.

Origin budget checks no longer retain a serialized vector of up to 8 MiB.
The checked inventory retains original bytes by `Arc` and one parsed span list
per file. These are allocation proxies; no total allocator-byte saving is
claimed. Other source/census/graph/bundle checks remain outside these counts.

## Validation

The final pinned wrapper passed 662 tests, zero skipped, and workspace doctests.
Warning-free workspace/all-target Clippy, formatting, diff checks, 24 Python
study tests and two harness tests passed. Both quality gates passed with zero
style violations (2,025 functions) and zero actionable pattern candidates. The
15 existing reviewed ignores are unchanged. An initial spacing violation was
fixed; logs are retained in `/tmp/rstyle-origin-inventory-*`. No assertions,
thresholds, suppressions or ignores were weakened.
