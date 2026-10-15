# Borrow family-test evidence

After streamed hashing (`cb6464c`), the family CLI regressions still repeatedly
imported the same strict shared bundle for read-only assertions. The read-family
location helper loaded a 20-function unit six times, once per inspected caller.
The inherent-call helper loaded its 21-function unit twice despite the caller
already owning it. Edited-source propagation loops also reloaded the new unit
once per inspected function.

These helpers now borrow the existing owner. Read-family locations and stored
reference paths borrow pairs from the initial store; inherent-call evidence
borrows the same owner used by its main verification loop. Each edited-source
loop strictly loads its new bundle once, then borrows its callers' pairs.
The pair accessor asserts exact report/store identity before resolving the
function index. There is no global fixture cache or sharing across source edits.

This removes seven full imports from the read-family test and four from the
inherent-family test. All existing path, operand-position, location-count,
projection-feature, pair-count, admission, warm-run, source-edit and completeness
assertions remain. Corruption checks retain independent strict imports and
rejection under generated ambient policy. No production API, format, rule,
ignore entry, concurrency setting or admitted function changes.

## Measurements

Three alternating samples of the complete read-family regression used the
baseline test executable from `cb6464c` and the rebuilt test executable. Both
used exactly the same CLI and driver binaries, pinned nightly-2026-09-30,
locked dependencies, debug profile, Linux aarch64, eight-CPU quota and 16 GiB
memory. Each test creates its own fresh fixture and exercises initial capture,
strict evidence checks and tampering, warm capture, edited-source capture and
all 190 pair decisions. Runs were sequential with no concurrent build/test
work. Tool build/setup time was excluded; each measured test includes its
fixture setup and internal Cargo invocations. `RSTYLE_PERF` was not enabled.

| Measurement | Before | After |
| --- | --- | --- |
| Wall seconds, three samples | 32.23, 32.08, 32.12 | 27.95, 27.57, 27.87 |
| Median wall seconds | 32.12 | 27.87 |
| Median peak RSS KiB | 113,824 | 113,776 |

The local runtime improvement is about 13%; memory is effectively unchanged.
This is a test-harness improvement, not a production analysis speedup. Raw logs
and the measurement script remain in `/tmp/rstyle-borrow-fixture-{before,after}-{0,1,2}.log`,
`/tmp/rstyle-borrow-fixture-measure.jsonl` and
`/tmp/rstyle-borrow-fixture-measure.py`. An initial direct harness invocation
failed before analysis because `CARGO_BIN_EXE_cargo-rstyle` was unset; the
measurement script explicitly supplies it, as Cargo/nextest normally do.
All six reported samples are successful complete test runs.

## Validation

The six focused read-family sample runs passed. Final sequential validation:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- `CARGO_TARGET_DIR=target timeout 600 ./scripts/test.sh --nextest`: 642 passed,
  zero skipped, 106.284 seconds, no slow warnings; workspace doctests passed.
- `./scripts/style.sh --format json`: 1,970 functions, zero violations.
- `./scripts/style.sh patterns --fail-on-candidates --show-ignored`: zero
  actionable candidates, 15 existing reviewed ignored candidates.

Final logs are `/tmp/rstyle-borrow-fixture-{fmt,clippy,nextest,patterns}.log`,
`/tmp/rstyle-borrow-fixture-style.json` and
`/tmp/rstyle-borrow-fixture-style.err`. The full-suite time is an observation;
only the focused alternating samples support the reported local speedup.
