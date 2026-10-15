# Shared concrete declaration and field ranges

Concrete struct source and field-type declaration text now use `SourceText`
instead of `String`. Compiler capture retains the checked original declaration
range and memoizes repeated requests for the same file/span. Nominal capture
uses that range's actual identity; subsequent concrete-type validation reuses it
instead of hashing a copied declaration again.

During immutable store normalization, a local range table binds declarations
only when the complete nominal key, original span and exact original text agree
with the supplied origin record. Field-type text borrows an exactly equal slice
of the declaration. Equal type text may share storage; its internal byte range
is not a semantic field occurrence ID. Existing field position/name, parsed
source-type, nominal namespace, ordered projection and occurrence checks remain.
Malformed/mismatched evidence remains byte-for-byte unchanged for the semantic
validators to reject. Sharing never installs supplied digest claims or proofs.

The range table is local to one normalization operation. Its complete key is a
live immutable byte allocation and checked start/end offsets; retained ranges
keep allocations alive, so addresses cannot be recycled during lookup. Neither
addresses nor local cache keys are exported. Independently owned/edited buffers
cannot reuse a range through a digest, name or file spelling. Function ranges
use the same table after source-path association. Actual identity memos are
separate from strict/generated policy-local verification results.

Normalization moves type records through a bounded traversal without cloning
whole type trees. Existing depth/entry limits remain. It performs no new source
hashing solely to intern data and no JSON round trip. Standalone/imported text
begins with its own immutable source owner; normal store construction binds it
to shared file owners after exact equality. The public/interchange validators
continue to parse and verify declarations at their required boundaries.

The Rust API changes the struct/field source values to `SourceText`; string
construction uses `.into()`. JSON remains the exact same strings, and bundle,
report and standalone schema versions do not change. Explicit null presence and
malformed option records retain their existing representations.

Focused regressions cover original BOM/CRLF/UTF-8 buffers, shared declaration and
field owners/actual identity memos, unchanged readable bytes and invalid source,
field, digest and range records. Normalization reuses existing element boxes;
its temporary empty value is confined to an owned transformation and cannot be
published on failure.

## Measurements and validation

Baseline source is `869d88f`, using retained isolated samples and saved tools in
`/tmp/rstyle-before-declaration-ranges`. Candidate is this increment. Existing
pipeline harnesses ran `after target` against
`/tmp/rstyle-fast-checksum-{measurement,broad}` after builds and gates.
[312 raw samples](declaration-ranges-samples.json) cover six fixtures, debug/release,
trace modes and strict replay. One warmup, three ordinary samples and separate
diagnostics preserve fixture paths, warm caches and analyzed check-profile
settings; strict replay has three samples. Builds, tests and comparisons do not
overlap timing. Environment: aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30 and locked dependencies; GNU time resolution 0.01 seconds,
whole-command RSS proxy.

Median ordinary no-trace seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.24 → 0.23 | 0.21 → 0.21 |
| read | 1.08 → 1.11 | 0.29 → 0.31 |
| inherent | 1.55 → 1.52 | 0.33 → 0.32 |
| shared-fanout | 0.87 → 0.87 | 0.26 → 0.29 |
| many-pairs | 1.55 → 1.53 | 0.35 → 0.32 |
| larger-graphs | 3.64 → 3.66 | 0.70 → 0.70 |

Timing changes are mixed; no uniform speedup is established. Representative
bundle bytes remain unchanged. Structural claims are shared range ownership and
reuse of actual declaration identity, not wire compression or allocator-event
counts. Separate compiler-range and declaration/field-owner tests prove reuse.

All 120 matching reports and 2,160 frontend records/options compare exactly after
normalizing run-directory paths in compiler arguments. Harness admission, exact
scores, completeness and strict replay assertions pass. All 697 nextest tests
passed, zero skipped, followed by workspace doctests. Formatting, warning-free
workspace/all-target Clippy, 24 study tests and two harness tests passed. Both
quality gates report zero style violations/actionable pattern candidates; the
same 15 reviewed ignores remain unchanged. Logs: `/tmp/rstyle-declaration-ranges-*`.
Suite duration 32.132 seconds is observational, not a whole-suite speedup claim.

Storage normalization now enforces the same declaration-source byte and field
count limits as semantic declaration validation *before* origin lookup, range
rebinding or per-field substring searches. The shared budget helper keeps these
limits consistent. Regressions cover source text one byte over the maximum and
field count one entry over the maximum. Both return the explicit budget error
at normalization; valid range sharing and malformed-evidence checks remain.

Validation of the three review fixes together: 701/701 nextest tests passed,
zero skipped; workspace doctests, formatting and warning-free workspace/all-target
Clippy passed. Both quality gates report zero violations/actionable candidates;
the same 15 reviewed ignores remain. Logs: `/tmp/rstyle-findings-*`. Initial
Clippy test-length and style-spacing failures were corrected without suppressions
or removed assertions before the final clean run.
