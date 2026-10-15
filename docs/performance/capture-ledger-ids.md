# Run-local capture ledger routing

Capture plans declare `ledger_schema: "rstyle.capture-ledger/2"`. Both the
compiler wrapper and CLI reconciler resolve a target by complete equality
against that exact frozen plan's target table. Its checked `u32` ordinal and
the test/check mode form `unit-tN-test.json` or `unit-tN-check.json`.

This removes two JSON constructions, temporary byte serializations and SHA-256
operations per captured unit: producer filename creation and consumer lookup.
Neither path scans source, build metadata or executables. The target table
already belongs to the plan; its ordinal needs no persistent registry. IDs have
meaning only within that run directory and plan. Reordering the plan may change
filenames. These names make no content-fingerprint or cross-run identity claim.

Missing, changed or duplicate exact target descriptors fail lookup; unsupported
ledger schemas are rejected. Publication still uses `create_new`, so a second
capture for the same target/mode cannot overwrite the first ledger. The CLI
still checks the loaded target, test mode, run-directory association, planned
build settings, expected/observed compilation inventory, selected frontend
invocation and all artifact/graph integrity checks. Equal target spelling alone
does not authorize a handoff. Evidence directories already used `tempfile`
`invocation-*` names; this change does not claim to replace directory hashes.

The internal plan and ledger filename contract changes deliberately. No legacy
hash-filename adapter is retained. Ledgers remain JSON records and publication
remains contingent on compiler success and complete artifact inventory.

Focused regressions cover package target ordinals, test/check separation,
changed kind/manifest descriptors, duplicates, unsupported schemas and failed
duplicate publication. Existing concurrent capture, all-target, selected package,
feature/context, interrupted compiler and stale-source tests remain enabled.

## Measurements and validation

Baseline `33b7a4d` is the retained unit-namespace candidate measurement. The same
minimal fixture paths, prebuilt tool profiles, pinned nightly, locked dependencies,
eight-CPU aarch64 host and warm state are used. Tool compilation is excluded and
no tests/builds overlap measurements. The [52 command samples](capture-ledger-samples.json)
retain one warmup, three ordinary and one separate diagnostic run per trace mode
and debug/release profile, plus three strict imports per profile. Ordinary median
no-trace seconds: debug 0.22→0.23, release 0.21→0.20. With trace: debug
0.23→0.24, release 0.20→0.20. At 0.01-second resolution these results show no
measurable command improvement. The structural removal is two descriptor hashes
and serializations per captured unit; no whole-suite speedup is claimed.

The measurement harness checks decisions, exact score inputs, admitted functions,
stage outcomes, exhaustive pair counts and completeness against the baseline.
All checks and strict-import replays passed. Command:

```sh
PYTHONPATH=scripts python3 -c 'import sys, measure_library_resolution as m; sys.argv=["measure", "after", "target", "/tmp/rstyle-report-function-ids-measurement"]; m.main([m.workloads()[0]])'
```

670 nextest tests passed, zero skipped; workspace doctests passed; warning-free
workspace/all-target Clippy and formatting passed. Three focused ledger tests,
24 study bookkeeping and two measurement-harness Python tests passed. Required
gates report zero style violations and zero actionable pattern candidates; the
15 existing reviewed ignores are unchanged. Logs: `/tmp/rstyle-ledger-*`.
