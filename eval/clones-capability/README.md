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
exercise the mutation consumer. The bounded ordinary `if`/`else` extension counts
both authored arms once, with distinct lexical roots and unknown boundaries;
conditional helper-effect and formal-use proof coverage stays conservative.
The original lexical-block shadowing case also has explicit live coverage.
The bounded direct loop extension counts original primitive half-open u32 range,
boolean while and unlabelled loop sites once, including statements after plain
break/continue. Its staged source label has live coverage. Labels, arbitrary
iterators and broader loop semantics remain outside this inventory; next/later
cases retain explicit future scope.
Mutation policy acceptance separately checks captured named suppressions and
workspace path exclusions. Schema 2 retains raw evidence while exposing visible,
suppressed and excluded findings and separate raw/policy coverage. Exclusions
are explicit skips, and suppression alone never makes an opaque owner clean.
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
same-unit leaves and one checked wrapper (at most two call edges), distinct call
occurrences, formal mappings, unknown boundaries,
bounded summary reuse and source-free replay. It extends the direct-site cases
without changing their metrics or claiming purity. C11d deliberately changes the
previous nested-helper negative to a complete owned-scalar two-edge case and adds
a separate third-edge negative; it does not establish arbitrary transitive closure. See
[mutation effects](../../docs/clones-mutation-effects.md).

`capability-cases.json` specifies witnessed local two-method capability proposals
from original ordered wrapper calls. It includes opaque method bodies, exact
existing-trait suppression, negative namesakes and repeated members, unavailable
namespaces/coercions, explicit budgets and source-free replay. See
[capability proposals](../../docs/clones-capability-proposals.md).


`density-cases.json` independently specifies the bounded static policy numerator
and complete statement denominator: repeated direct roots, delegated call
occurrences, local path-kind propagation, classified safe external calls and
unknown-zero-weight rejection. Real Cargo tests cover thresholds, requested-axis
completeness, source policy, budget failures and full/compact source/tool-free
replay. Native metadata and checked arithmetic have separate negative tests.
This metric intentionally differs from legacy mutable-borrow counting and is
not a claim of general effects, runtime frequency, purity or legacy retirement.

Run the separate density comparison after building the pinned workspace binaries:

```sh
python3 eval/clones-capability/run_density_compare.py \
  --output /tmp/clones-density-comparison --timeout 60
```

`density-comparison-cases.json` independently labels the ten-function
`density-corpus` workspace. The full policy explicitly fixes all weights and
classified path lists. Separate legacy/new config files express the same effective
two-edge policy: legacy accepts `transitive_depth = 2`, while the new frontend fixes
that bound internally and rejects that input field. Both effective policies must
match the independent labels before capture.
The harness reuses serial capture, fresh target directories, recorded compiler
and revision metadata, lockfile checks and captured legacy owner locations. It
snapshots the comparator/runner code as well as Rust sources, labels and policy.
Legacy/new results, exit statuses, commands, elapsed times and comparison rows
remain in the new output directory. A prebuilt executable's path/stat is
provenance metadata; use the preceding build checkpoint to associate its source.

Rows compare original event-kind counts, statement/code-line denominators,
weighted milli arithmetic, exact policy, raw outcomes and visible source policy.
Legacy clean diagnostic agreement does not establish a legacy numeric value.
The delegated-call case explicitly reviews the change from legacy mutable-borrow
and delegated propagation observations to one event per checked original call
occurrence. Other disagreements require their own review; they cannot inherit
this exception. The classified-helper case separately reviews legacy deduplication
of inherited kind/label pairs across call sites versus original occurrence counts.

The unknown-call case is deliberately `expected-uncovered`: it must have no
numeric density or clean result. It stays outside `summary.measured` and forces
`fully_complete: false`, even when all acceptance cases match and the comparison
command exits zero. The harness retains a new-analysis exit two for this declared
incompleteness; the adapter still rejects failed capture/handoff, changed contexts,
missing/duplicated owners, and unexpectedly unavailable completed cases. Thus
successful acceptance does not claim complete project coverage or legacy
retirement. Excluded policy rows have their own denominator, and raw source
measurements are checked even when suppression/exclusion hides a finding.

