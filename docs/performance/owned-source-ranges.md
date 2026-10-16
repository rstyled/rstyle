# Owned function-source ranges

Baseline source: `53f71233251ec7a6581ce093002629dd96bd5cb9` on
`perf/hash-and-matching-optimization`. This increment adds `SourceText`, an
immutable checked UTF-8 range into a shared original-file snapshot. In a frozen
nominal unit, functions resolve their exact captured source path against the
canonical census and compare every byte of the claimed function range before
sharing the census buffer. Reversed, out-of-bounds, split UTF-8 and mismatched
ranges fail. Functions without a census retain their own immutable snapshot.

A range owns its actual SHA-256 identity through `OnceLock`. It never installs a
serialized digest claim in this slot. Driver capture calculates its source
identity through this owner; range rebinding retains an already computed actual
identity only after exact byte equality. Field observation verification, nominal
range binding and strict producer verification compare claims with that actual
identity rather than rehashing the same bytes. Producer graph validation uses
the same immutable source range and compares every source alias's claim with
its actual identity. Public `validate` / `validate_sources` retain their raw
byte-boundary checks, including absent bytes, malformed identities and spans.

This identity memo is a pure property of immutable bytes, not successful semantic
verification or policy authority. Digest claims, build facts, graph structure,
source ranges, aliases and paired documents remain independently checked. No
strict/generated proof cache, owner association, evidence budget or admission
rule is weakened. Replacing bytes creates a new owner; editing only a digest
claim cannot change the cached actual identity. Deserialization always creates
an uncomputed owner and checks imported claims afresh.

## Representation and limits

`SourceFacts.bytes` is now `SourceText` in the Rust API. Constructors accept a
string through `.into()`; read access uses `Deref<str>` / `as_str()`. Snapshot
edits replace the value. Its JSON representation remains the original string,
so existing wire schemas and SHA-256 meanings do not change. Range bounds and
owners cannot be supplied through serialized cache fields.

Functions and declarations can now share the census's original-file buffer.
The independent checked lexical environment still supplies environment-local
`SourceId` handles. Function ranges retain their owner directly; they are not
persistent identifiers and cannot resolve a bare ID from another store.

This does not yet eliminate capture-time reconstruction of compiler file buffers,
repeated declaration/manifest identities, source copies in HIR snippets or CFG
wire records, or repeated file/function bytes in the bundle's JSON. A versioned
source dictionary and further boundary consolidation remain required. No claim
is made that all source hashing or copying has been removed.

## Evidence

Regressions cover shared original byte addresses, BOM/CRLF/Unicode, invalid ranges,
identity memo reuse across immutable clones, fresh imported identity calculation,
source-path mismatch, edited source and honest unverified claims. An internal
parity regression compares the owner-backed validator with the public raw-byte
validator under both fixture and compiler contexts, using aliases, changed
bytes, out-of-bounds spans, missing source IDs and mismatched alias identities.
The hand-authored schema example is adapted to current typed policy records
inside that regression only; no production compatibility adapter is added.

## Matched measurements

Linux aarch64, eight CPU quota, 16 GiB; pinned nightly-2026-09-30 and locked
dependencies. Prebuilt debug/release analyzer and driver; fixture settings stayed
unchanged. Both builds recorded producer revision `53f7123+dirty` (the preserved
untracked user file was present); baseline source was `53f7123`, candidate source
is the implementation in this commit. Source revisions and captured build labels
are distinguished deliberately. The test wrapper and candidate builds finished
before after-measurements started; no builds or tests ran alongside measurements.

