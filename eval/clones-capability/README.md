# Bounded clones capability acceptance

This corpus freezes source-authored expectations for the capability migration.
It is independent of new producer output and is not a broad Rust benchmark or a
claim of Go compatibility. The isolated Cargo workspace has no dependencies.

`expectations.json` labels five source/signature cases, one strict pair and one
negative abstraction case. Length counts the entire function, including its
signature and closing brace, excluding blank/comment-only lines. The test policy
is five code lines and four parameters, with `self` excluded. Exactly-at-limit
cases are clean; a value strictly greater than the limit produces a finding.

The closure in `unsupported_body_parameters` deliberately excludes strict graph
analysis while leaving complete source and five explicit parameters available.
A strict accepted pair is not evidence of refactoring safety. In particular,
the identity forwarding pair must not become a helper proposal merely because
it is structurally similar.

`coverage-cases.json` supplies synthetic consumer inputs for missing observations,
known-empty signatures with unsupported bodies, and exhausted work. These are
consumer contract cases, not serialized trusted capture records. Do not forge
compiler records or corrupt a real capture to manufacture ordinary unavailability.

The guard and pair-difference labels are prepared acceptance requirements. Their
presence does not claim those consumers are implemented. Arithmetic examples
describe syntax/dataflow; suggestions are advisory and do not assert absence of
overflow or a safe rewrite over all input values.

Run from the repository root, after the opt-in consumer is integrated:

```sh
CARGO_TARGET_DIR=target ./scripts/style.sh clones \
  --manifest-path eval/clones-capability/corpus/Cargo.toml \
  --extraction-profile hir-data-v3 --locked --format json \
  --report-detail compact --source-rules \
  --analysis-config eval/clones-capability/analysis.toml \
  > /tmp/clones-capability-new.json
```

Run legacy style separately against the same manifest and equivalent rule policy.
Legacy defaults include unrelated rules; compare the named rules, not total
finding counts or abstraction counts against strict accepted pairs. Preserve
revision, toolchain, target/features/profile, declared source scope and exact
report paths. Do not select a previous successful capture implicitly. `--locked`
must be used once the fixture lockfile is present. No platform-specific result is
implied by the source labels.

Comparison must distinguish uncovered/not-evaluated cases, evaluated agreement,
reviewed intentional changes, and regressions against these labels. Missing
report entries are not automatically evaluated-no-finding. A complete input
assessment and explicit denominator are required for a clean result.

Compare retained explicit reports with:

```sh
python3 eval/clones-capability/compare.py \
  --legacy /tmp/clones-capability-legacy.json \
  --new /tmp/clones-capability-new.json \
  --expectations eval/clones-capability/expectations.json \
  --scope /tmp/clones-capability-scope.json \
  --output /tmp/clones-capability-comparison.json
python3 -m unittest discover -s eval/clones-capability -p test_compare.py
```

For repeatable live comparison, first build the pinned workspace binaries, then
run the serial capture harness from the repository root. The output directory
must be new:

```sh
cargo build --workspace --locked
python3 eval/clones-capability/run_compare.py \
  --output /tmp/clones-capability-run --timeout 180
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
```

The harness retains commands, logs, exit status, elapsed time, revision/worktree
state, compiler versions, input snapshots, and both reports. It uses separate
fresh target directories and derives the legacy function roster from the actual
captured fact file, checking its count against the report summary. These fresh
captures measure execution, not incremental analysis reuse. Prebuilt executable
paths and timestamps are recorded; run the build checkpoint first to establish
which source produced them. The fixed fixture is a library with no features and
uses host/dev defaults. Legacy CLI has no `--locked` switch, so its capture runs
offline and the harness verifies that the existing lockfile stays unchanged;
the new path uses `--locked` explicitly. These limitations remain in the sidecar.

The required scope sidecar records explicit run metadata because legacy diagnostic
JSON does not record a full build context. Its shape is:

```json
{
  "legacy": {
    "context": {
      "manifest": "eval/clones-capability/corpus/Cargo.toml",
      "package": "clones-capability-acceptance",
      "target": "lib",
      "features": [],
      "profile": "dev",
      "toolchain": "RECORD_ACTUAL_PINNED_TOOLCHAIN",
      "revision": "RECORD_ACTUAL_REVISION",
      "policy": {"max_lines": 5, "max_params": 4, "count_self": false}
    },
    "complete": true,
    "functions": [
      {"name": "source_rules::length_at_limit", "source_file": "ACTUAL_DIAGNOSTIC_PATH", "line": 4}
    ]
  },
  "new": {"context": "COPY_THE_SAME_CONTEXT_OBJECT_AFTER_VERIFYING_THE_NEW_RUN"}
}
```

Replace placeholders and record the entire independently checked legacy function
roster, not just the example entry. `new.context` must be an object matching the
legacy context. Run metadata is an operator declaration, not compiler evidence;
record it while executing the commands. The tool rejects missing/different
contexts and report manifest/toolchain contradictions. Manifest paths resolve
relative to the invocation directory; the expectations manifest resolves relative
to its own file. Exact source path/line matches attach nameless legacy diagnostics
to roster entries. Duplicate functions/diagnostics are unavailable for comparison,
so multiple compilation contexts are never silently merged. Compare one declared
context at a time.

Output rows distinguish `uncovered`, `context-mismatch`, `evaluated-agreement`,
`actual-regression`, and `unreviewed-disagreement`. `intentional-improvement`
requires a source-case `reviewed_improvements` map from rule ID to review reason;
the initial labels have no such entries. A new result must match the independent
outcome, actual measure and configured limit before it can count as agreement or
improvement. Exit status is zero only when every row agrees or has a reviewed
improvement. This adapter assesses source rules only; pair and abstraction quality
need their own consumers and are deliberately excluded from its denominators.