The [bounded project study](project-study.md) separately measures actual
`rstyle-ccgraph` coverage with explicit policies and independent source samples.
Its incomplete source/density results do not establish legacy retirement.


The compiler-quality study also reviews completed compiler advisory diagnostics
without reanalysis. `project-compiler-propagated-density-depth{0,1,2}-cases.json`
freeze four production source/count expectations; the positive-depth cases list
manually reviewed callee inputs and exact inherited detail labels. Missing
inputs remain unavailable. The reducer compares the completed core results,
including immediate `via` and collapsed-label multiplicity, and preserves global
incompleteness. The [project study](project-study.md) records the initial display
label corrections, replay commands, numeric results and resource limits. Run
`python3 -m unittest discover -s eval/clones-capability -p 'test*.py'` for the
independent reducer controls. This does not establish runtime purity or retire
legacy density policy.


`compiler_proposal_quality.py` reviews completed core proposal reports using
independent original-source labels. It reuses `compiler_quality.sample_owner`
and matches exact family and separate site/definition owner sets, without
candidate fingerprints, reanalysis or a second proposer. The production
`project-compiler-proposal-quality-{cases,results}.json` sample records three
structural matches independently judged not useful. All seven raw proposals stay
in its inventory; unreviewed entries remain unreviewed and visibility is separate.
Counts describe labeled reviews, not whole-project precision. Missing source,
graphs or selected-owner rosters remain unavailable; absence over incomplete
input cannot establish a negative. The reducer exits 1 for observed not-useful or
needs-revision judgments and 2 for unavailable samples. See the project study for
capture scope, resource measurements and a reproduction command.


The later `project-compiler-proposal-alignment-results.json` uses the same frozen
source labels on an explicit saved-run replay. It adds unmatched-node warnings
to five proposals from existing completed coverage, retaining all numeric fields,
roles and three negative usefulness judgments. See C35 in the project study;
this is clearer evidence presentation, not improved proposal precision.


`project-compiler-proposal-effect-results.json` records the C36 completed-effect
handoff replay against that same captured core library and C33 source labels.
All seven candidate evaluations, quality judgments and strict results remain
identical; the only added report field is effect-evidence provenance. This sample
has no additional configured/inherited kind contrast. Four live-Cargo controls
separately show configured IO/lexical versus equal-IO, inherited depth0/1,
structural review persistence, unavailable generated callees, distinct test/lib
units and source-free replay. Recorded kinds are not purity, weighted density or
refactoring safety. Structural reviews do not approve recomputed effect notes.
The measured single replay is comparable to C35; no speedup is claimed.


`project-compiler-proposal-forwarding-{cases,results}.json` records C37 policy 2.
It preserves C33 source judgments and requires the pipeline sample's exact
built-in rejection reason and source sites. One negative proposal is rejected;
two remain emitted and the reducer still exits 1. Missing proposals over
incomplete input stay unavailable. Historical implicit-version-1 observations
remain unchanged. Policy version drift is an error, and explicit rejection is
not a user review or proof that extraction is safe.


`project-compiler-direct-density-review-{cases,results}.json` records C38's
real-project review lifecycle experiment. A declared temporary source variant
creates a genuine density4500 finding; the frozen original is negative at the
same limit3000/depth0 policy. Completed owner-local direct measurements now
support retained review resolution despite unrelated missing input. Deeper
propagation and whole-analysis completion keep their existing gates. The
variant is not a historical-production claim, and no persistent repository
review or ignore is added.


