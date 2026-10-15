# Graph compatibility metadata without frontend duplication

Graph producer configuration now contains `matching_build`, with the closed
`rstyle.matching-build/1` schema and three captured facts: `overflow_checks`,
`debug_assertions` and `panic_strategy`. These are the only `captured_build`
fields consumed by strict matching compatibility. The complete owning frontend
still retains compiler, target, version, contract/profile, source, package,
feature, invocation and build evidence. Producer revision and compiler language
version remain in graph producer metadata.

This removes `json!(build)` from normal graph emission. Previously both PDG and
CFG copied all frontend options, including the full census and declaration-origin
file buffers, into each graph. Source snapshots and checked frontend handles now
carry that evidence instead. No secondary identity system or digest is added.

Strict admission decodes the explicitly versioned, closed three-fact record,
then constructs the exact same sorted compatibility-setting map used before.
The schema version is not part of the mathematical score or a weaker build
identity. Native-width, package/target association, source correspondence,
semantic reconstruction, full frontend-reference association, completeness,
verification-policy separation and evidence budgets retain their existing
checks. Missing, unknown-version, unknown-field and malformed records reject.

The deliberately retired guarantee is a second complete diagnostic build copy
inside each standalone graph producer configuration. Full evidence belongs to
the referenced frontend. No runtime fallback reads the old `captured_build`
format; old local graph documents require recapture with the matching producer.
Standalone PDG/CFG schemas keep their existing open configuration object;
the new nested record explicitly versions its contract. Bundle version 4,
report function/context contracts and transport seals are unchanged.

Regressions check exact compatibility fact values/order and malformed/missing
records. The CLI bundle test checks both graph documents omit the full build
copy and retain all three original facts. Diagnostic-provenance tests now assert
that the frontend build evidence differs while graph compatibility metadata,
nodes, edges and pair decisions stay equal. They preserve the provenance check
at its owning record instead of requiring the retired duplicate representation.

## Measurements and validation

Baseline source is `20a39c6`; retained isolated samples and saved tools in
`/tmp/rstyle-before-matching-build` precede this change. Candidate is this
increment. Existing pipeline harnesses ran `after target` with
`/tmp/rstyle-fast-checksum-{measurement,broad}` after builds and gates.
[312 raw samples](matching-build-samples.json) cover six fixtures, debug/release,
trace on/off and strict imports. One warmup, three ordinary samples and separate
diagnostic runs preserve fixture paths, warm caches and ordinary analyzed
check-profile settings; strict replay has three samples. No builds, tests or
comparisons overlap timing. Environment: aarch64, eight-CPU quota, 16 GiB,
nightly-2026-09-30, locked dependencies. GNU time resolution 0.01 seconds;
RSS is a whole-command proxy.

Median ordinary no-trace seconds and representative debug bundle bytes:

| Workload | Debug before → after | Release before → after | Bundle bytes before → after |
| --- | --- | --- | --- |
| minimal | 0.24 → 0.24 | 0.20 → 0.21 | 53,002 → 41,112 |
| read | 1.13 → 1.08 | 0.31 → 0.29 | 1,939,416 → 1,351,446 |
| inherent | 1.68 → 1.55 | 0.37 → 0.33 | 2,937,494 → 1,719,194 |
| shared-fanout | 0.89 → 0.87 | 0.27 → 0.26 | 879,628 → 784,838 |
| many-pairs | 1.61 → 1.55 | 0.38 → 0.35 | 3,365,707 → 2,135,585 |
| larger-graphs | 3.64 → 3.64 | 0.70 → 0.70 | 2,140,798 → 2,093,230 |

The three nominal fixtures shrink substantially, and their ordinary medians
improve in this sample set. Minimal/larger-graph timings do not improve. These
observations do not establish a universal or whole-suite speedup. The full-file
census/origin copies left in graph metadata by version 4 are gone; the owning
frontend still references the single original-file table.

All 120 complete matching reports and 2,160 frontend records/options compare
exactly after normalizing run-directory paths in compiler arguments. Harness
admission, exact-score, completeness and strict-replay assertions pass. All 695
nextest tests passed, zero skipped; workspace doctests passed. Formatting,
warning-free workspace/all-target Clippy, 24 study tests and two harness tests
passed. Both quality gates are clean: zero violations/actionable candidates,
with the same 15 reviewed ignores. Logs: `/tmp/rstyle-matching-build-*`.
Suite duration 29.767 seconds is observational, not a controlled suite claim.
