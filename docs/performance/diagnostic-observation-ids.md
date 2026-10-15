# Diagnostic observation IDs

Base: `2a25005`. Previously `RSTYLE_PERF=1` SHA-256 hashed every borrowed helper
evidence description at observation entry, using the digest to group visits.
That work occurred before the stage timer started, so the diagnostics could
both add wall time and omit their own hashing cost from the attributed stage.
No correctness or reuse consumer used the diagnostic digest.

Performance events now declare schema `rstyle.performance-event/2`. Each entered
operation receives a private `ObservationId(u64)` from a process-wide relaxed
atomic counter. Start/final events carry the same `observation_id`; pair it with
`pid` and the diagnostic run when correlating events. IDs do not persist across
runs and never confer proof or authorize reuse. Nested depth and strict/generated
policy labels remain. A failed or unwound operation still emits its final event.

`evidence_digest` is removed. `evidence_bytes` remains an O(1) borrowed slice
length: diagnostics do not scan, copy, serialize or hash evidence to label an
event. Visit counts now group by stage and supplied identity/navigation label
within the existing thread-local counter. Equal labels may represent distinct
observations; differing evidence under the same label no longer starts another
content-based group. Use observation IDs to pair events rather than interpreting
labels or visit counts as exact evidence identities. Human-readable labels remain
for stage attribution. The normal disabled-diagnostic path still creates no
observation or counter entry.

This deliberately retires diagnostic content-equivalence grouping. Cryptographic
source/evidence identities, transport consistency and verification results are
unaffected. No consumer in scripts depends on the retired event field; historical
raw logs retain their old interpretation. New and old diagnostic streams must be
interpreted according to their declared version (old streams had no schema key).

The focused regression changes evidence bytes while retaining a label, checks
consecutive visits and distinct observation IDs, verifies a shared start/final
ID and the absent digest field, and retains nested failure/depth checks.
Formatting, warning-free workspace/all-target Clippy, full nextest/doctest wrapper
(649 passed, zero skipped), and both style gates passed. Style has zero
violations and patterns has zero actionable candidates; the 15 reviewed ignores
are unchanged. No suppressions or thresholds were added.

## Measurements and limits

Prebuilt base and candidate tools ran sequentially through
`scripts/measure_library_resolution.py`; method matches
[unit build contexts](unit-build-contexts.md): Linux aarch64, eight CPU quota,
16 GiB limit, pinned nightly, locked dependencies, one command at a time, builds
excluded. Each debug/release workload has one warmup, three ordinary samples per
trace setting, one separate diagnostic sample per setting and three strict
imports. [All 156 raw samples](diagnostic-observation-samples.json) include RSS
and report sizes. Before/after phases were not interleaved and timing resolution
is 0.01 s.

The baseline shared-unit minimal/read/inherent diagnostic streams contain **zero
non-null evidence digests and zero diagnostic evidence bytes hashed**. These
workloads already use owner-bound shared helper views rather than the standalone
helper-description verification path. Therefore this cleanup cannot explain
normal runtime costs on these fixtures, and no speedup is claimed. Structurally,
the remaining standalone diagnostic path changes one complete-evidence SHA per
observation to zero; counting evidence length remains constant time. This is
not a measured global allocation or hashing count for other workloads.

Ordinary wall medians, seconds before → after:

| Analyzer | Workload | Plain | Trace |
| --- | --- | --- | --- |
| Debug | Minimal (2 functions/1 pair) | 0.23 → 0.24 | 0.23 → 0.23 |
| Debug | Read (20/190) | 1.54 → 1.55 | 1.58 → 1.55 |
| Debug | Inherent (21/210) | 2.19 → 2.18 | 2.22 → 2.20 |
| Release | Minimal | 0.21 → 0.20 | 0.22 → 0.21 |
| Release | Read | 0.36 → 0.36 | 0.35 → 0.36 |
| Release | Inherent | 0.40 → 0.42 | 0.41 → 0.41 |

Separate diagnostic plain/trace wall samples: debug read 1.53/1.59 →
1.58/1.55 s, inherent 2.21/2.18 → 2.17/2.18 s; release read 0.33/0.37 →
0.34/0.35 s, inherent 0.42/0.44 → 0.42/0.43 s. One diagnostic sample per mode
cannot establish a timing improvement. Strict replay medians, minimal/read/
inherent: debug 0.01/0.64/0.97 → 0.01/0.64/0.96 s; release 0.00/0.06/0.09 →
0.00/0.06/0.09 s (zero below timer resolution). Maximum RSS over all modes and
iterations, debug read/inherent: 114040/121340 → 114060/121144 KiB; release:
109468/116612 → 109404/116704 KiB. No memory improvement is established.

All admissions, named pair scores, stage outcomes, decisions and completeness
flags matched across revisions, trace settings and analyzer profiles. Every
strict replay completed. Every after diagnostic stream was inspected for the
v2 schema, absent digest field, unique `(pid, observation_id)` start events and
exactly corresponding final events. The full source/identity/transport/matching
optimization goal remains active.