`project-shared-source-quality-{cases,results}.json` reviews C40's explicit saved
metric5 `run-mb8gH8` using the same `compiler_quality.py` reducer. Independent
source labels cover seven original functions in ten owner/compilation contexts:
four authored test spacing findings, six clean spacing controls and ten checks
each for parameter count and mutated-target count. All thirty checks agree;
clean rows compare only outcomes, with no synthesized numeric observations.
The sample remains separate from overall advisory/planning completeness.

Labels may now select `test: true` or `test: false` explicitly; omission retains
the old non-test selection. Spacing labels supply an explicit outcome because
a blank line can make a function clean at the same statement count. The reducer
reads existing completed core fix-group source locations, checks one-to-one
diagnostic ownership and preserves exact original source association. It does
not parse source, collect effects or run another evaluator. The initial attributed
source-scope mismatch remained unavailable until the labels included the original
`#[test]` attributes; the comparison was not relaxed.

Reproduce against an explicit full/compact report from that saved run:

```sh
python3 eval/clones-capability/compiler_quality.py \
  --report /path/to/run-mb8gH8-report.json \
  --labels eval/clones-capability/project-shared-source-quality-cases.json \
  --output /tmp/shared-source-quality-review.json
```

The output path must be new. This is a routing-only original-source sample,
not a full-package precision/recall or runtime purity assessment. The four
spacing groups contain no signature steps or bundles under the default
parameter limit. Global advisory/planning scope stays incomplete because the
generated test harness remains unavailable. Strict clone matching and existing
metric versions are unaffected.

### Shared complexity from main (C42)

`project-complexity-cases.json` freezes four original `rstyle-core` library
functions against shared complexity policy 1/default 12 limits. An independent
source review specified one positive, two boundary controls and a straight-line
control before capture. Source metadata was then corrected to include original
doc comments and the lifetime-qualified method display name; score and outcome
expectations did not change. No Go comparison or retirement claim is made.

The explicit metric 6 run `run-4zkGtw` measured 407 original functions. All eight
labeled CC/COG rule outcomes agree. Clean comparisons report outcomes only;
absence of a diagnostic never invents a numeric score. Whole-package advisory
coverage is incomplete: 806 generated owners and eight unresolved callees remain
unavailable. The command exits 2 for that incomplete scope. The original capture
observation took 18.21s with 588,176KiB peak RSS; this is not a speedup measurement.
The source inventory, captured observation roster, completed core fix plans and
numeric diagnostics supply the offline comparison; there is no extra parser,
scorer, hash or live analysis.

```sh
CARGO_TARGET_DIR=target RSTYLE_DRIVER="$PWD/target/debug/rstyle-driver" \
  timeout 600 target/debug/cargo-rstyle clones -p rstyle-core \
  --extraction-profile hir-data-v3 --locked --compiler-advisory --compiler-plans \
  --format json --report-detail compact > /tmp/project-complexity-report.json
python3 eval/clones-capability/compiler_complexity.py \
  --report /tmp/project-complexity-report.json \
  --labels eval/clones-capability/project-complexity-cases.json \
  --output /tmp/project-complexity-results.json
```

For a fresh capture, bind a copy of `labels.run` to that explicitly selected new
run before reduction; retain the frozen sample expectations. The checked-in
results describe `run-4zkGtw`, not an implicit latest run. Full complexity scores
and contribution behavior are independently checked by the shared scorer tests
and the advisory CLI fixtures, including source-free replay, suppressions,
exclusions, exact reviews and changed-limit staleness. Offline reducer tests run
with the existing Python unittest discovery command.

### Large-project complexity quality (C49)

`project-pdg-complexity-cases.json` freezes four original `rstyle-pdg` library
owners against the same metric 6/policy 1/default 12 limits. Labels were audited
from source before comparing target scores, then associated with the explicitly
saved C48 run `run-WizKMH`. That capture already existed; this is an independent
source-label comparison, not a prospective capture experiment. All four exact
source strings match the captured inventory without metadata corrections.

