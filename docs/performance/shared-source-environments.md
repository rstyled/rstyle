# Shared checked source environments

Base: `3211768e7d5932088210a05e21645ad1462da2ab`. This increment shares immutable source censuses and lexical
verification; the remaining source range/transport ownership work is still part
of the full plan. It does not claim that all source copies or hashes are gone.

Previously each nominal function resolution decoded a full JSON census, hashed
its files, parsed the entire closed module inventory and built a lexical index.
An inherent-owner lookup decoded and verified the census again even after
resolving the function. Declaration-origin checks separately hashed and parsed
copies of the same original files.

The driver now installs one typed `Arc<Census>` for its selected unit. Frontend
options preserve typed ownership and borrow this record without materializing
JSON. A frozen unit interns complete census records by exact selected context
and record equality. Conflicting snapshots under the same selected context are
rejected; identical records share the same owner. Unknown/malformed evidence
retains the existing JSON compatibility/error path. Compatibility mutations
detach a record and normalization re-establishes typed ownership before freezing.

Each builder/analysis owns `Sources`, which resolves the exact interned owner to
a checked lexical `Environment`. Successful environments are reused separately
for strict and generated policy; failures and unwinds publish no successful
check. Independent analyses/owners cannot borrow one another's cached results.
Construction validates package context and budgets, every source fingerprint,
paths, duplicates and the complete ordinary-module closure, then builds the
lexical declaration/import index once. No serialized field constructs this
checked capability. Each function still checks exact selected context,
interface/declaration inventory, nominal association, type spellings, original
source bytes/ranges and exact lexical function/receiver ownership.

Within an environment, private `SourceId(u32)` values index the census's owned
file snapshots. Path resolution returns an owner-bound `SnapshotView`; checked
ranges reject reversed/out-of-range spans and UTF-8 splits, retaining BOM/CRLF
and original Unicode bytes. No API resolves a caller-supplied bare source ID.
IDs are local to this environment and do not fingerprint content or persist
through edits, reordering, or independent owners. Declaration checks compare
exact full bytes and the already-verified digest metadata, then compare checked
ranges and independently parsed struct item spans. They do not rehash or reparse
that original file. Inherent-owner lookup reuses the resolution's environment.

The standalone public resolution/origin APIs still construct/check their own
source environment when given unvalidated records. Sharing never crosses raw
record mutation, source edits, selected contexts, analysis owners or verification
policy. Persistent source/declaration SHA meanings and all transport schemas
remain unchanged. This is an internal ownership change; serialized census JSON
retains its previous sorted-key representation. The added Serde `rc` feature
supports typed shared census ownership; no hashing dependency is added.

## Remaining source work

Raw function source strings and declaration-origin transport still contain
source byte copies. Per-function nominal binding, interface/declaration digests,
public graph-boundary checks and whole-unit identity serialization/hashing have
not all been consolidated yet. Source wire dictionaries and checked function
source ranges need a further format/ownership increment. The source requirement
is not marked fully complete by this change.

## Regression and validation

New coverage checks shared typed census ownership without JSON rendering,
byte-identical compatibility encoding, contradictory/foreign contexts,
owner/policy-local environment reuse, changed source rejection, original-byte
range bounds including UTF-8/BOM/CRLF, exact parsed struct spans, origin digest
and byte tampering, and local source-handle changes under file reordering.
Existing source-edit, module ownership, receiver, corruption and strict-import
regressions remain enabled. Formatting, warning-free workspace/all-target Clippy,
and the complete nextest/doctest wrapper passed: 654 tests passed, zero skipped,
workspace doctests passed. Both required style gates passed with zero violations
and zero actionable candidates; 15 existing reviewed ignores remain unchanged.
Twenty-four Python study tests plus two new exact-score harness regressions
passed. Earlier failing test/style logs were retained while correcting expected
JSON key ordering and splitting functions with four mutation targets. No
assertions, thresholds, suppressions or ignores were removed or weakened.

## Measurements