Commands: `python3 scripts/measure_library_resolution.py before|after BIN_ROOT OUTPUT`
and `python3 scripts/measure_broad_pipeline.py before|after BIN_ROOT OUTPUT`.
Baseline tools: `/tmp/rstyle-before-source-ranges/{debug,release}`. Candidate
tools: `target/{debug,release}`. Outputs: `/tmp/rstyle-source-ranges-measurement`
and `/tmp/rstyle-source-ranges-broad`. Build/setup time is excluded. Each case
uses one warmup and three ordinary samples per trace setting, one separate
diagnostic run per trace setting, and three strict imports: 312 commands total
over both revisions and both suites. Phases were sequential, not interleaved;
`/usr/bin/time` has 0.01-second resolution. This small sample cannot establish
statistical significance. Diagnostic times are not included in ordinary medians.

All complete/admitted/exhaustive-pair checks, named pair decisions, displayed
scores, exact score inputs and stage outcomes matched before/after, including
trace on/off. Strict replay completed for every case. Exact preliminary scores
are compared within matching profile/workload paths; fixture roots differ across
debug/release and affect nominal string features. Cross-profile decisions,
displayed scores and stage outcomes still match.

Median seconds before → after:

| Workload / analyzer | Ordinary | Trace | Strict import |
| --- | --- | --- | --- |
| minimal / debug | 0.23 → 0.23 | 0.23 → 0.24 | 0.00 → 0.01 |
| minimal / release | 0.20 → 0.21 | 0.19 → 0.22 | 0.00 → 0.00 |
| read / debug | 1.27 → 1.32 | 1.31 → 1.36 | 0.57 → 0.59 |
| read / release | 0.30 → 0.33 | 0.33 → 0.34 | 0.07 → 0.06 |
| inherent / debug | 1.88 → 1.90 | 1.92 → 1.91 | 0.87 → 0.88 |
| inherent / release | 0.38 → 0.39 | 0.38 → 0.37 | 0.09 → 0.08 |
| shared-fanout / debug | 0.89 → 0.92 | 0.94 → 0.96 | 0.69 → 0.70 |
| shared-fanout / release | 0.27 → 0.29 | 0.30 → 0.31 | 0.06 → 0.06 |
| many-pairs / debug | 1.94 → 1.95 | 2.12 → 2.12 | 0.63 → 0.62 |
| many-pairs / release | 0.38 → 0.38 | 0.38 → 0.41 | 0.06 → 0.07 |
| larger-graphs / debug | 3.79 → 3.79 | 4.60 → 4.59 | 4.19 → 4.22 |
| larger-graphs / release | 0.72 → 0.72 | 0.77 → 0.77 | 0.41 → 0.40 |

No wall-time improvement is demonstrated for this increment. Some small-family
medians increased; larger-graph medians were unchanged. The change establishes
immutable source ownership and eliminates repeated range hashing structurally.
It does not justify a suite-wide speedup claim. Raw wall time, peak RSS and report
size samples are retained in `owned-source-range-samples.json`.

Storage proxy: frozen read/inherent/many-pair functions previously held 1,164 /
1,294 / 2,142 bytes in separate function strings, in addition to their census
snapshot. These exact ranges now share their 1,392 / 1,572 / 2,225-byte census
owner. This excludes range objects, cached digest strings, HIR/CFG/JSON copies
and allocator overhead. Scalar controls without a census retain their own
function snapshots. Identity calculation is bounded by one full SHA-256 per
immutable range owner; rebinding can copy an already-computed actual identity
only after complete byte equality. No total-run digest invocation count or
allocation-byte measurement is inferred from this storage proxy.

## Validation

The final wrapper passed 659 tests, zero skipped, and workspace doctests.
Warning-free workspace/all-target Clippy, formatting, diff checks, 24 Python
study tests and two harness tests passed. Both gates were clean: zero style
violations across 2,018 functions, zero actionable pattern candidates, 15 existing
reviewed ignores unchanged. Initial compile/fixture-adapter and pattern failures
were retained in `/tmp/rstyle-source-ranges-*` logs and fixed without removing
assertions, adding ignores or changing thresholds. The sole new pattern was
a duplicated Serde wrapper; the new string-backed type uses Serde’s standard
`from = "String"` conversion.