The existing reducer finds eight agreements: four positive rule outcomes on
`check_contents` (CC 24 / COG 17) and `recursive_element` (19 / 18), and four
negative outcomes on `check` and `check_nominal_fields`. Expected clean scores
are source labels; observed clean outcomes do not claim numeric measurements.
The project has 7,034 measured owners, 6,494 unavailable owners and 142 unresolved
callees. These eight outcomes do not establish whole-project quality or cleanliness.

```sh
python3 eval/clones-capability/compiler_complexity.py \
  --report /tmp/clones-advisory-function-measure/pdg-after.json \
  --labels eval/clones-capability/project-pdg-complexity-cases.json \
  --output /tmp/project-pdg-complexity-results.json
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
```

For a fresh capture use the C42 command with `-p rstyle-pdg` and
`--compiler-advisory-only`, then bind a copy of `labels.run` to that explicit
new run, retaining the frozen labels. The checked-in results refer only to
`run-WizKMH`. The raw capture and command/resource observations are described in
`docs/performance/advisory-function-enrichment-samples.json`.

`project-pdg-complexity-review-results.json` records three serialized saved-run
replays using an ephemeral review file outside this repository. Dismissing unit
0 diagnostic 285 hides exactly the CC finding on `check_contents`; repeated
lookup retains `hidden_exact`. Raw evaluations and completed plans are unchanged
(the reviewed report additionally contains visible-diagnostic indices).
Reevaluating with `[complexity] max_cyclomatic=24, max_cognitive=12` removes that
CC finding at equality and retains its COG 17 finding. All replays exit 2 because
the original scope remains incomplete.

Retained review **status remains unavailable**: the existing original-owner
population exhausts its 4,096-identity budget on this project. This study therefore
does not verify changed-policy staleness or pruning. No budget increase, ignore,
suppression, repository dismissal, new collector/scorer or checksum was added.
The next bounded improvement is retained-owner observation reuse/selection while
preserving complete ambiguity checks and unavailable outcomes.

### Reuse full name counts for retained status (C50)

`project-pdg-complexity-retained-results.json` repeats the C49 temporary-review
study against exactly the same `run-WizKMH` inputs. No live recapture is needed.
The retained `check_contents` diagnostic now reports `hidden_exact` at the
original limits and `stale` when CC limit24 removes its CC finding. The policy
change prevents that negative from retiring the prior review. Raw evaluations
and plans equal their corresponding C49 replay outputs. Both project replays
still exit2 for the unchanged incomplete scope.

The implementation retains the existing complete compiler declaration-name
counts rather than constructing more full owner identities. Counts include
excluded/ambiguous declarations and are combined across matching package/target
variants. Check/test targets stay distinct. Missing retained owner payloads
remain unavailable; the 4,096 per-unit cap and whole-scope baseline limits are
unchanged. No new checksum, parser, source scan or scoring path is introduced.

The source-free Cargo acceptance case has 4,098 original owners. Both reviewed
diagnostics remain hidden by exact lookup; status resolves the retained-prefix
owner and keeps the omitted-tail owner unavailable. Changing the parameter
limit produces stale/unavailable status, with zero pruning candidates. Unit
cases also place a duplicate method beyond the 4,096 prefix and distinguish
shared-target feature variants, disjoint names, package roots and check/test
contexts. The project replay samples are 9.42s/9.26s at about 2.1GB peak RSS;
these are single local observations, not a speedup measurement.

### Source-labeled project parameter plans (C51)

`project-pdg-planning-cases.json` freezes the original
`eligibility::{diagnostic,error,site_of}` sources before consulting their target
scores or plans. This study explicitly sets `fn_params.max_params=3`; the
repository/default limit of four stays unchanged. The two four-parameter owners
are study positives, not default violations.

`compiler_planning.py` reuses the existing quality reducer's source/context and
completed diagnostic associations, then checks completed core planner output.
All eight checks agree: three parameter outcomes, one shared edit group,
callee-before-caller signature order, exactly three bare forwarded parameters,
their shared `&str` bundle, and no signature change for clean `site_of`.
The computed `&site_of(node)` argument does not become a bare forwarded parameter.
Other callers may enlarge the group; no exact total group/step count is asserted.
The comparator checks parallel owner rosters and preserves unavailable sources
and contexts, including the clean control, rather than inferring numeric zero.

