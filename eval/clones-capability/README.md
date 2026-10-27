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
assessment and explicit denominator are required for a clean result. The report
adapter is added after the shared report contract is frozen; no temporary schema
is established by these label files.
