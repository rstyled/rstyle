# Bounded clones capability acceptance

This corpus freezes source-authored expectations for the capability migration.
It is independent of new producer output and is not a broad Rust benchmark or a
claim of Go compatibility. The isolated Cargo workspace has no dependencies.

`proposal-review-cases.json` separately freezes the next helper-proposal and
review-lifecycle requirements: eight source-authored proposal cases and nine
identity/dismissal transitions. Their embedded source examples and synthetic
transitions are specifications, not captured facts or passing implementation
results. They include ordered subtraction, opaque calls, usefulness boundaries,
nontransitive pair relationships, movement, body edits and exact-key collision
isolation. Their delivery gate requires real consumer execution and lifecycle tests.

`expectations.json` labels twelve source/signature cases, one strict pair and one
negative abstraction case. Length counts the entire function, including its
signature and closing brace, excluding blank/comment-only lines. The test policy
is five code lines and four parameters, with `self` excluded. Exactly-at-limit
cases are clean; a value strictly greater than the limit produces a finding.

Spacing is scored only when a case explicitly declares `spacing` and
`top_statements`; historical length/arity-only cases retain their denominator.
Its policy is `min_statements = 5`, with a reported limit of four. Five direct
statements without a blank line produce a finding; a blank line makes the same
count clean. Nested statements do not increase the direct count. The six spacing
examples cover four/five statements, blank lines, a nested closure, a multiline
initializer and named suppression. Independent outcomes are authored separately
from counts because counts alone do not establish blank-line policy.

An explicit zero minimum disables spacing. Comparison requires a complete
`not_applicable`, null-count, zero-limit clean assessment, a complete clean legacy
assessment and matching suppression permission. This is
`policy-disabled-agreement`, never measured agreement. Exclusion takes precedence.
Missing policy, count, incomplete or malformed evidence is uncovered; contradictory
known measurements or policy outcomes are regressions.

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

Source cases may declare `suppressed_rules`, a list of independently expected
source-rule permissions (omitted means none). Raw source-rule outcomes and counts
remain the measurement labels. Comparison separately checks the captured
permission and compares the visible outcome with legacy diagnostics; suppression
cannot excuse a wrong measurement. Missing suppression evidence is uncovered.
Historical reports without a suppression field are treated as unsuppressed.
Invalid-suppression diagnostics are validated by CLI acceptance, outside this
measurement comparator.

Source cases may also declare `excluded: true` (omitted means false). This checks
workspace path policy, separately from measured agreement. The legacy scope
roster must retain that owner with `excluded: true` and complete scope; its numeric
diagnostics must be absent. The new report must retain a completed owner with an
explicit `source_exclusion` decision and complete `not_applicable` skips, null
measurements and the configured limits. Only then is the row
`policy-excluded-agreement`. The raw source labels are retained but not scored as
measurements for that row. Missing or malformed evidence remains uncovered;
known exclusion decisions that contradict the independent label are regressions.
Aggregate excluded counts grant no permission. Historical reports without an
exclusion field are treated as included.

The fixed corpus deliberately exceeds both limits in `src/excluded.rs` and excludes
that exact path. The runner validates the fixed exclusion configuration, retains
all captured legacy owners, and reconciles the summary against included owners.
Its per-owner legacy exclusion declaration uses the known fixture path, without
implementing a second glob matcher. General project exclusions still require an
independently checked operator declaration in the sidecar.

Compare retained explicit reports with:

```sh
python3 eval/clones-capability/compare.py \
  --legacy /tmp/clones-capability-legacy.json \
  --new /tmp/clones-capability-new.json \
  --expectations eval/clones-capability/expectations.json \
  --scope /tmp/clones-capability-scope.json \
  --output /tmp/clones-capability-comparison.json
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
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
      "policy": {"max_lines": 5, "max_params": 4, "count_self": false,
                 "exclude": ["target/**", "src/excluded.rs"]}
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
`actual-regression`, `policy-excluded-agreement`, `policy-disabled-agreement`,
and `unreviewed-disagreement`. `intentional-improvement`
requires a source-case `reviewed_improvements` map from rule ID to review reason;
the initial labels have no such entries. A new result must match the independent
outcome, actual measure and configured limit before it can count as measured agreement or
improvement. Exit status is zero only when every row has measured agreement,
explicit excluded/disabled policy agreement, or a reviewed improvement. This adapter assesses source rules only; pair and abstraction quality
need their own consumers and are deliberately excluded from its denominators.

`enum-dispatch-cases.json` freezes nine local dispatch requirements. It includes
an opaque callee body whose signature remains usable, exact existing-trait
suppression, unrelated traits, arity/receiver/free-callee counterexamples and
unsupported guarded/opaque arms. The real-Cargo `migration_dispatch` tests assess
these source labels, full/compact/default reports, budget and mixed-coverage
semantics, and explicit saved-run replay. See the
[bounded consumer contract](../../docs/clones-enum-dispatch.md).

`effects-cases.json` freezes direct scalar assignment budgets, caller-visible
provenance, opaque boundaries and saved-policy reevaluation. Initial cases
exercise the mutation consumer; next/later cases retain explicit future scope.
A complete direct-write inventory does not establish purity or density, and the
static-site vocabulary differs intentionally from legacy mutable-borrow events.

`planning-cases.json` defines required-edit coordination from explicit helper
wrapper intents. Initial real-Cargo tests cover shared bodies, disjoint bodies,
helper-policy rejection, unavailable evidence, shared lib/test source regions and
source-free replay. Pure cases cover half-open intersections, conflicting snapshots
and bounded work. Next/later local-guard and caller-parameter intents remain pending;
no whole-plan independence or safe rewrite is inferred. See
[edit planning](../../docs/clones-edit-planning.md).

`helper-effect-cases.json` defines caller-visible scalar mutation through original
same-unit leaves, distinct call occurrences, formal mappings, unknown boundaries,
bounded summary reuse and source-free replay. It extends the direct-site cases
without changing their metrics or claiming purity. See
[mutation effects](../../docs/clones-mutation-effects.md).

`capability-cases.json` specifies witnessed local two-method capability proposals
from original ordered wrapper calls. It includes opaque method bodies, exact
existing-trait suppression, negative namesakes and repeated members, unavailable
namespaces/coercions, explicit budgets and source-free replay. See
[capability proposals](../../docs/clones-capability-proposals.md).