Prebuilt tools from source base `3211768` were frozen at
`/tmp/rstyle-before-source-environment`; candidate tools are in `target`.
The base's captured producer revision is `2a25005420beddac8a25a97c5deb045ead670b47+dirty`
and the candidate's is `3211768e7d5932088210a05e21645ad1462da2ab+dirty`: these
are the actual introducing-build metadata, retained without substitution.
The frozen base binaries contain the preceding diagnostic increment before its
commit; the source changes for this comparison start from committed `3211768`.
Linux aarch64, eight-CPU quota, 16 GiB limit, pinned nightly-2026-09-30, locked
dependencies, unchanged debug/release analyzer profiles, ordinary fixture Cargo
check settings in both cases. Builds, tests and setup are excluded. No concurrent
builds/tests ran during timings. Phases ran before then after, not interleaved;
cache/process drift limits attribution, especially RSS. Time resolution: 0.01 s.

Commands:

```sh
python3 scripts/measure_library_resolution.py before /tmp/rstyle-before-source-environment /tmp/rstyle-source-environment-measurement
python3 scripts/measure_library_resolution.py after /home/dev/src/rstyle-main/rstyle/target /tmp/rstyle-source-environment-measurement
python3 scripts/measure_broad_pipeline.py before /tmp/rstyle-before-source-environment /tmp/rstyle-source-environment-broad-3
python3 scripts/measure_broad_pipeline.py after /home/dev/src/rstyle-main/rstyle/target /tmp/rstyle-source-environment-broad-3
```

Use `PATH=/home/dev/.cargo/bin:$PATH`. An initial broad launch without that PATH
failed before Cargo metadata ran; its logs were preserved. Each workload/profile
has a warmup, three ordinary samples per trace mode, a separate diagnostic sample
per mode, and three strict imports. [Small-family samples](shared-source-environment-samples.json)
and [broad samples](shared-source-broad-samples.json) retain all 312 command
records, including ordinary/diagnostic distinction, wall time, RSS and report
sizes. No diagnostic stage times are summed as independent work.

Ordinary wall medians in seconds, before → after:

| Analyzer | Workload | Plain | Trace |
| --- | --- | --- | --- |
| Debug | Minimal, 2 functions/1 pair | 0.24 → 0.22 | 0.24 → 0.24 |
| Debug | Read, 20/190 | 1.57 → 1.32 | 1.57 → 1.34 |
| Debug | Inherent, 21/210 | 2.14 → 1.93 | 2.18 → 1.95 |
| Release | Minimal | 0.21 → 0.20 | 0.21 → 0.22 |
| Release | Read | 0.36 → 0.31 | 0.35 → 0.31 |
| Release | Inherent | 0.42 → 0.40 | 0.44 → 0.39 |
| Debug | Shared fanout, 8/28 | 0.90 → 0.89 | 0.96 → 0.93 |
| Debug | Many nominal pairs, 49/1176 | 2.56 → 1.96 | 2.74 → 2.15 |
| Debug | Larger scalar graphs, 8/28 | 3.75 → 3.78 | 4.56 → 4.60 |
| Release | Shared fanout | 0.30 → 0.30 | 0.30 → 0.29 |
| Release | Many nominal pairs | 0.45 → 0.39 | 0.50 → 0.41 |
| Release | Larger scalar graphs | 0.72 → 0.71 | 0.78 → 0.78 |

Fanout has eight levels with six calls per level; it remains memory-free scalar
helper code. Many-pair input has one nominal field reader and 48 callers, each
with four PDG nodes. Larger graphs have 16 scalar updates and 67 PDG nodes per
function. Enumeration is exhaustive. These samples support a source-specific
improvement on the nominal families; they do not establish a whole-suite speedup.

Strict import medians, minimal/read/inherent: debug 0.00/0.65/0.95 →
0.01/0.56/0.87 s; release 0.00/0.06/0.09 → 0.00/0.06/0.09 s. Zero is below
timer resolution. Fanout/many-pair/larger-graph replay: debug 0.69/0.83/4.17 →
0.68/0.60/4.18 s; release 0.06/0.10/0.41 → 0.06/0.06/0.40 s. Each replay
completed all functions/unique graphs; replay is a strict evidence stage, not a
complete fresh clone-analysis command.

