# Executable fingerprint retirement

Clone analysis no longer opens or hashes the CLI or driver executable. It still
resolves the driver path for launching. No opt-in flag or replacement identity
cache is provided. Reports omit `producer_binary_sha256` and
`orchestration_binary_sha256`; plans omit `driver_sha256` and
`orchestrator_sha256`. Compiler, Cargo, producer revision and contract/profile
metadata remain. Source and evidence identity semantics are unchanged by this
increment.

The historical CCGraph study runners were the only consumers of these removed
report fields. Those runners and recorded evaluations have since been removed;
they remain available in Git history. No replacement executable fingerprint
or identity cache is required by the current analysis or evaluation tools.

## Measurement

2026-10-15, Linux aarch64, pinned nightly-2026-09-30, same machine, driver and
build profiles as the prior production profile. Baseline CLI copies are from
`81ea0ed`; candidate binaries contain this retirement. Tool compilation is
excluded. No builds or tests ran during measurement. Each profile used the same
workspace containing two identical scalar functions, warm filesystem caches,
one unmeasured warmup per binary and three alternating before/after ordinary
runs. Command: `cargo-rstyle clones --format json --offline`, with the same
explicit `RSTYLE_DRIVER` per build profile and no `RSTYLE_PERF`.

| Profile | Before samples (s) | After samples (s) | Median change |
| --- | --- | --- | --- |
| debug | 2.16, 2.25, 2.16 | 0.24, 0.24, 0.24 | 2.16 → 0.24 s (1.92 s) |
| release | 0.23, 0.21, 0.21 | 0.21, 0.19, 0.20 | 0.21 → 0.20 s (0.01 s) |

All 20 runs completed capture, assessment, enumeration and evaluation, admitted
both functions and produced the same accepted pair and score. Separate diagnostic
runs showed two binary-hash spans before and none after (each span emits a start
and end event). Timing resolution is 0.01 s; the release difference is small.
These measurements do not predict whole-suite speedup.

Raw local artifacts: `/tmp/rstyle-hash-retirement-0bkzcork`; ordinary timings and
separate diagnostic records are preserved in `executable-hash-retirement.json`.

Validation: 643 nextest tests passed, doctest wrapper passed, 24 Python
bookkeeping tests passed, formatting and warning-free workspace/all-target Clippy
passed. Both style gates passed with zero violations and zero actionable
candidates; the 15 existing reviewed ignores were unchanged. The executable-only
permissions regression checks actual complete runs, absent report/plan fields,
identical matching output and absence of binary-hash diagnostic spans. Digest
known-vector and edited-file checks remain.