Reproduce using the existing C48 saved run and a temporary policy file:

```sh
printf '[fn_params]\nmax_params=3\n' > /tmp/project-plans-policy.toml
target/debug/cargo-rstyle clones --extraction-profile hir-data-v3 \
  --compiler-advisory --compiler-advisory-only --compiler-plans \
  --replay-run target/rstyle/clones/run-WizKMH \
  --compiler-advisory-config /tmp/project-plans-policy.toml \
  --format json --report-detail compact > /tmp/project-plans-report.json
# Replay exits 2 because captured scope remains incomplete.
python3 eval/clones-capability/compiler_planning.py \
  --report /tmp/project-plans-report.json \
  --labels eval/clones-capability/project-pdg-planning-cases.json \
  --output /tmp/project-plans-results.json
```

The replay retains 7,034 measured and 6,494 unavailable owners, with 142
unresolved callees. A single local replay took 13.48s and 2,109,120KiB peak RSS;
this is not a speedup measurement. These source samples do not establish global
quality, full caller closure, safe rewrites or migration retirement. No new
production collector, planner, proof, checksum or capture was introduced.

### Opaque closure counter-evidence (C52)

The existing graph collector represents ordinary closures as opaque nodes; their
bodies and captures are not compared. `for_each` bodies handled as iteration by
the existing collector remain distinct from these opaque nodes. The existing WL
call-summary pass now retains one boolean and the existing cluster forwards it
from retained members. Helper/generic candidates expose the limitation as
counter-evidence, including at full recorded-node coverage. This adds no scan,
body copy, proof or hash. Candidate selection, score and identity policy stay
unchanged; no warning does not establish semantic equivalence.

`project-compiler-proposal-closure-cases.json` reuses C33's independent unchanged
source judgments and C37's explicit pipeline rejection expectation against fresh
metric-6 advisory-only `rstyle-core` run `run-mycjUM`. The corresponding results
retain two emitted not-useful groups and one explicit policy rejection; quality
exit1 is deliberate. All six emitted proposals contain opaque closures and now
warn. Their full candidate records equal the saved C37 records after removing
only the new warning and source line numbers. The source revision has advanced,
so this is not an identical-input or timing comparison. Four other proposals
remain unreviewed. Capture has 408 measured / 806 unavailable owners and exits2.
The study artifact records the single 7.65s / 433,784KiB observation without a
speedup claim.

The attempted historical `run-OFIAA9` replay rejected an unexpected `policy`
field in its saved pointer-width fact before analysis. It supplies no successful
current replay evidence; the original artifacts were preserved and the importer
was not weakened. The fresh capture and a separate real-Cargo source-free replay
exercise the warning through supported current observations.

```sh
target/debug/cargo-rstyle clones -p rstyle-core \
  --extraction-profile hir-data-v3 --locked --compiler-advisory \
  --compiler-advisory-only --compiler-patterns --format json \
  --report-detail compact > /tmp/closure-project-report.json
# Bind a copy of labels.run to that explicit fresh run; frozen source judgments stay unchanged.
python3 eval/clones-capability/compiler_proposal_quality.py \
  --report /tmp/closure-project-report.json \
  --labels eval/clones-capability/project-compiler-proposal-closure-cases.json \
  --output /tmp/closure-project-quality.json
# Quality exit1 retains the two independently unsuitable emitted proposals.
```

### Exact target contexts in shared quality review (C53)

