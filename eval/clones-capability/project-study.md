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

## Saved compiler proposal review lookup

The schema-8 review checkpoint replays that same `run-I9FEzJ` capture with an
explicit absent review path. It completes in 2.12 seconds (maximum resident set
201,248 KiB) on the pinned Linux aarch64 toolchain. This single replay observation
is not a performance comparison. The sole candidate `c5f442ffa59a` receives
`visible_new`; no review file is created and no finding is dismissed. Its earlier
source review above remains unsuitable. Raw core proposal and style reports,
capture/handoff outcomes and all four accepted strict pairs equal the previous
report exactly. Generated owners and unresolved callees still make coverage
incomplete and require exit 2; the review lookup does not erase those gaps.

```sh
python3 scripts/watchdog.py --timeout 90 \
  --output /tmp/clones-compiler-reviews-project-1 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory --compiler-patterns \
  --replay-run target/rstyle/clones/run-I9FEzJ \
  --review-file /tmp/clones-compiler-reviews-project-1/choices.json \
  --report-detail compact
```

Session evidence is `/tmp/clones-compiler-reviews-project-1`. Independent fixture
acceptance covers actual saved choices across all five proposal families,
changed observations, check/test separation, source-free replay and conservative
retention when bounded discovery no longer produces a proposal. This delivers
access to the existing saved-review workflow; it establishes neither better
proposal quality nor baseline/retirement readiness.

## Compiler diagnostic review access

The next read-only replay uses the explicit compiler study policy
[`project-compiler-review-policy.toml`](project-compiler-review-policy.toml), which
sets the distinct mutation-target budget to one. Production and legacy dog-food
policies are unchanged. The independent case in
[`project-compiler-review-cases.json`](project-compiler-review-cases.json) was
recorded before replay: the original `frac::gcd` tuple assignment writes the two
owned roots `a` and `b`, so its `mutated_targets` diagnostic must report actual 2,
limit 1 and a visible-new review decision. The replay matches that expectation,
including one event for each root. This is numeric review access, not an endorsed
refactoring or a claim that gcd should be rewritten.

The same retained library capture has 39 diagnostics under that explicit policy;
all 39 obtain available positive review identities and remain visible-new with
an empty review store. The 238 original-owner population, 176 unavailable
generated owners and twenty unresolved callees remain unchanged. Raw core style
and proposal reports exactly equal the same-policy run without reviews, and all
four accepted strict pairs equal both that run and the earlier project capture.
No file is created or finding dismissed. Incomplete coverage still requires exit
2; available identities for known findings do not establish a clean full scope.

```sh
python3 scripts/watchdog.py --timeout 90 \
  --output /tmp/clones-compiler-diagnostics-project-review-1 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory --compiler-patterns \
  --compiler-advisory-config eval/clones-capability/project-compiler-review-policy.toml \
  --replay-run target/rstyle/clones/run-I9FEzJ \
  --review-file /tmp/clones-compiler-diagnostics-project-review-1/choices.json \
  --report-detail compact
```

For the same-policy reference, omit `--review-file` and use a separate output
directory. The pinned Linux aarch64 replay takes 2.12 seconds and 202,208 KiB peak
RSS without review lookup, and 2.11 seconds and 202,168 KiB with lookup. These are
single-run resource observations, not a speedup or a bound on lookup overhead.
Evidence is retained in `/tmp/clones-compiler-diagnostics-project-{raw,review}-1`.
Independent real-Cargo cases separately exercise recorded choices for all six
style-rule families, movement, changed measurements, source-free replay,
check/test separation, failed writes and incomplete generated input. Baselines,
completed-negative pruning, broader quality and retirement remain open.

## Compiler diagnostic baseline boundary

The baseline checkpoint reuses the same retained `run-I9FEzJ` library capture
and explicit `project-compiler-review-policy.toml`. Its independently recorded
expectation is in
[`project-compiler-baseline-cases.json`](project-compiler-baseline-cases.json):
176 unavailable generated owners and twenty unresolved callees prevent a clean
whole-scope baseline even when the known diagnostics are usable.