Maximum RSS across modes/iterations, small debug read/inherent: 113924/121372 →
113992/121184 KiB; release: 109268/116556 → 109344/116728 KiB. Broad debug
fanout/many-pair/larger-graph: 84876/99972/91396 → 70256/86900/77048 KiB;
release: 80248/95388/85780 → 67100/82096/71604 KiB. Broad RSS also declines on
scalar controls without census reuse, so it is not attributed solely to this
change. Small-family peak memory is essentially unchanged.

An initial larger probe had 16 functions, 32 updates each, 131 PDG nodes/function
and 120 pairs. It completed in 46.58 s plain and 59.74 s trace with the debug
base; trace peak RSS was 437260 KiB. Remaining pilot repetitions were deliberately
stopped before choosing the manageable repeated fixture above. The
[probe record](shared-source-sizing-probe.json) is retained as matching-cost
evidence, not a matched speedup claim. Reproduce its source with the same loop
in `measure_broad_pipeline.py` using 16 functions and 32 updates. This larger
shape remains relevant for the pending matcher work.

## Structural counts and parity

For one identical census in one policy/analysis, full-file census hashing,
module parsing and lexical indexing now happen once. Inherent-owner lookups and
checked declaration-origin lookups do not repeat those operations. The existing
successful-check test seam plus new shared-pointer/policy/owner regressions prove
reuse; this is independent of timing. Source files are not reread from disk by
these checks. This increment does not count all other hashes or allocations in
the pipeline and does not claim to eliminate public boundary verification.

Frozen read-family functions previously owned 20 copies of a 1392-byte census
file (27840 file bytes); inherent owned 21 copies of 1572 bytes (33012); the
49-function nominal workload owned 49 copies of 2225 bytes (109025). They now
share one file-buffer owner per census. These are scoped allocation/byte proxies
for census storage, excluding origin/function copies, JSON presentation buffers,
struct overhead and allocator behavior. Wire reports still repeat source text.
The harness's library fingerprint counters concern the already-retired artifact
hashes and are zero on both revisions; they are not source-verification counts.

All sampled admissions, completeness flags, named pair decisions, displayed
scores and stage outcomes match across revisions and analyzer profiles. Offline
inspection additionally compares every exact rational score, numerical cosine
component and per-round score input within each matched profile/workload across
before/after, trace modes and iterations. The harness now performs those checks
for future runs, with tests that detect changed exact fractions despite equal
displayed scores and preserve pair orientation by function name. Raw histogram
color IDs are not treated as persistent identities.

Exact preliminary string-feature scores are not compared across independent
debug/release fixture directories: their package paths differ, and nominal type
strings deliberately include that namespace. The retained final-score/decision
cross-profile checks pass. No scoring or admission code changed. No source
integrity guarantee was deliberately removed; redundant proof execution within
an immutable checked environment was retired. Remaining source ownership and
identity/transport/matching requirements remain active.

Census interning compares inventory meaning by exact path, bytes and claimed
digest, rather than interpreting file-vector order as a contradiction. Equivalent
reordered inventories keep separate immutable owners so each readable wire order
survives. Identically ordered records still share an owner. Different identities,
file sets, bytes or claims in the same selected-unit context remain contradictory;
duplicate-path inventories are not accepted as equivalent to unique inventories.
Order-independent comparison checks file/byte budgets first. A regression runs
the source verifier on both orders, checks lossless options normalization, and
retains conflicting-byte and duplicate-path rejection.

Validation of the three review fixes together: 701/701 nextest tests passed,
zero skipped; workspace doctests, formatting and warning-free workspace/all-target
Clippy passed. Both quality gates report zero violations/actionable candidates;
the same 15 reviewed ignores remain. Logs: `/tmp/rstyle-findings-*`. Initial
Clippy test-length and style-spacing failures were corrected without suppressions
or removed assertions before the final clean run.
