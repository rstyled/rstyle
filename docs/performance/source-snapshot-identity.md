# Actual identities belong to immutable source snapshots

`SourceSnapshot` owns immutable UTF-8 bytes and a lazy actual SHA-256 identity.
Clones share both. Census files, declaration origins, compiler capture records
and function ranges now use this owner. Census checks and independent declaration
inventories compare supplied digest claims with `SourceSnapshot::identity`,
rather than recomputing the file digest at every verification boundary. Semantic
verification remains independent and policy-local: a memoized actual identity is
not a memoized successful verification result.

A function covering a whole snapshot borrows the same identity memo. A partial
function range retains its separately memoized range digest. Moving an existing
range onto an exactly equal checked file range preserves an already established
actual range identity without hashing again. No supplied digest claim populates
either memo.

Deserialization creates a fresh owner with an empty identity memo. Edits replace
the owner; independently constructed equal snapshots need not share a memo.
Equality retains complete-byte comparison, with owner identity as an exact fast
path. Bounds, UTF-8, source namespaces, contradictory-record checks, digest
claims and strict/generated verification remain unchanged. The bundle source
table restores shared owners, not verification authority.

The Rust API uses `SourceSnapshot` instead of `Arc<str>` for file buffers and
`SourceText::from_snapshot`; callers may construct it from strings or `Arc<str>`.
Readable JSON and bundle version 4 retain the same string encoding. This is an
ownership/API change, not a wire-schema or fingerprint-policy change. A second
small allocation stores the memo beside the existing shared byte allocation.

Diagnostic `source.snapshot_hash` now measures actual whole-snapshot hashing
inside the memo initializer, including hashing on import. It replaces the
capture-only `capture.source_hash` stage; historical samples retain their old
stage name. Range hashes remain separate. Diagnostics never serialize bytes to
label the event.

Regression coverage checks shared memo pointers, fresh imported/edited owners,
actual digest values, whole-range reuse and preservation of an established range
identity. Existing corrupt-claim, source-edit, malformed-import and policy
isolation tests remain required.

## Measurements and validation

Baseline source is `4b106f8`, using its retained isolated samples with saved
binaries in `/tmp/rstyle-before-source-snapshot`. Candidate is this increment.
The existing minimal/read/inherent and broad pipeline harnesses ran `after target`
against `/tmp/rstyle-fast-checksum-{measurement,broad}` after builds and gates.
[312 raw samples](source-snapshot-samples.json) retain ordinary timing separately
from diagnostics. Fixtures, paths and warm caches match. One warmup, three
ordinary samples and a separate diagnostic sample cover debug/release analyzers
and both trace modes; strict replay has three samples. Builds, tests and report
comparisons did not overlap timing. Environment: aarch64, eight-CPU quota,
16 GiB, nightly-2026-09-30 and locked dependencies. GNU time resolves 0.01
seconds; RSS is a whole-command proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.22 → 0.24 | 0.21 → 0.20 |
| read | 1.14 → 1.13 | 0.32 → 0.33 |
| inherent | 1.65 → 1.64 | 0.34 → 0.36 |
| shared-fanout | 0.87 → 0.90 | 0.27 → 0.26 |
| many-pairs | 1.66 → 1.67 | 0.36 → 0.37 |
| larger-graphs | 3.66 → 3.67 | 0.69 → 0.70 |

These mixed small changes do not establish a uniform runtime improvement.
Representative bundle sizes are unchanged. The structural guarantee is that
actual full-file hashing occurs at most once per shared owner, irrespective of
how many census/declaration checks borrow it. Separate debug diagnostics record
two snapshot-hash completions per nominal fixture: one in the compiler process,
one in the importing CLI. Read hashes 2 × 1,392 bytes, inherent 2 × 1,572 and
many-pairs 2 × 2,225. These counts exclude distinct partial-function identities
and graph manifests; no claim is made that they are all hashing work. The
baseline lacked equivalent full-boundary instrumentation, so these are observed
after counts, not a fabricated before/after invocation count.

All 120 matching reports and all 2,160 frontend records/options compare exactly,
normalizing only run-directory paths in compiler arguments. Harness exact-score,
admission, decision, completeness and strict-replay assertions pass. All 692
nextest tests passed, zero skipped, followed by workspace doctests. Formatting,
warning-free workspace/all-target Clippy, 24 study tests and two harness tests
passed. Both quality gates are clean: zero style violations, zero actionable
pattern candidates and the same 15 reviewed ignores. Logs:
`/tmp/rstyle-source-snapshot-*`. Suite timing (33.950 seconds) is observational,
not a controlled suite-wide speedup claim.