A reference replay retains the same 39 diagnostics, raw core style/proposal
reports and all four accepted strict pairs. The subsequent explicit export
request exits 2 with `Incomplete analysis cannot publish a compiler diagnostic
baseline` and creates no baseline file. No real-project finding is dismissed or
review file written. This establishes the baseline workflow's incomplete-scope
boundary; it does not establish a clean project baseline or legacy retirement.

```sh
python3 scripts/watchdog.py --timeout 90 \
  --output /tmp/clones-compiler-baseline-project-export-1 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory --compiler-patterns \
  --compiler-advisory-config eval/clones-capability/project-compiler-review-policy.toml \
  --replay-run target/rstyle/clones/run-I9FEzJ \
  --write-compiler-baseline /tmp/clones-compiler-baseline-project-export-1/baseline.json \
  --report-detail compact
```

For the reference, omit `--write-compiler-baseline` and use a separate output
directory. On pinned Linux aarch64 the reference takes 2.07 seconds and 201,568
KiB peak RSS; the refused export takes 2.02 seconds and 201,684 KiB. These are
single observations with different output work, not a speedup or overhead bound.
Evidence is `/tmp/clones-compiler-baseline-project-{raw,export}-1`. Complete
six-family baseline publication/comparison and source-free replay are separately
verified by the independent real-Cargo fixture cases. Broader project completeness
and quality remain open.

## Compiler changed-file presentation

The independently pre-recorded
[`project-compiler-selection-cases.json`](project-compiler-selection-cases.json)
selects `crates/rstyle-ccgraph/src/frac.rs` from the same retained library run and
explicit compiler policy. Two numeric mutated-target findings are selected.
The 39 raw diagnostics and their complete evaluations/fix plans, the one global
proposal, unavailable/unresolved lists and all strict matching results are
identical to the reference replay. Four of the fifteen strict pair evaluations
are accepted in both reports. Global advisory findings remain forty.

The selected path has 23 matched functions, eight with unavailable observations,
so its owner coverage is incomplete. Both reference and selected runs exit 2;
file selection does not hide that missing data or the whole-run 176 unavailable
generated owners and twenty unresolved callees. No review file is written and
no real-project finding is dismissed.

Repeat the preceding reference command without the baseline export, then append:

```sh
--changed-files /tmp/clones-compiler-selection-project-manifest.json --fail-on-findings
```

The manifest is the `selection` object in the recorded cases file. Evidence is
`/tmp/clones-compiler-selection-project-{raw,selected}-1`. On pinned Linux aarch64,
reference/selected take 2.13/2.04 seconds and 201,632/201,568 KiB peak RSS. These
single observations do not establish a speedup or overhead bound. This checks
presentation over the existing retained project population, not broader project
completeness or legacy retirement.

## Compiler proposal baseline boundary

The independently pre-recorded
[`project-compiler-pattern-baseline-cases.json`](project-compiler-pattern-baseline-cases.json)
uses the same library capture and explicit compiler policy. The reference replay
exactly preserves the prior complete advisory report: 39 numeric diagnostics,
one raw proposal, 176 unavailable generated owners and twenty unresolved callees.
All strict results remain identical, including four accepted pairs.

An explicit `--write-compiler-pattern-baseline` request correctly refuses export
with exit 2 and `Incomplete analysis cannot publish a compiler proposal baseline`.
No baseline or review file is created, and no real-project finding is dismissed.
This checks the incomplete-scope boundary; it does not establish a complete
project baseline or improve the previously reviewed proposal's quality.

Repeat the previous compact reference replay, then append:

```sh
--write-compiler-pattern-baseline /tmp/clones-compiler-pattern-baseline-project-export-1/baseline.json
```

Pinned Linux aarch64 evidence is
`/tmp/clones-compiler-pattern-baseline-project-{raw,export}-1`. Reference/refused
export take 2.24/1.99 seconds and 201,024/202,008 KiB peak RSS. These single runs
perform different output work and establish neither a speedup nor an overhead
bound. Complete five-family baseline comparison/publication is separately tested
with independent real-Cargo fixture cases and source/tool-free replay.

## Completed compiler numeric scopes

