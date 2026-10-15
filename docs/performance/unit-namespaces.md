# Unit namespaces replace raw frontend fingerprints

`rstyle.shared-unit/2` replaces the bundle's `identity` digest with a typed
`namespace` record. `rstyle.unit.function-reference/2` qualifies the local
function ordinal with that namespace; CLI `unit_evidence` and the strict-import
replay output also export `namespace` instead of `identity`. Old bundle versions
are rejected. No permanent version-1 runtime is retained.

A producer allocates a namespace without inspecting any frontend bytes. It
contains the local process ID, a process-start allocation epoch in nanoseconds,
and a checked monotonic allocation ordinal. The epoch is read once; ordinal
exhaustion and clock/range errors fail explicitly. The allocator takes no facts
argument and performs no serialization or hashing. It is a local namespace,
not an authenticated or globally persistent content identity. Separate captures
of identical facts intentionally have distinct namespaces. An imported bundle
preserves its exported namespace so its references still resolve.

The namespace does not authorize cache reuse or prove evidence correctness.
Private-constructor function/helper/graph handles resolve only through sealed
owner-bound views. Each loaded copy owns separate tables, analyses and proof
caches even when it retains the same wire namespace. Successful proof results
remain analysis- and strict/generated-policy-local. There is no cross-store
cache keyed by the namespace. Host migration, clock manipulation and malicious
namespace replacement are outside its uniqueness claim; arbitrary imports
still require strict semantic validation.

The deliberately retired guarantee is a content fingerprint of the entire raw
frontend JSON, including diagnostic and build metadata. Changing that data no
longer requires refreshing a unit digest to import it. This is not permission
to change graph meaning: import reconstructs exact call links and checks table
bounds, graph ownership, inventory completeness, summaries, endpoints, source
bytes/ranges, build semantics and paired-document correspondence. Current-run
CLI import independently inspects every frontend against the frozen invocation
and selected unit. Bundle byte seals remain separately checked at the boundary.
Source digests retain their existing meanings. No integer or namespace occupies
a field named `sha256`.

Mutation regressions now submit edited evidence directly, without recomputing
the retired unit hash. They must still fail strict import, including when called
inside a generated scope. Existing foreign-unit/function reference checks remain.
New coverage proves identical captures receive distinct namespaces, imported
copies retain namespaces, namespace encoding round trips, malformed namespace
records fail, and retired identity fields/schema versions are rejected.

This removes whole-frontend serialization and SHA-256 at both capture and import
construction. It does not change transport checksums, wire source duplication,
capture ledger routing or matching. Those remain separate increments.

## Measurements and validation

Baseline source: `5a27ebe`. The baseline is the previous increment's measured
candidate, with identical saved binaries in
`/tmp/rstyle-before-unit-namespaces/{debug,release}`. Existing fixture directories
are reused to hold nominal source paths constant. Baseline samples were retained
without rerunning or relabeling a different implementation. The current candidate
is this commit. Embedded producer revisions are recorded in the
[312 raw command samples](unit-namespace-samples.json).

Method matches [report-function handles](report-function-handles.md): Linux
aarch64, eight CPUs, 16 GiB, pinned nightly, locked dependencies, prebuilt debug
and release analyzer/driver binaries, ordinary check-profile fixtures, warm state,
no overlapping builds/tests/timings. One warmup plus three ordinary and one
separate diagnostic sample per trace setting; three strict replays per workload
and analyzer profile. The 0.01-second wall timer and peak RSS samples are retained.
These are not allocator event counts.

```sh
python3 scripts/measure_library_resolution.py after target /tmp/rstyle-report-function-ids-measurement
python3 scripts/measure_broad_pipeline.py after target /tmp/rstyle-report-function-ids-broad
```

Median ordinary no-trace wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.24 → 0.22 | 0.21 → 0.21 |
| read | 1.19 → 1.15 | 0.34 → 0.29 |
| inherent | 1.78 → 1.70 | 0.36 → 0.37 |
| shared-fanout | 0.91 → 0.91 | 0.28 → 0.27 |
| many-pairs | 1.79 → 1.73 | 0.36 → 0.37 |
| larger-graphs | 3.80 → 3.76 | 0.71 → 0.70 |

All cases passed decision, exact score-input, stage, admission, enumeration and
completeness parity with trace on/off in both profiles. All strict-import replays
succeeded. Results do not establish a uniform command speedup or whole-suite gain.

Separate diagnostic `unit.structure` samples show debug read 24.020→0.259 ms
and inherent 38.921→0.434 ms; release read 0.817→0.084 ms and inherent
1.984→0.127 ms. These individual samples attribute the removed serialization/hash
work, and are not added to nested stage times or used as ordinary wall timings.
Structurally, one whole-frontend JSON serialization and SHA operation disappears
at each capture construction and each import construction, including strict
replay. No replacement scan or cache is added. The namespace allocator has no
frontend argument.

Validation: 667 nextest tests passed, zero skipped; workspace doctests passed;
warning-free workspace/all-target Clippy; formatting; 24 study bookkeeping and
two measurement-harness Python tests passed. Both required quality gates passed
with zero style violations and zero actionable pattern candidates. The existing
15 reviewed ignores are unchanged. Logs: `/tmp/rstyle-unit-namespaces-*`.