All compiler-quality consumers now honor an optional `target_name` through the
shared `sample_owner` association: numeric/spacing/density labels, propagation
inputs, complexity labels, proposal sites/definitions and parameter-plan labels.
An explicit name must be a nonempty string and match the captured target name
in addition to package, kind and check/test selectors. Omitted names retain the
historical unique-context requirement; ambiguity stays unavailable. Numeric
comparison rows retain explicit target names, and independent labels for the
same shared function in different targets remain distinct. Overlapping named and
unnamed selectors cannot double-count a completed root or propagation owner.
C51 no longer needs
a separate prefilter. No runtime collection, proof, graph or hash changes.

`target-context-corpus` declares two binary targets that compile one shared
five-parameter function. `target-context-cases.json` freezes two positive shared
function outcomes and two clean `main` controls before capture. Locked run
`run-2ntpNc` has four measured owners, no unavailable owners and complete input
for that small declared scope. All four checks agree; the previous shared
association classified all four as ambiguous because it ignored target names.
`target-context-study.json` records commands and one 0.31s / 90,248KiB capture
sample, not a production-project quality or speedup claim.

```sh
target/debug/cargo-rstyle clones \
  --manifest-path eval/clones-capability/target-context-corpus/Cargo.toml \
  --extraction-profile hir-data-v3 --locked --compiler-advisory \
  --compiler-advisory-only --compiler-plans --format json \
  --report-detail compact > /tmp/target-context-report.json
# Bind a copy of labels.run to that explicit fresh run, keeping the source labels.
python3 eval/clones-capability/compiler_quality.py \
  --report /tmp/target-context-report.json \
  --labels eval/clones-capability/target-context-cases.json \
  --output /tmp/target-context-results.json
```

C49 complexity, C51 parameter-plan and C52 proposal results reproduce exactly
with their explicit saved report inputs. C52's frozen proposal owner labels now
also declare `rstyle_core` explicitly. Tests cover wrong/invalid names, two binary
targets sharing source, missing target-specific inventory, propagated callee
contexts and both proposal roles. The independent source-only check for a new
production concrete-trait IO study found generic callers instead; it supplies no
concrete-dispatch effect/planning quality claim.

### Complete raw proposal review and file selection (C55)

`project-compiler-proposal-complete-review-cases.json` preserves the three C52
source judgments and independently labels the four previously unreviewed groups
in the exact saved `run-mycjUM` rstyle-core capture. All six emitted groups are
judged not useful. Their shared collection/predicate shells hide different
closure computations, domain contracts or existing shared helpers. The seventh
label is the already rejected forwarding pipeline. Every emitted raw candidate
now has a source review in this declared sample; capture and discovery remain
incomplete, and no useful production positive or automatic suppression policy
has been established.

The offline reducer validates both `visible_candidates` (explicit reviews) and
`selected_candidates` (file selection) as unit-local index masks. Inventory
`visible` is their intersection. When a file mask exists, optional
`review_visible` and `file_selected` fields preserve the independent inputs;
without it the historical output fields remain unchanged. All raw candidates,
source-role matches and independent judgments remain inventoried. The quality
command reviews that raw scope and still exits1 for emitted negative judgments,
even when file selection hides every proposal. Missing inputs remain unavailable.
This is separate from the runtime CLI's selected finding exit.

`project-compiler-proposal-complete-review-results.json` records six structural
agreements plus the explicit policy rejection, all seven judged not useful.
`project-compiler-proposal-selection-quality-study.json` compares the exact C54
saved replays: global/align.rs/clip.rs inventories now show 6/1/0 visible candidates
rather than 6/6/6. Raw sample outcomes and judgments agree exactly in all three
cases. The C52 default result reproduces exactly with its original labels. The
study improves visibility accounting and completes source review; it does not
improve proposal precision, complete the project, or establish safe refactoring.

```sh
python3 eval/clones-capability/compiler_proposal_quality.py \
  --report /tmp/clones-c54-project/global.json \
  --labels eval/clones-capability/project-compiler-proposal-complete-review-cases.json \
  --output /tmp/complete-proposal-review.json
# Expected quality exit1. Use the explicit saved C54 replay, not an implicit latest run.
```