The independently recorded
[`project-compiler-negative-cases.json`](project-compiler-negative-cases.json)
requests read-only status against a missing review file over the same explicit
library capture and policy. It requests completed owner scopes while creating no
review file or retained choices. The 39 raw numeric findings, one raw proposal,
176 unavailable generated owners, twenty unresolved callees and all strict
results remain identical to a fresh reference replay, including four accepted
pairs. Both runs exit 2. Ordinary review presentation adds visibility indices;
raw core evaluations remain unchanged.

Use the previous reference command without baseline export, then append:

```sh
--review-file /tmp/clones-compiler-negative-project-status-1/reviews.json --review-status
```

Pinned Linux aarch64 reference/status resource observations are both 2.09 seconds
and 201,792/201,984 KiB peak RSS. These single runs do not establish a speedup or
an overhead bound. Evidence is `/tmp/clones-compiler-negative-project-{raw,status}-1`.
The seven independent real-Cargo fixture cases establish six-family negatives,
explicit source-free pruning, policy/context/selection and suppression boundaries,
direct-versus-density completeness and independence from strict pair budgets.
This project check establishes preservation with requested scope retention, not
new project findings, measured negative coverage, broader quality or retirement.

## Cross-package compiler effect propagation

The independently pre-recorded
[`project-compiler-cross-unit-cases.json`](project-compiler-cross-unit-cases.json)
labels sixteen free-function dependency targets and four associated methods over
the non-test `rstyle-ccgraph` and `rstyle-pdg` libraries. The intended observation
is expanded resolved-call coverage, with generated owners and associated methods
still incomplete. It is not a speed or strict-matching study.

```sh
env PATH=/home/dev/.cargo/bin:$PATH CARGO_TARGET_DIR=target \
python3 scripts/watchdog.py --timeout 180 \
  --output /tmp/clones-cross-unit-propagation-project-capture-1 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle \
  -p rstyle-ccgraph -p rstyle-pdg --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-advisory-config eval/clones-capability/project-compiler-review-policy.toml \
  --max-pairs 0 --locked --report-detail compact
```

On pinned Linux aarch64, this capture terminated with exit 2 after 44.16 seconds,
with 4,833,952 KiB peak RSS reported by `/usr/bin/time`. The driver stopped while
publishing `rstyle-pdg` source evidence: `Source census exceeds file/byte budget`.
The saved run is `target/rstyle/clones/run-S1tXzo`; the advisory unit roster is
empty. No usable project propagation measurement resulted. The existing census
limits (256 files and 8 MiB), strict admission and pair budget are unchanged.
This project coverage target remains unverified. Independent small real-Cargo
acceptance tests validate the bounded connection; they do not establish the
larger project's coverage gain or a performance improvement.

## Deep-body transport and production core call coverage

The smaller non-test `cargo-rstyle` binary plus `rstyle-core` library selection
fits the unchanged source census budget. Before capture,
[`project-compiler-core-cases.json`](project-compiler-core-cases.json) labels
seven ordinary core callees from existing source call sites; uncaptured
`rstyle-pdg` callees must remain unresolved. This is a call-coverage expectation,
not a diagnostic-quality or purity label.

```sh
env PATH=/home/dev/.cargo/bin:$PATH CARGO_TARGET_DIR=target \
python3 scripts/watchdog.py --timeout 180 \
  --output /tmp/clones-cross-unit-core-project-capture-2 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle \
  -p cargo-rstyle -p rstyle-core --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory \
  --compiler-advisory-config eval/clones-capability/project-compiler-review-policy.toml \
  --max-pairs 0 --locked --report-detail compact
```

The version-5 baseline `run-HeBbkZ` completes compilation but loses the entire
CLI unit during bundle import: `recursion limit exceeded at line 1 column
210297864`. Its CLI observation roster is empty with an unknown denominator;
this is not a zero-function or clean result. The compiler-generated HIR body
exceeds JSON container nesting. Compilation itself and the census budget pass.

