# Bounded project coverage study

This study selects the `rstyle-ccgraph` library in check mode with no features,
using the existing `hir-data-v3` extraction and unchanged consumer work budgets.
The four production functions and twelve source expectations in
`project-study-cases.json` were independently labeled before reading the report.
They are a small measurement sample, not project-wide precision/recall labels.
Density quality has no independent labels and remains unmeasured.

Capture 6 on Linux aarch64, nightly-2026-09-30, after the match-guard and associated
call routing repairs, completed in 20.416 seconds with exit 2. Capture and handoff
were complete, with no capture/handoff failures. The known roster contains 414
owners. Source and density executions both report `budget_exhausted` and
incomplete coverage. Exit 2 therefore remains required. The earlier five failed
captures do not provide coverage or comparable performance baselines.

| Consumer | Known evaluated | Findings | Evaluated negatives | Unavailable |
| --- | ---: | ---: | ---: | ---: |
| Length | 238 | 117 | 121 | 176 |
| Parameters | 238 | 123 | 115 | 176 |
| Spacing | 208 | 0 | 208 | 206 |
| Density | 0 | 0 | 0 | 414 |

Source assesses 396 owners, retaining available individual measurements even when
another requested rule exhausts its budget. All 238 original-source owners have
length and parameter measurements, including methods. The 176 unavailable owners
are expansion-generated functions; they cannot become original-source clean
results. Spacing has 21 unknown and 185 unsupported assessments: 176 generated,
18 parser-budget, nine expansion/lowering, and three statement-roster gaps. Density assesses 394 owners but measures none: every event inventory
is incomplete; statement inventories are known for 128 owners, unknown for 20,
and unsupported for 266. No effect purity or density absence follows.

Eleven sample measurements agree with their independent labels. The twelfth,
`frac::split_decimal` spacing, is unknown because original statement inspection
exhausts its budget. Its independently expected seven direct statements remain
unchanged in the label file. Its length and parameter measurements still agree.
There is no observed disagreement, but the unavailable case has not passed.
The function has 146 punctuation characters against the unchanged lexical
parser limit of 128; its seven direct statements individually have
8, 26, 22, 17, 30, 15 and 14. A future statement-local parser would need complete
ordered source partitioning, trivia-only gaps, independent per-statement bounds
and an explicit aggregate bound; simply increasing 128 would move the cliff.

Reproduce from the repository root (use fresh output paths):

```sh
cargo build --locked -p rstyle-driver -p cargo-rstyle
RSTYLE_DRIVER="$PWD/target/debug/rstyle-driver" \
python3 scripts/watchdog.py --timeout 180 --grace 1 \
  --output /tmp/clones-project-study-capture -- \
  target/debug/cargo-rstyle rstyle --manifest-path "$PWD/Cargo.toml" \
  --package rstyle-ccgraph --format json clones \
  --extraction-profile hir-data-v3 --locked --report-detail compact \
  --source-rules --analysis-config eval/clones-capability/project-source-policy.toml \
  --effect-density --effect-config eval/clones-capability/project-density-policy.toml
python3 eval/clones-capability/project_coverage.py \
  --report /tmp/clones-project-study-capture/stdout.log \
  --labels eval/clones-capability/project-study-cases.json \
  --workspace-root "$PWD" --output /tmp/clones-project-study.json
python3 -m unittest discover -s eval/clones-capability -p test_project_coverage.py
```

The reducer joins exact unit/function identities against the full captured roster,
checks consumer aggregates and policy/build context, and retains per-owner
availability, execution, boundaries and reasons. Missing rows cannot become
clean. Inputs are explicit and bounded; output creation refuses overwrite.
It rejects duplicate JSON object keys. It is a study reducer, not a producer of
trusted evidence or a substitute for checking capture exit/completion status.

Original local evidence is retained under
`/tmp/clones-project-ccgraph-capture-6` and
`/tmp/clones-project-ccgraph-study-6-final.json`. These paths are session artifacts;
the commands, policies, independent labels and reducer are the repeatable assets.
Next work should investigate the small source-statement budget case while
preserving existing limits and unknown semantics. Broader durable review locators
for external methods are a separate need; ordinary source methods already measure.
Native macOS, Go comparison and per-capability legacy retirement remain pending.
