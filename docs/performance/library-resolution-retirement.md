# Library artifact fingerprint retirement

First increment of the [hash/matching implementation](hash-and-matching-plan.md).
Base: `9cb2b32d50f30eed075e9903139058937344c20d`; candidate code is the commit
introducing this document. The full task remains active, including typed internal
identities, source ownership, transport checksums and matching optimization.

`library_resolution::capture` records compiler-resolved core crate identity;
it no longer asks for used-crate-source artifact paths, opens files, counts
artifact bytes or computes their SHA-256. It performs zero explicit library
fingerprint reads. The compiler's own metadata I/O is separate and unchanged.
There is no replacement file checksum, metadata cache or implicit library scan.

The [version 2 contract](../extraction-profiles/read-only-library-resolution-v2.md)
documents exact format/API changes and retired guarantees. Normal operation
trusts the supported local compiler/sysroot. Definition/crate identity, compiler,
target, width, resolved dispatch, concrete receiver/signature, source invocation,
argument/effect, paired document and strict-import checks remain. Old artifact
records are not a supported runtime adapter. Historical v1 contracts and Python
experiment snapshots are unchanged.

## Method

Linux aarch64, eight-CPU cgroup quota, 16 GiB limit, pinned nightly-2026-09-30
(rustc `5c543b0b8c73c7b72bc8284ced4fb22ead15734d`), locked dependencies. Build
profiles unchanged; debug/release refer to analyzer and driver binaries. All
fixtures use the ordinary analyzed Cargo check settings. Builds, tests and gates
finished before each measurement phase; none ran concurrently with measurements.

Baseline binaries were built at the exact base and copied into
`/tmp/rstyle-hash-matching-before/{debug,release}` before edits. Candidate tools
were built after the change. Commands use explicit absolute CLI and driver paths.
Each profile/workload reuses the same fixture directory across phases, with warm
filesystem/build state, one warmup per trace setting, three ordinary samples
alternating trace off/on, then one separate diagnostic sample per trace setting.
Before and after are separate phases, not interleaved revisions; environmental
drift can affect small differences. `/usr/bin/time` measures wall and peak RSS;
its 0.01-second resolution does not establish sub-10 ms changes. Commands have a
120-second timeout. Compilation of the tools is excluded.

Reproducer: `scripts/measure_library_resolution.py before|after BIN_ROOT OUTPUT`.
The roots contain prebuilt debug/release tools and the strict replay example.
Use the same OUTPUT for parity assertions. All reports, stderr and time records
are retained in `/tmp/rstyle-library-retirement`; committed
[literal samples](library-resolution-samples.json) distinguish diagnostic,
warmup, ordinary, trace and strict-import runs. Measurements total 120 CLI
commands and 36 strict imports, 78 commands per revision. Minimal has two
functions/one pair; read/inherent use the exact test families with 20/21 admitted
functions and 190/210 pairs. The script checks all four completeness flags,
exhaustive counts, every admission and every pair's decision/displayed score and
stage outcomes. Canonical pairs match by function names to account for deliberate
provenance differences. Debug and release canonical signatures also agree.

## Ordinary timing

Medians of three samples, seconds. No diagnostic samples are included.

| Workload | Debug plain before → after | Debug trace before → after | Release plain before → after | Release trace before → after |
| --- | --- | --- | --- | --- |
| Minimal | 0.22 → 0.23 | 0.24 → 0.22 | 0.21 → 0.22 | 0.21 → 0.22 |
| Read helpers | 1.93 → 1.57 | 1.95 → 1.60 | 0.39 → 0.34 | 0.39 → 0.38 |
| Inherent helpers | 2.56 → 2.21 | 2.60 → 2.24 | 0.46 → 0.39 | 0.47 → 0.44 |

Each read/inherent capture previously fingerprinted one 67,886,530-byte rmeta
artifact; the candidate fingerprints zero library files/bytes. This structural
count comes from the captured per-unit artifact inventory and the removed
capture implementation, not from claiming that rustc no longer reads metadata.
Minimal already had zero such reads. No allocation reduction is claimed; no
allocator profiler was used. Normal library-evidence serialization remains, but
it no longer serializes file descriptors or their digest strings.

Plain median RSS (KiB) for read/inherent: debug 113,748/120,916 before and
113,908/120,832 after; release 109,048/116,188 before and 108,968/116,312 after.
There is no meaningful measured peak-memory improvement.

Strict-import medians for read/inherent are debug 0.64/0.95 before versus
0.65/0.97 after; release 0.06/0.09 before versus 0.08/0.10 after. These coarse
samples show no strict-import improvement. It still reconstructs and validates
semantic evidence. Library artifact hashing was capture work, not strict replay
work. No whole-suite speedup is inferred.

## Validation and next work

- Focused length-family CLI tests passed: complete 15-function/105-pair family,
  unsupported receivers/lookalikes and legacy-profile exclusions, edited core
  identity/definition/signature, and unsupported compiler/target/width resolution.
- Existing serialized CALL corruption and incomplete-library handoff tests remain;
  obsolete artifact-digest mutations now exercise contradictory core identity
  and retired evidence-version rejection instead.
- 645/645 nextest tests passed (36.928 s), zero skipped; workspace doctests passed.
- Formatting and workspace/all-target Clippy with `-D warnings` passed.
- Both quality gates passed: zero style violations across 1,965 functions,
  zero actionable pattern candidates, 15 unchanged reviewed ignores.
- All 24 Python study/bookkeeping tests passed; the benchmark script ran
  successfully and its stored cross-profile canonical signatures agree.

Earlier local checks caught a simplifiable boolean and a focused test run used
an old driver binary. The boolean was fixed and the driver rebuilt before the
successful focused/full runs. Initial log paths were overwritten by successful
reruns; `/tmp/rstyle-library-initial-failures.txt` records the observed failures
and that limitation. They are not counted as successful validation.

Next implement complete-key location/build interning, then source/context owners
and unit/function/ledger namespaces. Hash-based WL interning still needs its own
FNV/alternative/tree benchmark and collision/parity evidence. No matching
container or scoring algorithm has changed in this increment.