The version-6 run `run-ynuMSl` completes capture and assessment without handoff
failures. The CLI now retains 1,190 observed and 1,428 unavailable functions out
of 2,618; its 28 resolved core dependency targets include all seven independently
pre-labeled targets. Eighty-seven other workspace targets remain unresolved,
including uncaptured PDG/ccgraph functions and associated core methods. There
are 140 raw CLI diagnostics; their individual truth/quality is not independently
reviewed here. The core unit's 378 observed, 722 unavailable, 48 diagnostics and
entire evaluation are identical before and after. Source location/outcome/reason
rows for its 1,100 assessed functions are also unchanged.

Both runs exit 2 because advisory inputs remain incomplete. The final strict
population contains 3,718 excluded functions, zero admitted functions and zero
accepted pairs; its complete empty pair roster is not evidence of broad strict
Rust coverage. No matching permissions or budgets changed. These measurements
establish recovered production advisory coverage and dependency resolution.

Resource observations are 48.07/58.03 seconds and 1,785,696/1,788,984 KiB peak RSS
before/after. The latter performs the previously discarded unit's assessment;
these are not equivalent successful workloads and establish no speedup.
Evidence is `/tmp/clones-cross-unit-core-project-capture-{1,2}`. The machine
observation is [`project-compiler-core-results.json`](project-compiler-core-results.json).
The larger two-library PDG census limitation above remains unresolved.

Strict saved-run replay also completes without capture/handoff failures and
preserves the entire advisory, matching and accepted-pair reports exactly. It
retains exit 2. Replay evidence is `/tmp/clones-flat-hir-project-replay-2`:
16.67 seconds and 1,439,088 KiB peak RSS. This is a different operation from fresh
capture and is not a speed comparison. Replay uses the original saved build
options; the first attempted invocation repeated `--locked` and was correctly
rejected before loading evidence. The successful command is the preceding
capture command with package/build options removed and
`--replay-run target/rstyle/clones/run-ynuMSl` appended (no `--locked`). The small
compiler regression separately deletes its source before replay.

## Cross-unit compiler planning on the C25 saved project scope

The source-inspected labels in
[`project-compiler-planning-cases.json`](project-compiler-planning-cases.json)
precede planner execution. With an explicit two-parameter study limit,
`rstyle_core::{evaluate_retained,evaluate_with_effects,all_effects}` require
signature steps and their exact existing cargo-rstyle caller owners. The first
two share `diagnostics::evaluate_core`; the third is called by
`propagation::evaluate`. This tests advisory planning, not strict clones or
safe rewrites. The policy retains C25's one-mutated-target study limit; production
defaults are unchanged.

Both observations use the same current binary and explicit C25 saved run
`run-ynuMSl`; they do not recapture or silently select a latest successful run.
Run once without `--compiler-plans` (baseline) and once with it:

```sh
env PATH=/home/dev/.cargo/bin:$PATH CARGO_TARGET_DIR=target \
python3 scripts/watchdog.py --timeout 180 \
  --output /tmp/clones-cross-unit-planning-project-after-1 -- \
  /usr/bin/time -v target/debug/cargo-rstyle rstyle --format json clones \
  --extraction-profile hir-data-v3 --compiler-advisory --compiler-plans \
  --compiler-advisory-config eval/clones-capability/project-compiler-planning-policy.toml \
  --max-pairs 0 --report-detail compact \
  --replay-run target/rstyle/clones/run-ynuMSl
```

Baseline per-unit groups are 147 CLI and 51 core. The optional shared planner
returns 195 groups over the same 1,568 known functions; three groups contain
steps from both units. All three labeled signature/caller relationships are
present, and the first two callees share `evaluate_core`'s edit footprint. Their
exact keys and group are in
[`project-compiler-planning-results.json`](project-compiler-planning-results.json).
The complete advisory report with only the new optional planning field removed
is exactly equal to baseline, as are strict matching and accepted-pair reports.
No findings or strict permissions changed.

Both executions exit 2. The CLI/core still have 1,428/722 unavailable owners;
87 CLI callees remain unresolved. Shared `input_complete` is false. All 3,718
strict functions remain excluded, with zero accepted pairs. Only the three
pre-labeled planning relationships are independently checked; the remaining raw
plans and 509 diagnostics under this study policy are not quality-reviewed.
External callers, physical edits across compilation contexts, type equivalence
and safe rewrites remain outside this heuristic planning claim.

