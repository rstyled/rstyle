# Report-local function handles

Reports declare `function_identity_schema: "rstyle.report-function/2"`.
Function keys now use `function:uN:fM`: a checked compilation-unit ordinal and
function ordinal. Internal keys are typed integers. Creating a key no longer
constructs a JSON build-context tree, serializes it, or computes SHA-256.

`unit_contexts` maps each numeric `id` to an index `observed_unit` in this
report's `observed_units`. A function's `unit_evidence.report_unit` selects that
context. Existing source navigation, compiler invocation, target, package,
features, build contexts and bundle/function references remain available.
Units are assigned in exact descriptor order rather than compiler completion
order. Duplicate or contradictory descriptors and duplicate source occurrences
within a unit are rejected; no digest collision can combine them.

These are labels within a report, not persistent content fingerprints. Separate
reports can reuse labels for different functions or builds. Consumers comparing
runs must resolve source and compilation context in each report; key equality
alone makes no cross-run identity claim. This deliberately retires the previous
content-derived report-key guarantee. The release/context regression now checks
the actual selected package and compiler optimization context through the context
table instead of assuming keys from separate reports cannot overlap.

Bundle identity, source identity, strict import verification, graph admission,
matching mathematics, pair enumeration and completeness checks are unchanged.
The bundle schema remains `/1`; the report-function schema changes independently.
Pair orientation can change with the new ordering; compare named unordered pairs
when checking cross-schema score parity.

The Python study readers treat keys as opaque labels for joins. Their historical
source-pinned transport protocols remain historical; no production compatibility
runtime is added. Their independent binary snapshot/drift checks remain intact.
The current measurement harness likewise resolves pairs through named functions.

Focused coverage checks dense qualified labels, overflow, completion-order
independence, feature/package/test contexts, descriptor contradictions, and
end-to-end package/module/concurrent/release context resolution.

## Measurements and validation

Baseline source is `c0b9cb7`; candidate is this commit's report-handle change.
The raw [312 command samples](report-function-handle-samples.json) record the
producer revisions embedded in the prebuilt tools. Producer metadata can name
the preceding revision plus `dirty`; the source comparison is the working
increment against `c0b9cb7`. Saved baseline binaries are in
`/tmp/rstyle-before-report-function-ids/{debug,release}`.

Linux aarch64, eight-CPU quota, 16 GiB memory, pinned nightly-2026-09-30, locked
dependencies. Build/setup time is excluded, with no overlapping tests or builds.
Debug and release analyzer/driver binaries analyze the same ordinary check-profile
fixtures. Each trace setting has one warmup, three ordinary samples and one
separate diagnostic sample; each workload/profile also has three strict imports.
`/usr/bin/time` has 0.01-second resolution and records peak RSS in KiB. These
RSS samples are a process proxy, not allocator event counts.

Commands, reusing the output directories to enforce before/after exact parity:

```sh
python3 scripts/measure_library_resolution.py after target /tmp/rstyle-report-function-ids-measurement
python3 scripts/measure_broad_pipeline.py after target /tmp/rstyle-report-function-ids-broad
```

Median ordinary no-trace wall seconds (three samples):

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.22 → 0.24 | 0.20 → 0.21 |
| read | 1.23 → 1.19 | 0.30 → 0.34 |
| inherent | 1.84 → 1.78 | 0.37 → 0.36 |
| shared-fanout | 0.92 → 0.91 | 0.29 → 0.28 |
| many-pairs | 1.82 → 1.79 | 0.37 → 0.36 |
| larger-graphs | 3.82 → 3.80 | 0.72 → 0.71 |

Results are mixed and small; this does not establish a general runtime speedup
or a whole-suite speedup. The structural result is exact: one whole-build JSON
tree, temporary serialization vector and SHA operation per function key are
eliminated (2/20/21/8/49/8 functions in these workloads). No content bytes enter
the key constructor. Typed comparison and ordinal allocation replace that work.
Matching decisions, exact score inputs, stage outcomes, admitted counts,
exhaustive enumeration and completeness passed before/after parity for every
trace setting and profile. All strict-import replays succeeded.

Validation: 665 nextest tests passed, zero skipped; workspace doctests passed;
warning-free workspace/all-target Clippy; formatting; 24 study bookkeeping and
two measurement-harness Python tests passed. Both required style gates passed
with zero violations and zero actionable candidates; 15 existing reviewed ignores
are unchanged. Logs are retained under `/tmp/rstyle-report-function-ids-*`.

Unit-store frontend identity serialization, capture ledger hashing, transport
checksums, source wire dictionaries and matcher interning/allocation work remain
separate pending increments in the full optimization plan.
