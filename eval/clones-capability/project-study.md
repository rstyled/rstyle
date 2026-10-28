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

A subsequent external-method locator checkpoint replays this same saved run with
`--source-rules --analysis-config eval/clones-capability/project-source-policy.toml
--review-file /tmp/clones-project-method-reviews.json`. Strict replay resolves
identities for 121 of 123 parameter findings and 116 of 117 length findings.
The three unavailable identities are one cross-module qualified trait impl and
two lifetime-generic owners. All 240 findings remain visible; ordinary lookup
creates no review store. Source coverage remains incomplete and exit 2 remains
required. This 4.390-second replay is a separate workload from the original
capture and does not establish a speedup. Session evidence and the full command
are `/tmp/clones-project-method-review/{stdout.log,stderr.log,run.json}`.

The lifetime-only locator checkpoint replays the same explicit saved run with
`--review-file /tmp/clones-project-lifetime-reviews.json` and otherwise the same
source policy. Identities resolve for 122 of 123 parameter findings and all 117
length findings. `LabelNames::partition` and `AdmittedPair::check_correspondence`
now have original method identities. The remaining unavailable parameter owner
is the cross-module qualified trait impl `features::MulFrac`. All 240 raw findings
remain visible, and no review store is created. Capture/handoff failures remain
empty; existing source/spacing incompleteness still requires exit 2. This is a
4.407-second strict replay observation, not a fresh capture or a speed comparison.
Full session command and results are retained in
`/tmp/clones-project-lifetime-review-final/{run.json,stdout.log,stderr.log}`.

The spacing gap remains `frac::split_decimal`: its 146 raw punctuation bytes
exceed the existing whole-function parser limit of 128. Reapplying that limit
independently per direct statement would increase aggregate syntax work, so the
checkpoint preserves the explicit budget-exhausted result. This gap requires a
separately justified bounded algorithm, not an increased limit.


## Reuse-first compiler advisory measurement

A fresh explicit `rstyle-ccgraph` library capture with `--compiler-advisory`
completed in 20.685 seconds on the same Linux aarch64 pinned toolchain. Capture
and handoff failures were empty. The existing Rust collectors supplied facts for
all 238 original-source functions out of the 414-owner roster; 176 generated
functions remained unavailable. The core evaluator assessed those 238 functions
using its defaults, producing zero diagnostics and retaining 27 mutation, one
panic and four unknown effect events in its summary. Twenty unresolved workspace
callee identities keep advisory coverage incomplete and the exit status at 2.
The advisory configuration is recorded in full; it differs from the historical
study's deliberately small source limits.

This is a capability gain from reusing the compiler collector and existing core
policy, not a speedup or strict density coverage comparison. The strict density
study above is unchanged. Production `frac::gcd` now has two compiler-observed
owned mutation events and zero escaping events. Its compiler statement count is
nine, whereas the source lexical contract counts five: metric versions must not
be merged. The deferred integer/tuple consumer extension would have unlocked only
this one owner's strict density assessment; the reuse adapter reaches the full
238-original-owner population without another integer-family scan.

Reproduce with the build command above, then:

```sh
RSTYLE_DRIVER="$PWD/target/debug/rstyle-driver" \
python3 scripts/watchdog.py --timeout 180 --grace 1 \
  --output /tmp/clones-project-compiler-advisory -- \
  target/debug/cargo-rstyle rstyle --manifest-path "$PWD/Cargo.toml" \
  --package rstyle-ccgraph --format json clones \
  --extraction-profile hir-data-v3 --locked --report-detail compact \
  --compiler-advisory
```

The four accepted strict pairs exactly match the previous saved project report.

Session evidence: `/tmp/clones-project-reuse-capture-1`, saved run
`target/rstyle/clones/run-1MUfZV`. No project-wide quality/retirement claim follows
from this measurement; abstraction proposal reuse is the next unconnected policy.

## Reusing the compiler-backed abstraction proposer

The next fresh library capture adds `--compiler-patterns` to that same explicit
scope and pinned Linux toolchain. The final source checkpoint completes in
19.228 seconds with empty capture/handoff failures. All 238 original functions
have compiler graphs; there are no missing graphs among those observations.
The other 176 of 414 owners remain unavailable generated functions, and the same
20 unresolved workspace callees keep input coverage incomplete and exit status 2.
The four accepted strict pairs exactly match the earlier compiler-advisory run.
This is not a speed comparison: optional collection and compilation costs differ,
and these observations do not isolate runtime improvement.

The adapter directly reuses the existing core proposer, producing two signature
groups and one helper candidate. Its complete core proposal report exactly equals
an independent `patterns --no-ignore` run on the same library. That legacy run
completed in 22.981 seconds; it is comparison evidence, not a relative speed
claim. The candidate, `c5f442ffa59a`, groups `Frac::checked_add`, `checked_mul`,
`checked_div` and `ge`, with score 373 and alignment coverage 807/1000. All four
owners are excluded from strict clone admission, demonstrating advisory scope
independence.

This candidate is **reviewed unsuitable**, not a newly endorsed refactoring.
The existing unchanged `rstyle-ignore.toml` records that merging distinct exact
rational operations would obscure overflow-critical arithmetic. Source inspection
confirms that addition, multiplication, zero-checked division and comparison have
different arithmetic and result behavior. The core report's counter-evidence is
empty; this known quality limitation is preserved rather than hidden. No new
review entry or suppression was added. The adapter intentionally does not load
the legacy ledger implicitly, so that candidate remains visible in its report.
The gain is access to existing proposal capability over captured compiler facts,
not an increase in useful findings or a legacy-retirement claim.

For reproduction, append `--compiler-patterns` to the prior compiler-advisory
capture command. Evidence is retained at
`/tmp/clones-project-compiler-patterns-final-1/{run.json,stdout.log,stderr.log}`,
saved run `target/rstyle/clones/run-I9FEzJ`; the legacy comparison is
`/tmp/clones-project-patterns-legacy-1`. The report now distinguishes workspace
`source_root` from `package_root`, deriving source paths from already captured
locations without consulting source files on replay. Independent real-Cargo
acceptance cases separately cover positive trait, capability, dispatch, generic
and helper opportunities, plus existing-trait/getter/forwarder negatives.