Tests cover both mask intersections, independent compilation units, invalid
indices, empty/absent candidates, unchanged historical fields, and a command-line
negative judgment hidden by file selection. No Rust collector/evaluator/proposer,
source scan, proof, graph copy or checksum changes.

### Exact completed complexity scores (C57)

The historical complexity reducer checks rule outcomes. A clean threshold result
alone cannot verify a numeric score. Add `--require-scores` to also compare
`policy_version`, `cyclomatic` and `cognitive` against independently authored
labels, including functions without diagnostics. Replay the explicitly selected
saved run with `--compiler-complexity-top N --compiler-plans` first; choose N large
enough to include the labeled owners. Ranking truncates presentation, not input
coverage. Missing rankings, omitted rows or null scores remain unavailable and
produce exit2, even if threshold comparisons agree. No scores are inferred from
an absent diagnostic and no parser, scorer, proof or checksum is added.

```sh
python3 eval/clones-capability/compiler_complexity.py \
  --report /tmp/clones-c57-core-ranked-report.json \
  --labels eval/clones-capability/project-core-complexity-score-cases.json \
  --require-scores --output /tmp/project-core-exact-scores.json
```

The opt-in output adds `score_counts` and `score_comparisons`; the ordinary
`counts` and `comparisons` retain threshold outcomes. Either comparison family
containing a disagreement or unavailable result makes the command exit2.
`scope_complete` remains the original whole-capture status; exact sample
agreement does not imply whole-project completeness. Without the flag, historical
output is unchanged, including when ranking fields happen to be present.

Exact score rows use the existing compilation-context/original source/span join,
then the unit-local ranking owner handle. Display lines/columns are positive
navigation metadata: leading doc comments/attributes mean a source label and its
score row can start on different lines. They never replace the original exact
source association. Paired owner rosters, identities, names, measurement counts,
summary counters and full missing-owner denominators are checked before use.

`project-complexity-score-quality-study.json` records explicit saved core and PDG
replays: all eight threshold outcomes per package still agree. Four PDG scores
agree exactly. Original core labels produce three exact agreements and one
mismatch, preserved in `project-core-complexity-frozen-score-results.json`:
Namer::render COG4 omitted three authored direct recursive self.render calls.
The documented policy yields saved COG7; CC12 and its clean default outcome are
unchanged. A separate `project-core-complexity-score-cases.json` records that
reviewed source-label correction, without rewriting C42 inputs/results. All four
scores then agree in `project-core-complexity-score-results.json`; the PDG score
results use the unchanged C49 labels. Core408/806 and PDG7034/6494
measured/unavailable-owner scopes retain exit2. Single replay observations are
not performance comparisons or legacy-retirement evidence.

### Evaluated owners and source-policy review (C58)

Captured compiler observations are retained even when core policy excludes their
paths. Numeric quality and parameter-plan validators now require evidence that a
sample was selected for evaluation before inferring a clean outcome from absent
diagnostics. They reuse completed plan owners, displayed complexity-ranking
owners, or an exact full selected-function count. When some owners were excluded
and no individual membership is available, the sample stays unavailable. Missing
population counts also stay unavailable. These offline checks do not implement a
second glob matcher, parse source again, or change native analysis.

Complexity and parameter-plan labels now require `unsuppressed: true`, matching
the existing compiler-quality precondition. Review the original owner attributes
and preceding source lines before setting it; an absent diagnostic alone cannot
establish this precondition. All existing numeric labels are preserved.

`project-compiler-evaluation-membership-study.json` records a native saved core
replay with `crates/rstyle-core/src/lines.rs` excluded. It retains408 captured
owners but evaluates404. The old comparator incorrectly accepted both clean
complexity outcomes for `lines::has_blank_line` and exited0. The corrected
comparator marks those two outcomes unavailable and exits2; the other six
threshold comparisons agree. The original core/PDG score results and PDG
parameter-plan results reproduce exactly. These are validation corrections,
not new runtime capability coverage, performance or retirement claims.