One serial replay pair measures 16.89/16.81 seconds and
1,440,112/1,440,148 KiB peak RSS without/with plans. The second run has a warm
filesystem page cache (zero filesystem input reported by time versus 560,800
baseline). These samples establish no speedup. Evidence is
`/tmp/clones-cross-unit-planning-project-{before,after}-1`. Source/tool-free
replay, policy changes, suppressions, missing/ambiguous contexts and raw-plan
independence from reviews/changed-file presentation have separate real-Cargo
fixture acceptance; the production study leaves original sources in place.

## Independent compiler finding quality sample (C27)

[`project-compiler-quality-cases.json`](project-compiler-quality-cases.json)
labels eleven unchanged original functions before looking up their diagnostics:
parameter counts and distinct explicitly mutated source roots. The last three
functions extend the initial eight-function sample to include newly recovered
CLI mutation findings. These labels remain independent of the producer output.
`save_logs` has filesystem effects but no mutated source root; that label does
not assert purity. Owned local containers count as mutation targets.

The offline [`compiler_quality.py`](compiler_quality.py) reducer reads completed
JSON only. It reuses the existing legacy diagnostic assessment and original
owner/context associations, including the C26 indexed plans. It checks the
complete parameter/mutation finding-owner roster before interpreting an absent
finding. Missing or ambiguous observations remain unavailable. Clean rows
compare outcomes only: no numeric zero or other clean metric is synthesized.
The sample explicitly requires reviewed, unsuppressed original owners under the
reported root-granularity policy and target exclusion.

At C26's one-target limit, twenty of twenty-two comparisons agree. Two mutation
counts/provenances disagree: `dependencies::select` has source roots
`requested, selected`, and `dependencies::resolve` has `exact, named`. The
collector reports an additional `iter` in both. Both remain findings at limit
one. These disagreements are preserved in
[`project-compiler-quality-results.json`](project-compiler-quality-results.json).
Reproduce against the explicitly retained C26 report:

```sh
python3 eval/clones-capability/compiler_quality.py \
  --report /tmp/clones-cross-unit-planning-project-after-1/stdout.log \
  --labels eval/clones-capability/project-compiler-quality-cases.json \
  --output /tmp/compiler-quality-limit-one.json
```

The reducer exits 1 for these disagreements, 2 for unavailable/invalid input,
and 0 for agreement. Output creation never overwrites an existing file.

The existing `rstyle-driver::mutations` collector uses rustc's `ExprUseVisitor`
and names every local/upvar root through `tcx.hir_name`; this includes the
compiler-generated iterator of a lowered `for` loop. It is reused by legacy
style and compiler advisory. This is a source-versus-compiler metric discrepancy;
it does not establish a migration regression. No collector behavior or metric
version changed in C27, and filtering by the spelling `iter` would be unsound.

The explicit [`project-compiler-quality-policy.toml`](project-compiler-quality-policy.toml)
sets the mutation limit to two. Replay C26's command above with this policy and
output directory `/tmp/clones-compiler-quality-boundary-1`. Both source functions
are now below the finding threshold while the collector still reports three
roots, creating two outcome disagreements. The source labels are unchanged;
derive the second label input by changing only `limits.mutated_targets` to 2
and `policy` to the new policy filename. Compare this input with that replay's
`stdout.log`. All twenty other comparisons agree. The full observation, including
that derivation, is in
[`project-compiler-quality-boundary-results.json`](project-compiler-quality-boundary-results.json).

Boundary replay takes 16.60 seconds and 1,439,752 KiB peak RSS; this is a single
observation, not a speed comparison. Replay exits 2 because the captured scope
remains incomplete. Both quality samples are fully evaluated, but neither makes
the project complete or establishes project-wide precision/recall, density
quality, effect absence, rewrite safety or retirement readiness. Strict matching
is unchanged. The next collector investigation should use compiler identities
and source origins within the existing collector, preserving metric-version
compatibility. No hashes, source scans, proofs or runtime caches were added by
this offline review.
