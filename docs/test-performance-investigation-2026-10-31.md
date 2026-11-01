# Test capture consolidation, 2026-10-31

## First pass

Low audited and implemented the changes; Max independently reviewed the fixture
setups, assertion bodies, mutation isolation and measurements. The baseline is
`66a22342625b9b1f6d5ddda82a844cd070c606a1`.

The baseline workspace coverage run passed 3,789 tests. Its 1,187 CLI integration
tests accounted for 881.232 summed test-seconds. The other 2,602 cases accounted
for 43.998 seconds. Compiler captures are therefore a more useful target than
deleting inexpensive unit and boundary tests.

The changes remove **29 repeated compiler captures** across 16 integration-test
files. Most former tests become named assertion helpers that borrow the original
report. One `cfg_if` test is deleted because its complete setup and assertions
are already the prefix of the retained corruption test. There are 27 fewer
standalone test cases; loops account for the difference between cases and captures.

| Shared setup | Captures removed |
| --- | ---: |
| Default violations report | 3 |
| Default animals pattern report | 3 |
| Default literals pattern report | 1 |
| Fully subsumed `cfg_if` test | 1 |
| Accelerator packing corruption checks | 2 |
| Nine other immutable corruption-report groups | 9 |
| Positive mutation-effect cases | 6 |
| Control-effect composition and replay | 1 |
| Mutation provenance, presentation and policy replay | 2 |
| Guard selection and source-free replay | 1 |

The nine other corruption groups cover bitset cardinality, byte comparisons,
borrowed accelerator views, transition callbacks, sine tails, checked lengths,
external imports, native prefix readers and named callbacks.

Every distinct assertion, corruption variant and verifier entry point remains.
Each corruption uses a fresh pair, facts value or bundle. Fixtures stay independent
between tests, and their temporary directories outlive every assertion. Full, compact,
default, strict-score, policy and failure-gate invocations remain distinct.
Source/tool-free replay runs after the live-source checks; mutation policy is
loosened only after the strict replay. Determinism and text golden tests remain.

### Matched timing

Three baseline runs preceded three candidate runs on the same Linux ARM64
machine, using the pinned nightly, two build jobs, disabled incremental builds,
the CI nextest profile and four test threads. Each run selects the same 45
original test names. The candidate runs the 18 retained cases, including their
consolidated assertion helpers. All runs pass; the same 1,142 unrelated CLI
cases are excluded. JUnit runner times exclude compilation in the first command
of each group.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Affected tests, runner wall time | 9.754 s | 7.289 s | 25.3% |
| Affected tests, summed case time | 36.732 s | 25.310 s | 31.1% |

Runner ranges are 9.665–9.898 seconds and 7.247–7.301 seconds. These measurements
describe the affected selection; they do not establish a full CI speedup.

### Coverage and checks

The complete instrumented workspace suite passed **3,762 tests**, with the same
five existing skips, followed by passing workspace doctests. Comparing JUnit
identities confirms exactly the 27 intended removals and every other test retained.
The complete coverage run took 230.906 runner seconds versus 232.357 seconds in
the single baseline run: this is a modest overall change.

Coverage is **exactly unchanged**, including the identities of covered lines:

- 1,152 source files in both LCOV reports.
- 166,336 instrumented `(path, line)` entries in both reports.
- 154,098 covered entries in both reports, **92.6426%**.
- Zero lost or gained covered entries, and zero changed instrumented entries.

The coverage script, production source files, exclusions and thresholds are unchanged.
No new tests, suppressions or pattern-ignore entries were introduced. Source
review kept changed functions within the existing 50-code-line and four-parameter
limits.

Low ran `cargo fmt --all -- --check`, workspace/all-target Clippy with warnings
denied, and the actual shared-capture CI numeric and pattern gates, including
the saved-pattern status reader. All exited **0**. The configured analysis
covered 10,181 functions with zero numeric violations and zero visible pattern
candidates; the 12 existing reviewed ignores are unchanged. Max independently
accepted the source changes, matched timings and complete covered-line sets.

### Retained local evidence

[target/test-reduction-20261031](../target/test-reduction-20261031/) contains the
baseline LCOV and JUnit, matched selection and measurement script, three timing
reports per version, frozen source hashes, complete coverage comparison and
validation logs. `target/` is ignored by Git; this document preserves the method
and results independently of those local artifacts.

## Max-led second pass

Max selected the next consolidation groups from the test costs and independently
reviewed their source and state transitions before Low implemented them. The
baseline for this round is `e0e8e1fdc31a66e1c8471769f5a25cd34fc80d7d`, whose hosted
CI run passed on all three architectures. Max then reviewed the complete
eleven-file implementation diff and the individual scenario bodies.

This round removes **33 additional repeated compiler captures**. The same 42
original scenarios now run inside nine standalone tests; their distinct
assertions and compiler variants remain. Together, both passes remove 62
repeated captures and 60 standalone cases.

| Shared setup | Cases before | Cases after | Captures removed |
| --- | ---: | ---: | ---: |
| Source baseline export | 9 | 1 | 8 |
| Compiler diagnostic reviews | 5 | 1 | 4 |
| Compiler diagnostic baselines | 6 | 1 | 5 |
| Compiler diagnostic negative reviews | 6 | 1 | 5 |
| Compiler pattern reviews | 5 | 1 | 4 |
| Compiler pattern baselines | 5 | 1 | 4 |
| Accelerator debug setup before release | 2 | 1 | 1 |
| Native `u16` policy corruption | 2 | 1 | 1 |
| Scalar conversion corruption | 2 | 1 | 1 |

Each group owns one temporary workspace. Named local helpers restore the known
source, policy and saved files before every stateful scenario. Diagnostic groups
share the initial raw capture while retaining each scenario's configuration
replay, baseline export or six review dismissals. Source-baseline scenarios
restore the exact original baseline bytes. Pattern-baseline scenarios each
export anew with the destination absent. Review lookup checks start with no
review file, and source/tool-free replay and destructive prune checks run last.
Every corruption still starts from fresh facts, a fresh pair or a fresh document.
There is no global fixture cache or new fixture framework.

The slowest demanded-owner test still needs its original, clean-source and clean
release captures. Target, `no_std` and check/test contexts also remain distinct.
Max deferred broader fixture unification where the lifecycle complexity would
outweigh the small number of captures saved.

### Matched timing

Three baseline runs preceded three candidate runs on the same Linux ARM64
machine, with the pinned nightly, two build jobs, disabled incremental builds,
the CI nextest profile and four test threads. Each run selects the same 42
original fully qualified names; the candidate runs the nine retained tests and
their scenario helpers. All runs pass, and 1,118 unrelated CLI cases are excluded
in both versions. JUnit runner times exclude compilation; separate workspace
build and complete command times are retained in the local evidence.
The first candidate nextest command rebuilt the CLI for 12.82 seconds before
starting its measured test runner; the other two candidate commands did not
rebuild it.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Affected tests, runner wall time | 8.507 s | 6.363 s | 25.2% |
| Affected tests, summed case time | 28.328 s | 19.349 s | 31.7% |

Runner ranges are 8.455–8.531 seconds and 6.269–6.473 seconds. These are
measurements of the affected selection, not a full CI speedup estimate.

### Coverage and checks

The unchanged coverage script passed **3,729 tests**, with the same five existing
skips, followed by passing workspace doctests. The full JUnit identity comparison
found exactly the 33 approved standalone removals, no additions, and every other
test retained. The complete coverage command took 342.377 seconds, including
compilation and reporting. The test runner took 226.470 seconds versus 230.906
seconds for the baseline; one full run per version does not establish a general
CI speedup.

The complete LCOV comparison requires equality of the explicit source-file sets,
all instrumented `(path, line)` coordinates and all positive-hit coordinates.
Duplicate line records are aggregated, and source records with no line entries
are included. Both versions have:

- 1,152 source files and 166,336 instrumented entries.
- 154,098 covered entries, **92.6426%**.
- Zero gained or lost source files, covered entries or instrumented entries.

Production source files, the coverage script, exclusions, thresholds and CI
triggers are unchanged. No suppressions or pattern-ignore entries were added.
Max's supplementary multiline-aware source review covered all 59 modified or new
functions, with at most 48 nonempty body lines and three parameters.

Low ran formatting, workspace/all-target Clippy with warnings denied, an ordinary
CLI/driver build, and the actual shared-capture CI numeric and saved-pattern gates
sequentially. All exited **0**. The configured analysis covered 10,181 functions
with zero numeric violations, zero unavailable graphs and zero visible pattern
candidates. The 12 existing reviewed ignores are unchanged.

### Retained local evidence

[target/test-reduction-max-led-20261031](../target/test-reduction-max-led-20261031/)
contains the immutable baseline artifacts, exact test selection, six matched
timing JUnits and logs, frozen Rust source hashes, complete candidate LCOV and
JUnit, coverage and identity comparisons, Max's source and timing reviews, and
the actual validation command logs. These ignored local artifacts supplement
the method and results recorded here.

## Direct owner-boundary tests

Low implemented this change and Max independently reviewed the source and the
mapping of existing assertions to their replacements. The baseline is
`c35aa2bd76f5fa3e04e78ea75739f3d576f646de`.

The diagnostic lifecycle fixture now compiles three functions. Three focused
unit tests exercise the actual 4,096-owner payload budget using synthetic
function records, real core evaluation, `Demand::new`, and `resolve_owners`:

- A positive finding after 4,100 unrelated functions is retained. Late clean
  baseline and review owners pass through the actual assessed/retained logic as
  completed negatives; an excluded owner remains unavailable.
- Exactly 4,096 requested owners resolve successfully. A following unit
  collection sharing the same demand cannot resolve another owner.
- Repeated diagnostics consume one payload per owner, unrequested rules do not
  consume payloads, and the 4,097th requested owner retains the budget error.
  That error survives snapshot construction and prevents baseline publication.

The compiled lifecycle still covers baseline export and comparison, review
dismissal, resolved findings, missing measurements, review status, dry-run and
actual pruning, release-context changes, and replay without sources or compiler
tools. Its missing-measurement branch also checks that an incomplete CLI export
fails without creating a baseline. The former all-positive large-fixture test is
replaced by the direct budget/publication checks. Production executable code,
budget values, command-line options and environment settings are unchanged;
the production file only gains an appended `cfg(test)` module declaration.

### Matched timing

Three baseline runs preceded three candidate runs on the same Linux ARM64
machine with the pinned nightly, two build jobs, disabled incremental builds,
the CI nextest profile and four test threads. Compilation is measured separately
and excluded from the runner timings. The selection includes both the original
integration-test module and the new unit-test module: two baseline cases become
four candidate cases covering the same lifecycle and boundary behavior.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Affected tests, runner wall time | 11.441 s | 0.929 s | 91.9% |
| Affected tests, summed case time | 16.168 s | 1.222 s | 92.4% |

Runner ranges are 11.368–11.591 seconds and 0.874–0.947 seconds. The retained
lifecycle case alone changes from a median 11.439 seconds to 0.927 seconds.
These figures describe the affected selection; they do not establish a full CI
speedup.

### Coverage and checks

The complete instrumented workspace suite passed **3,731 tests**, with the same
five existing nextest skips, followed by passing workspace doctests. Comparing
all JUnit identities confirms exactly one removed large-fixture test, three
added boundary tests, and every other identity retained. The full runner took
225.151 seconds versus 226.470 seconds in the prior full run; these single runs
do not establish an overall performance improvement.

The baseline LCOV and full JUnit are reused from the validated `b1773d9c` run.
A Git comparison verifies that all Rust source, Cargo manifests and lockfile,
toolchain, coverage script and test driver script are identical between that
commit and this round's baseline `c35aa2bd`. The intervening commit changes
logging, Python helpers and documentation; test concurrency and execution are
unchanged. The candidate coverage run is fresh and uses the unchanged coverage
script.

The explicit source-file sets, instrumented line coordinates, and covered line
coordinates are exactly unchanged:

- 1,152 source files and 166,336 instrumented entries.
- 154,098 covered entries, **92.6426%**, with zero gains or losses.
- All 19,947 previously covered normalized function identities remain covered;
  nine additional test-closure instantiations of production helpers are covered.

Max independently reviewed the assertion mapping, frozen source hashes, six
timing reports, complete JUnit identities, line coordinates and normalized
function coverage. The 20 new or modified functions reviewed by Low have at most
43 nonempty body lines and three parameters. No suppressions, pattern-ignore
entries or threshold changes are introduced.

Low ran formatting, workspace/all-target Clippy with warnings denied, an ordinary
CLI/driver build, and the actual shared-capture CI numeric and saved-pattern
gates. All exited **0**. The numeric analysis covered 10,181 functions with zero
violations and zero unavailable complexity measurements; the pattern gate
reported no visible candidates.

### Retained local evidence

[target/owner-boundary-20261031](../target/owner-boundary-20261031/) contains the
baseline-reuse proof, frozen source hashes, all six timing JUnits, fresh complete
coverage and JUnit, exact source/line and test-identity comparisons, independent
Max source/timing/coverage reviews, Low's function-complexity review, and actual
gate logs. These ignored local artifacts supplement the method and results
recorded here.

## 4. Reuse verification results during admission and replay

The baseline for this round is `a5e05cc6`. In the checked ID-reader replay,
each of four valid functions first serialized and fully verified its documents,
discarded the decoded result, then fully verified the original documents again
through admission. Six changed-document cases independently repeated lowering,
analysis and emission of the same expected graph. Admission also repeated CFG,
operation-map, correspondence and value-site checks already completed by the
producer verifier.

### Implementation

`producer::verify` now accepts the existing conversions into `PairView` and
returns an immutable `ValidatedPair`. The result borrows the exact documents
and captured facts and retains the checked operation map and value sites.
`admit_validated` imports those documents using the retained results.
`admit_produced` performs verification and delegates to that same admission
path. The separate `verify_view` forwarding API is removed.

Local imported documents also reuse the checked indexes. They retain the
independent validation of caller-supplied source bytes and validation context;
those inputs can differ from the captured frontend. Admission-specific profile,
interface, type, reference and native-width checks remain in the import stage.

`PreparedVerification` borrows one function's immutable facts and builds its
expected documents once. Candidate checks retain document comparison,
helper/data verification and structural validation. Preparation and candidate checks
enforce strict verification even when called inside a generated-policy scope.
Only prepared comparisons clone their retained expected documents; ordinary
verification moves its freshly reconstructed documents into comparison.

The ID-reader, owned-composition, reader-composition and validator-composition
replay call sites use a shared snapshot roundtrip check that verifies manifest
identities and exact typed document equality. Those call sites run semantic
verification separately, and admission consumes its checked result. The ID-reader,
owned-composition and validator-composition mutation loops reuse one prepared
expected graph. All six original ID-reader mutations and independent
CFG bypass cases remain. Added assertions check prepared strictness inside a
generated scope and agreement between checked, produced and local-imported
admission without introducing another compiler fixture.

This is reuse within a verification/admission handoff and the selected replay
families. Other tests still explicitly exercise full serialized verification
and admission. Unit loading, derived-instance construction and later admission
also retain separate checks: derived stores can change facts and dependency
context, so this change does not reuse results across those owners.

### Matched timing

Three fresh baseline trials from a clean detached `a5e05cc6` checkout preceded
three candidate trials on the same Linux ARM64 machine, using the pinned
nightly, two build jobs, disabled incremental builds and the CI nextest profile
with four test threads. The selection contains the ID-reader, reader-composition
and validator-composition tests. The candidate trials ran after the full coverage
job finished, with no competing build. Timings below come from JUnit and exclude
compilation, including the first candidate run's 58.08-second test rebuild.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Selected tests, runner wall time | 5.841 s | 4.971 s | 14.9% |
| Checked ID-reader case | 5.839 s | 4.969 s | 14.9% |
| Validator-composition case | 3.666 s | 3.311 s | 9.7% |
| Reader-composition case | 2.230 s | 2.164 s | 3.0% |

Runner ranges are 5.730–5.939 seconds and 4.936–5.069 seconds. The
reader-composition ranges overlap, so its small median change does not establish
a speedup. These measurements describe the selected tests, not overall CI or
memory consumption.

A separate diagnostic trace records `producer.profile` invocations decreasing
from 158 to 129, with strict invocations decreasing from 122 to 93.
`producer.verify` invocations decrease from 264 to 258, or 113 to 106 under
strict policy, and the candidate makes seven preparation calls. Raw verification
counts include the added policy-regression assertions. Stage durations are
inclusive and must not be added together.

### Coverage and checks

The complete instrumented workspace suite passed **3,731 tests**, with the same
five existing nextest skips, followed by passing workspace doctest checks. Full
JUnit comparison finds exactly the same test identities and no failures. The
instrumented runner took 219.775 seconds versus the baseline's 225.151 seconds;
these single runs do not establish an overall speedup. Source and build inputs
were fingerprinted before and after coverage and remained unchanged.

Baseline coverage and JUnit come from the validated owner-boundary candidate
committed as `a5e05cc6`, using the same 3,731-test corpus. Candidate coverage is
a fresh run of the unchanged coverage script. Both reports contain 1,152 source
files. Covered/instrumented line entries change from **154,098/166,336
(92.6426%)** to **154,144/166,387 (92.6419%)**. The five additional uncovered
entries belong to the new non-exhaustive `Debug` formatter.

Mapping unchanged source text across edits finds zero previously covered,
still-instrumented lines becoming uncovered. Two unchanged closing braces are
no longer instrumented, and 78 previously covered entries belong to deleted or
replaced code. This comparison follows surviving source text; the retained
semantic and mutation assertions provide the behavioral checks.

Formatting, workspace/all-target Clippy with warnings denied, and the ordinary
CLI/driver build all passed. The actual shared-capture production-target style
gates measured **10,191 functions**, with zero numeric violations and zero
unavailable complexity measurements. Pattern capture and assessment completed
with no visible findings. No suppressions, ignore entries, thresholds or CI
triggers changed.

### Retained local evidence

[target/verification-reuse-20261031](../target/verification-reuse-20261031/)
contains the baseline provenance, frozen source hashes, full coverage and JUnit,
coverage/test-identity comparisons, all six timing JUnits, separate traces,
Low's executed checks, and independent Max reviews. The final style reports are
under `style/final-frozen-results`; earlier attempts and provisional measurements
are retained separately. These ignored artifacts supplement the method and
results recorded here.

## 5. Reduce repeated range-getter integration work

The baseline for this round is `bccf4045`. The range-getter integration test
previously captured the complete fixture for each installed target among
ARM64 Linux, x86-64 Linux and ARM64 Darwin, each in `std` and `no_std` mode.
Every context also repeated strict verification and admission of each admitted
function. The separate endpoint execution test provided an independent oracle.

### Implementation

The first available target in `std` mode retains the complete fixture and its
12 named admission checks, 13 exclusion checks and selected local `From`
implementation check. Remaining contexts use a smaller fixture containing eight
named supported functions, the selected implementation and two representative
unsupported input/storage cases. Shared source fragments retain the original
full fixture's function bodies; its only textual difference is one blank line.

Every context still exercises actual compiler capture, graph construction,
admission and matching, checks all four completeness flags, verifies the selected
`From` implementation identity, and requires an accepted exact `fresh`/`same`
pair. The test asserts that a comprehensive run occurred, preventing success
when no target is available. The separate 8-by-8 endpoint oracle and its expected
outputs and counterexamples remain unchanged.

The comprehensive run performs strict producer verification once per admitted
function and passes the returned `ValidatedPair` to admission. Report-entry
namespace checks remain. The 17 captured-fact mutations reuse one expanded bundle
and one successful baseline load, while every independent changed candidate is
still rejected under both ordinary and ambient-generated policy. The mutation
batch therefore makes 35 bundle load attempts instead of 51.

The four constructor graph mutations retain direct data-document rejection and
reuse one strict `PreparedVerification` against the same immutable facts. The
original graph first passes prepared verification and checked admission. The
historical-profile mutation additionally retains direct `admit_produced`
rejection. The other graph mutations no longer repeat that forwarding API's
producer verification. Production code, verification rules and CI triggers are
unchanged.

### Matched timing

Three baseline trials preceded three candidate trials on the same Linux ARM64
machine with the pinned nightly, two build jobs, disabled incremental builds and
the CI nextest profile with four test threads. The filter selects both range-getter
tests, including the unchanged endpoint oracle. All trials passed. JUnit runner
and case durations exclude Cargo startup and compilation; diagnostic traces were
collected separately. No other build or test job ran during the timing trials.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Both range-getter tests, runner wall time | 8.586 s | 5.087 s | 40.8% |
| Main capture/admission/matching case | 8.583 s | 5.085 s | 40.8% |
| Both tests, summed case time | 8.732 s | 5.228 s | 40.1% |

Runner ranges are 8.564–8.889 seconds and 5.071–5.117 seconds. The endpoint
oracle's medians are 0.149 and 0.143 seconds, with overlapping ranges. These
measurements establish the scoped test improvement, without measuring overall
CI performance or memory consumption.

The separate trace still records six completed compiler captures and six
matching stages. Unit decode/verification invocations decrease from 64 to 43,
producer verification from 1,058 to 479, and producer profile application
(`producer.profile`) from 796 to 273. The candidate makes one strict preparation
call. These counts reflect both smaller secondary fixtures and shared validation
work; they do not imply fewer compiler startups. Stage times are inclusive and
must not be added together.

### Validation

The final `scripts/coverage.sh` run passes all 3,731 tests and doctest checks.
JUnit preserves the exact multiset of baseline test identities with no failures,
errors or recorded skipped cases; nextest excludes the same five configured
tests before execution. The LCOV comparison is exact: both runs contain 1,152
production source files, 166,387 instrumented line coordinates and 154,144
covered line coordinates (92.641853%). No source, instrumented line or covered
line is lost or gained. Source/build/test input hashes match the previously
validated `bccf4045` baseline except for the intended range-getter test file;
all 1,964 fingerprinted inputs remain unchanged during the final run.

Formatting, whitespace checks and workspace/all-target Clippy with warnings
denied pass. Low ran the actual shared-capture CI numeric and pattern gates;
both return zero. The numeric report covers 10,191 functions with no violations
or unavailable complexity measurements. The pattern report has no visible
candidates; the 12 existing reviewed ignores are unchanged. Max independently
checked the source, final report hashes and gate results.

A separate all-target audit of the cargo-rstyle test corpus is not a clean gate:
it reports 3,332 findings across 6,462 measured functions. Its 11 range-getter
findings have causes already present in the baseline: unchanged corruption
helpers and completeness/oracle checks, plus IO in capture/test orchestration.
Max reviewed those causes directly. The new bundle-mutation helper has no
finding, and the file has no length, parameter-count or complexity finding.
No thresholds, suppressions or ignore entries were added.

Evidence remains under `target/range-getter-20261031`: the baseline/candidate
timing JUnit files and command records, `timing-comparison.json`,
`trace-comparison.json`, `junit-comparison.json`, `coverage-comparison.json`,
input fingerprints, final coverage and focused-test logs, CI style summaries,
and independent Max reviews. The tested source SHA256 is
`24f044f3d260cbad7303492397a4e041a9089b895e5329294f295afb373a97e8`.

## 6. Reuse digit-product verification prerequisites

Fresh CPU profiles at the `25210cb2` source state identified repeated strict
producer verification in the digit-product integration fixture. The ordinary
producer verifier appears in 308 of 447 samples (68.9% inclusive), including
193 samples in expected-graph reconstruction (43.2%). A separate control run
records 69 producer-verification invocations. These are overlapping CPU regions
in one exact test; sampled weights are not invocation counts or whole-suite
shares. Low collected the profiles and Max independently checked the raw counts,
exact function frames and binary hashes.

### Implementation

The parser and forwarder fixtures each load their clean captured unit once,
while retaining namespace and pair-existence checks for every selected report
entry. All three parser bodies and both wrapper bodies still exercise explicit
producer construction. Unaffected positive cases retain public
`admit_produced` acceptance, without a separate preceding producer-verification
call. Renamed parser graphs retain direct data-document and public-admission
acceptance.

The three-digit parser and hour-wrapper graph-mutation batches each create one
strict `PreparedVerification` tied to their unchanged facts. Each original graph
first passes prepared verification and `admit_validated`. All 11 parser and
eight wrapper graph mutations retain independent direct data-document rejection
and prepared-verification rejection. Historical profiles and a representative
semantic mutation in each batch also retain direct public-admission rejection.

The two coherent ownership mutations and two fresh helper-selection mutations
retain independent verification of changed facts. The summary fixture, its
seven mutations under ordinary/generated policy, missing-certification check
and historical-policy assertion are unchanged. Component, conditional-read,
renaming, matching and completeness assertions remain. Production source and
verification rules are unchanged; this pass removes repeated test setup and
expected-graph reconstruction using the existing APIs.

### Matched timing

Three baseline trials preceded three candidate trials on the same Linux ARM64
host, using the pinned nightly, two build jobs, disabled incremental compilation
and the CI nextest profile with four test threads. The selection includes all
three digit-product tests. No competing builds or test jobs ran during these
measurements. All trials pass the same three case identities. JUnit runner/case
times exclude Cargo startup and compilation; separate serial diagnostic runs
provide stage counts.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Three-test runner wall time | 4.525 s | 3.118 s | 31.1% |
| Main parser/composition/mutation case | 4.523 s | 3.116 s | 31.1% |
| Forwarder case | 1.413 s | 1.157 s | 18.1% |
| Three tests, summed case time | 7.524 s | 5.924 s | 21.3% |

Runner ranges are 4.498–4.608 seconds and 3.105–3.155 seconds. The unchanged
summary fixture has medians of 1.615 and 1.651 seconds, with overlapping ranges;
no summary-test speedup is claimed. These scoped results do not establish
whole-suite/CI speedups or reduced peak memory.

The separate serial traces of all three cases record 30 versus 27 unit decode
and verification invocations, 136 versus 105 producer-verification invocations,
and 130 versus 80 producer profile applications. The candidate makes two strict
preparations. Compiler captures remain three and producer construction stages
remain 19. The producer-verification count includes the new prepared-candidate
checks; it does not alone count expected-graph reconstructions. Profile
application means transport/data annotations, rather than graph analysis.
Stage durations include nested work and must not be added together.

### Validation

Low's final focused tests, formatting, whitespace checks and workspace/all-target
Clippy with warnings denied pass. Low also ran the actual shared-capture CI
numeric and pattern gates; both return zero. The numeric report covers 10,191
functions with zero violations or unavailable complexity measurements, and the
pattern report has no visible candidates. Existing reviewed ignores are
unchanged.

The separate cargo-rstyle all-target test audit reports 3,327 findings overall,
down from 3,332, and is not a clean gate. Local digit-product findings decrease
from 21 to 16: the main function's length and spacing findings, the forwarder's
spacing/density findings and renamed-node spacing finding are removed. Both new
helpers are clean. Remaining findings concern unchanged mutation/source/oracle
helpers and summary tests, plus the main test's existing IO density. Its density
ratio rises from 521 to 571 after moving the pure source-selection lookup; its
four IO sites remain the same. No threshold, suppression or ignore change is
used to obtain these results.

The final `scripts/coverage.sh` run passes all 3,731 tests and doctest checks.
JUnit contains the exact baseline case identities, with no failures, errors or
recorded skipped cases; the five configured exclusions remain unchanged.
LCOV source sets, instrumented and covered line coordinates, and instrumented
and covered function-symbol sets are all exactly equal. Both runs cover 154,144
of 166,387 lines across 1,152 production source files (92.641853%) and 26,394 of
40,761 instrumented function symbols. No covered line or function is lost.
The baseline's 1,964 source/build/test fingerprints differ only in the three
intended digit-product test files, and every candidate input remains unchanged
during the full run.

Reproducible measurement and comparison scripts, timing JUnit files, raw
diagnostic traces, frozen source hashes, final logs, baseline/candidate LCOV and
JUnit reports, style summaries and independent Max reviews remain under
`target/digit-verification-20261031`. The frozen Rust source hashes are recorded
in `candidate-measurement-command.json`, `candidate-ci-gates-summary.json` and
`coverage-input-sha256.json`. `coverage-comparison.json` and
`junit-comparison.json` record the exact comparisons.

## 7. Retain validation across the production handoff

This round uses `e65d31ed` as its baseline. It changes production code, without
consolidating or removing any existing tests. Current-run bundle loading already
skips expected-graph reconstruction, but it still checked graph structure and
built operation/value-site indexes. Those results were discarded. Later helper
verification and matching admission repeated the same producer checks.

### Implementation

`FunctionView::verified_pair()` selects the function's own root from its exact
immutable `UnitStore`. Successful producer validation and its existing indexes
are retained privately by graph ID and verification policy. `ValidatedPair`
shares the indexes through `Arc`; it continues to borrow the exact documents
and facts. Loading, helper verification and CLI admission use this same path.

The standalone `producer::verify` remains uncached for independently supplied
pairs and facts. Generated cache entries cannot satisfy strict cache lookups. Failed
producer checks are not retained. Mutable builders do not use the store cache, and
newly decoded, copied or derived owners start with empty caches. Nothing about
the cache is serialized. Helper boundary and summary checks still run, and
complete helper-proof caches remain local to an analysis context.

Four focused regressions cover reuse across separate analyses, policy isolation
and failed verification, current-run loading, and fresh validation after copying
retained tables. Modified detached graphs and strict imports still reject invalid
evidence. The shared fixture adapts one existing captured-facts seed into a coherent
empty function and creates no compiler projects.

### Matched local ARM64 timing

Three baseline runs preceded three candidate runs on the same Linux ARM64 host,
with the pinned nightly, two build jobs, incremental compilation disabled and
the CI nextest profile's four test workers. No competing build or test job ran
during measurement. Workspace compilation was performed before each group's
runner measurements; doctests are checked separately by the coverage command.
Rust/build/test input fingerprints stayed frozen throughout each group.

| Metric | Baseline median | Candidate median | Reduction |
| --- | ---: | ---: | ---: |
| Full workspace nextest runner | 220.370 s | 218.992 s | 0.63% |

The three baseline times are 220.285, 220.371 and 220.370 seconds. Candidate
times are 218.992, 218.978 and 219.640 seconds. Every run passes; the candidate
retains all 3,731 original case identities and adds four regressions, for 3,735
cases. The same five configured exclusions remain. This is an observed small
local test-run improvement, rather than a whole-CI or cross-platform result.

### Ordinary production commands

Separate measurements run the normal CLI directly on the stored-slices and
ID-reader fixtures, without a test harness or a strict replay loop. Each group
uses one warmup and five measured current-run captures, followed by a separate
`RSTYLE_PERF=1` diagnostic run. Fixture bytes and CLI/driver binaries remain
frozen within each group. Expected admission/exclusion cases and matching
assertions run for every trial; complete normalized function outcomes, reasons,
matching decisions and scores are identical between baseline and candidate.

| Workload | Baseline CLI verification calls | Candidate calls | Baseline wall median | Candidate wall median |
| --- | ---: | ---: | ---: | ---: |
| Stored slices, 17 admitted functions | 40 | 17 | 0.986 s | 0.991 s |
| ID readers, 13 admitted functions | 34 | 13 | 2.763 s | 2.766 s |

The stored-slice wall ranges are 0.969–1.008 and 0.975–1.003 seconds; ID-reader
ranges are 2.700–2.806 and 2.725–2.796 seconds. These overlap, and no end-to-end
CLI speedup is established. Compiler captures remain one per command. Including
the compiler-driver process, producer verification calls fall from 63 to 40 and
55 to 34; the driver's mutable-builder work is unchanged. These are instrumented
invocation counts, rather than CPU samples.

Retaining indexes increases their lifetime to that of the owning unit. GNU
time's median maximum RSS observations are 141,436 to 143,444 KiB for stored
slices and 148,484 to 150,620 KiB for ID readers. The candidate medians are about 2 MiB (1.4%) higher, and the observed RSS
ranges do not overlap. This is propagated maximum process RSS, which can be dominated by the compiler; it does not measure
simultaneous process-tree memory or allocations. No memory improvement is
claimed. Structural validation is a small part of these representative runs;
source/helper analysis remains outside the removed duplicate checks.

### Validation

Formatting, whitespace checks and workspace/all-target Clippy with warnings
denied pass. The actual final production CI numeric and saved-pattern gates
both exit zero, with no violations or visible pattern candidates. No CI trigger,
threshold, suppression, ignore entry, fixture or existing assertion is changed.

The supplementary rstyle-pdg all-target numeric audit is not a clean gate: it
observes 16,997 functions, reports 1,424 findings, and exits two because compiler
advisory coverage is incomplete. All seven new test/helper functions have actual
numeric observations and zero findings. There is no baseline comparison of the
remaining broad-audit findings.

The final unchanged `scripts/coverage.sh` passes all 3,735 tests and workspace
doctest checks, with the same five configured exclusions. Coverage and normal
runs have exactly the same case identities. All 3,731 baseline cases remain;
only the four intended regressions are added. All 2,314 tracked or unignored
input files stay frozen during the full coverage command.

LCOV line coverage rises from 154,144 of 166,387 lines (92.64185%) to 154,251 of
166,494 (92.64658%). There are 1,153 source files, including the new validation
module. Mapping unchanged source lines across the edits retains coverage for
all 154,127 mapped baseline-covered lines. No unchanged covered line loses its
hit. Added or changed instrumented lines in the existing files are all covered.
The new module covers 26 of 27 instrumented lines; its missing-root error branch
is the uncovered line. The existing function/symbol counts change with the new
implementation; raw symbol counts do not establish one-to-one function coverage
preservation. These reports contain line and function coverage, without branch
coverage records.

Reproducible scripts, frozen input/binary hashes, all six normal timing JUnits,
production reports and balanced raw traces, GNU time observations, coverage
reports, style outputs and independent Max reviews remain under
`target/structural-validation-20261031`.

## 8. Put direct checks beside their owning Rust modules

This pass starts from merged main `3a1e0069`, whose Git tree is identical to
`a44e2ace`. Direct checks of hand-built inputs belong in owning `#[cfg(test)]`
modules. Compiler capture, cross-component admission, transport and compiler
authority contracts remain integration tests. Invalid-input checks protect
ordinary extraction, serialization and cache regressions; they do not require
an adversarial environment.

Low implemented the moves and Max independently reviewed their source,
assertions, placement and evidence. The following 41 tests retain every helper
and assertion, with only imports and qualified paths adjusted:

| Former file under `crates/rstyle-pdg/tests/` | New module under `crates/rstyle-pdg/src/` | Cases |
| --- | --- | ---: |
| `eligibility_tests.rs` | `eligibility/tests.rs` | 18 |
| `assertions.rs` | `assertions/tests.rs` | 3 |
| `bool_constants.rs` | `bool_constants/tests.rs` | 1 |
| `byte_literals.rs` | `byte_literals/tests.rs` | 1 |
| `conditional_source.rs` | `conditional_source/source_tests.rs` | 2 |
| `scalar_constants.rs` | `scalar_constants/tests.rs` | 4 |
| `scalar_patterns.rs` | `scalar_patterns/tests.rs` | 2 |
| `source_ownership.rs` | `source_evidence/ownership_tests.rs` | 7 |
| `frontend_facts.rs` | `frontend/tests.rs` | 3 |

One additional origin-cache regression moves out of the compiler-backed CLI
suite into `unit_store/derived/reference_origins/flow_tests.rs`. A small typed
source-flow fixture exercises the real `UnitStore`, `UnitAnalysis` and
`FunctionView::reference_origins` route. It checks that saving `a` retains its
origin after another local is assigned `b`, with repeated queries, edited-node
and selector failures, recovery and both verification policies. It creates no
Cargo project and launches no compiler. Existing reference rebinding and stored
slice integration cases retain the real compiler, graph verification and
admission assertions. No public API was added for tests.

### Local ARM64 execution timing

Three baseline runs preceded three candidate runs on the same Linux ARM64
host, using the pinned nightly, two build jobs, disabled incremental compilation
and the CI nextest profile's four workers. No other build or test workload ran
during the timing trials. Compilation is excluded. All six runs pass the same
3,735 cases after exactly 42 reviewed identity mappings; the same five configured
exclusions remain. Source/build/test input fingerprints stay frozen within each
measurement group.

| Metric | Baseline | Measured candidate |
| --- | ---: | ---: |
| Full normal nextest median | 220.541 s | 218.264 s |
| Origin-cache case median | 0.275 s | 0.007 s |
| Test binaries reported by nextest | 31 | 22 |

Baseline trials are 221.016, 220.232 and 220.541 seconds; candidate trials are
218.380, 218.264 and 218.064 seconds. The observed suite reduction is 2.277
seconds, or 1.03%. This does not isolate the contribution of one removed capture,
establish a cold-build or whole-CI improvement, or measure other architectures.
The 41 relocated cases already used hand-built inputs; their placement changes
remove nine standalone test binaries. Only the additional origin conversion
removes compiler capture.

After timing, the supplementary test-target style audit requested three blank
phase separators in the new fixture. Exactly three empty lines were inserted;
all nonempty lines and all other measured inputs remain identical. Independent
hash and diff proofs preserve that distinction. The final source was not
re-timed; final coverage and style validation use the corrected source.

### Coverage and final checks

The final unchanged `scripts/coverage.sh` passes all 3,735 tests and workspace
doctest checks. Its input fingerprints remain frozen throughout execution. The
coverage run has exactly the same cases as the normal runs after the reviewed
remap. The retained baseline coverage comes from the identical pre-merge tree;
all 2,314 historical input hashes were checked, including 145 JSON inputs. The
only historical difference is the investigation appendix added after that run.

The unique LCOV `DA` source-line records are exactly preserved: 154,251 of
166,494 lines have hits, **92.64658%**, across the same 1,153 source files.
No previously covered `DA` source line loses its hit. LCOV's separate `LF`/`LH`
summary totals differ: `LF` remains 169,594, while `LH` falls from 156,381 to
156,365, or 92.20904% to 92.19961%. Those summary counts differ from the `DA`
cardinality in both reports. The six files with lower `LH` retain exactly the
same `DA` hit sets; generated function copies receive different execution
counts, but the precise exporter-summary mechanism is not established. Global
`FNF`/`FNH` totals remain 18,428/17,152. This proves source-line hit preservation,
without claiming that every LCOV summary counter is unchanged. The unchanged
cargo-llvm-cov default filename filter omits the old integration test files,
their relocated test files and `flow_tests.rs`. Their execution is verified by
JUnit, rather than a claim of test-helper line coverage. This report has no
branch-coverage records. The final instrumented command takes 329.189 seconds;
that is separate from the normal runner benchmark.

Formatting, whitespace checks, workspace/all-target Clippy with warnings denied,
and the actual production CI numeric and saved-pattern gates pass. All six new
flow helper/test functions are observed in the supplementary all-target numeric
audit with zero findings. The broader audit remains incomplete: exit two,
17,003 observed functions and 1,424 findings elsewhere. It is not a clean gate,
and counts alone do not establish those findings' baseline provenance.

No production behavior, CI trigger, threshold, suppression or ignore entry is
changed. Raw timing JUnits, explicit case mappings, preserved-source proofs,
coverage reports, scope evidence, final style outputs and independent Max
reviews are retained under `target/unit-test-boundaries-20261101`.

## 9. Reuse compiler captures and immutable evidence

This pass starts from `36f3e1e3`. It removes repeated setup while retaining compiler
capture, serialization, verification, admission and matching. Checks reuse one
immutable `UnitStore`; mutations receive independent clones or edited wire.
Production, fixture files, CI, thresholds, suppressions and ignores stay unchanged.

Low implemented the changes and Max reviewed source, assertions and work counts.
With all three configured targets installed, the source/call-graph inventory is:

| Family | Compiler captures before → after | Direct test-side bundle loads before → after |
| --- | ---: | ---: |
| Input owners and stored slices | 37 → 29 | 26 → 19 |
| Accelerator packing | 9 → 8 | 50 → 8 |
| Word classifiers | 15 → 7 | 15 → 7 |
| Architecture readers | 5 → 4 | 14 → 6 |
| Total | 66 → 48 | 105 → 40 |

That removes **18 capture setups and 65 direct successful test-side bundle loads**.
Counts exclude all native oracle compilations, target-library queries, CLI-internal
imports and intentional rejection/control imports. These are static work counts.

The six stored-slice target/std captures include the complete passive-owner source
and replay its checks. An exact guard checks admitted keys before the first
appended borrowed method against the original eight keys, including generated
source callsites. Matching requires all 28 unique nonself pairs and the accepted
`caller`/`same` pair. Architecture counts every function in its original source
prefix against eight and retains all eight named outcome checks. These guards
preserve the original passive admission inventory and architecture function
inventory. Word retains its complete 20/11 partition of 31 functions;
accelerator packing retains all 19 functions.

Target and std/no_std matrices, completion flags, old-policy rejections, saved
receiver/read contracts, mutation scenarios and rejection entrypoints remain.
Same-source details run once in a retained std context. Distinct mutated sources,
accelerator release capture, native debug-policy oracles and the independent
256-byte ASCII oracle remain. Word fixtures share one namespaced capture per
context, adding 780 candidate matching pairs within the six-context matrix.
Measurements include that matrix overhead and the removal of separate host
mutation captures.

### Final normal execution and resources

Seven reviewed many-to-one case consolidations change 3,735 passing cases to
3,728 across the same 22 test binaries. Explicit mappings and source review account
for each consolidated case and its assertions. The five pre-existing nextest skips
remain outside that executed JUnit population. Baseline groups freeze their
original inputs; final candidate groups freeze the reviewed sources, including
both exact prefix guards. Earlier timings and coverage attempts without those
guards are archived and discarded from final result claims.

| Metric | Baseline | Final candidate |
| --- | ---: | ---: |
| Focused normal nextest median | 20.294 s | 15.592 s |
| Full normal nextest median | 218.009 s | 215.097 s |
| Focused per-trial user + system CPU median | 64.27 s | 50.31 s |
| Focused median largest-process RSS | 144,496 reported kbytes | 144,496 reported kbytes |

Focused trials are 20.405, 20.102 and 20.294 seconds before, and 15.680, 15.566
and 15.592 seconds after: a 4.702-second, 23.1694% median reduction. Full-suite
trials are 217.988, 218.204 and 218.009 seconds before, and 215.343, 214.498 and
215.097 seconds after: a 2.912-second, 1.3357% median reduction. The focused CPU
metric sums user and system time within each trial before taking the median.
Largest-process RSS is not total concurrent memory; full normal runs do not
include resource sampling with `time -v`.

These local Linux ARM64 measurements exclude compilation and use nightly
`2026-09-30`, four CI-profile workers, two build jobs and disabled incremental
compilation, with no competing heavy jobs. The four families account for about
7.4% of summed baseline test duration; other cases remain. Their localized saving
therefore produces a modest full-suite gain. These results do not establish a
cold-build, whole-CI or other-host improvement.

### Final coverage and checks

The unchanged coverage command passes all 3,728 cases and four doctest targets
with zero doctests each. Inputs remain frozen. Normal and coverage JUnit identities
agree after the seven reviewed consolidations. The collector takes 322.413 seconds,
separately from the normal benchmark.

The same 1,153 source files retain exactly the same unique `SF` + `DA` line keys
and hit sets without remapping: 154,251 of 166,494 lines, or 92.64658%, have hits.
The separate raw LCOV summary totals are also unchanged: `LF` 169,594, `LH`
156,365, `FNF` 18,428 and `FNH` 17,152. Raw `LF`/`LH` coverage is 92.19961%.
Unique source-line records and exported summary totals remain distinct metrics.

The unchanged cargo-llvm-cov default filter omits test directories and
`tests.rs`/`*_tests.rs` files equally before and after. JUnit establishes execution;
LCOV does not establish instrumentation or line coverage for omitted helpers.
No branch coverage is claimed.

Actual production CI numeric and saved-pattern gates pass with exit zero in v3.
Only test guards change afterward; v5's `ci-gate-provenance.json` records that
retained evidence. Final formatting, whitespace checks and workspace/all-target
Clippy with warnings denied pass. All 34 changed functions are observed in the
final supplementary numeric audit with zero non-density findings. Its broad
scope remains incomplete: exit two, 6,469 functions and 3,303 findings. All seven
changed-scope IO-density findings exist in the measured baseline; five values
are lower and two are higher due only to statement-denominator changes, with the
same classified IO effects. The broad audit is not clean. Max's manual source
pattern review approves the changed helpers separately from the automated
production saved-pattern gate.

Source reviews, work inventories, case mappings, timing JUnits, resource logs,
coverage and style evidence are retained under
`target/compiler-test-consolidation-20261101`.

## 10. Optimized test-build experiments

This experiment starts from `ba04f19c` and compares the existing test profile with
two optimization candidates. Both candidates preserve debug assertions, overflow
checks, debug symbols and unwind behavior. Actual unit graphs and verbose compiler
invocations record the effective CLI/driver profiles. Optimization settings live
in Cargo.toml, so tiny captured fixture projects retain independent Cargo profiles.

| Profile | Workspace configuration | Third-party configuration |
| --- | --- | --- |
| Baseline | Existing test opt0 | Existing four hot dependency overrides opt2 |
| Global opt1 | Test opt1 throughout | Global test opt1, existing hot overrides opt2 |
| Targeted opt1 | Core, PDG, ccgraph and driver opt1; CLI/harness and extraction fixtures opt0 | Exactly baseline profiles |

The existing hot overrides are serde_json, pinned syn 2.0.119, proc-macro2 and
xxhash-rust; unrelated syn 1 remains opt0 for baseline and targeted. Max verifies
the same 271 units for baseline and targeted: 48 selected workspace units differ
only in opt_level and all 215 third-party effective profiles remain equal.
Selected package tests, examples, doctests and driver build units also receive
opt1; CLI production, unit and integration targets remain opt0.

Each phase uses a separate initially empty target directory, nightly 2026-09-30,
two build jobs, disabled incremental compilation and the four-worker CI nextest
profile. Builds, warm preparation and three normal trials run sequentially with
no competing heavy jobs. The same download cache is available; sequential cache
and filesystem effects remain a limitation. Normal trials reuse prebuilt metadata
and an explicit prebuilt driver. Inputs and used binaries stay frozen per phase.

### Measurements and decision

| Metric | Baseline | Global opt1 | Targeted opt1 |
| --- | ---: | ---: | ---: |
| Empty-target no-run build | 175.585 s | 495.462 s | 389.758 s |
| Warm unchanged no-run preparation | 0.353 s | 0.339 s | 0.401 s |
| Median normal nextest run | 213.799 s | 138.135 s | 157.166 s |
| First build + median run (sum) | 389.384 s | 633.597 s | 546.924 s |
| Median normal user + system CPU | 859.38 s | 561.84 s | 638.29 s |
| Median largest-process RSS | 228,844 reported kbytes | 216,632 reported kbytes | 216,504 reported kbytes |

Baseline trials pass at 214.006, 213.451 and 213.799 seconds.
Global trials all pass at 138.432, 138.135 and 137.820 seconds. Their median is
35.390% faster, but the first build costs 319.877 seconds more. Build plus one
median run is 244.213 seconds slower. The scoped estimate breaks even after five
normal runs reusing that initial build; it does not predict future code-edit
recompilation or whole-CI performance.

The targeted build reduces the global candidate's compile penalty, but still
costs 214.173 seconds more than baseline. That extra build time exceeds the entire
213.799-second baseline run, so it cannot win a fresh build followed by one run.
Targeted trials pass at 157.423, 156.729 and 157.166 seconds: a 56.633-second,
26.4889% median saving. Build plus one median run is 546.924 seconds, or 157.540
seconds (40.46%) slower than baseline. The scoped estimate breaks even after four
normal runs reusing the initial build. Warm unchanged preparation takes 0.353,
0.339 and 0.401 seconds for baseline, global and targeted respectively.

CPU is the median of user plus system seconds summed within each normal trial.
Largest-process RSS is not simultaneous total memory. The build-plus-run row sums
one initial build observation and the three-trial runner median; it is not a
measured whole-CI duration. First-build, warm unchanged preparation and normal
runner times are separate measurements. The speedups apply
to this local Linux ARM64 workload and do not establish arbitrary edit/build or
other-host performance.

Both candidates are rejected for the fresh-build-plus-one-run use case. Warm
execution gains come with a larger compile cost; neither configuration is adopted.
The Ubuntu x64 coverage lane uses a runner-temporary build directory outside the
normal Cargo cache, making fresh-build cost relevant. Optimized instrumented
coverage and edited-source recompilation timings remain unmeasured.

### Validation and restoration

All nine normal trials pass the exact same 3,728 case identities across 22 test
binaries, with five pre-existing nextest skips outside that executed population.
Each phase preserves all tracked/unignored input and used-binary hashes.
`comparison.json` and `comparison-targeted.json` pass the identity, profile,
resource and exact cross-mode source-change checks.

During the experiment, the test helper fallback selects profile test to build a
compatible driver. The measured runs explicitly set RSTYLE_TEST_DRIVER and return
before that fallback. A temporary CI cache-key edit includes root Cargo.toml for
profile cache coherence; triggers and other workflow bytes remain unchanged.
The comparer restricts cross-mode source differences to that workflow expression,
the exact fallback argument change and the reviewed Cargo.toml profile settings.

All three implementation files are restored byte-for-byte to `ba04f19c`:
`Cargo.toml`, `crates/cargo-rstyle/tests/cli.rs` and `.github/workflows/ci.yml`.
Before adding these findings, `restoration.json` verifies all 2,315 tracked and
unignored input hashes equal the measured baseline and the tracked diff is empty.
Only this findings report is retained; neither optimization candidate is adopted.

Neither discarded profile receives a coverage run or new CI/style certification.
The byte-identical restored implementation retains the coverage and CI/style
evidence in section 9. That evidence does not certify the abandoned profiles.
Final formatting and whitespace checks pass. No optimized-profile coverage or
broad style-clean claim is made.

Independent Max reviews and raw unit graphs, invocation logs, empty-target build
resources, warm preparation logs, three-trial JUnits, fingerprints and comparison
reports are retained under `target/optimized-tests-20261101`.

## 11. Direct semantic unit tests

The next experiment removes compiler launches from checks whose assertions concern
native comparison, helper policy and report summaries. Low implements the move;
Max independently reviews fixture fidelity, assertion mappings, test ownership,
complexity and measurement evidence. Production functions and APIs, Cargo profiles,
CI configuration, style thresholds and saved ignores remain unchanged.

Five standalone CLI identities are retired. Their semantic checks move into private
unit modules; the native budget-report assertions also transfer verbatim into the
existing live helper-budget test. The alpha-only iteration of the positive helper
test moves to a unit case while its literal-hole compiler capture remains.

| Checks | Unit destination / retained integration |
| --- | --- |
| Forwarding, one-operation usefulness boundary and noncommutative formal reordering | Three `pair_differences::tests::helper_policy` cases |
| Two-operation alpha-only helper policy | `helper_policy`; real literal/HIR/span/advisory positive stays integrated |
| Pairwise evidence that is not transitive | `helper_policy`, retaining `(x+1)*2`, `(x+3)*2`, `(x+3)*4` |
| Complete native u64 comparison outside u32 helper policy | `helper_policy`, including real report completion predicate |
| Operator and subtree-shape differences | Strengthened existing `pair_differences` unit, with unconditional comparison/helper assertions |
| Incomplete matching denominator versus known-empty/unrequested summaries | Owning `pair_reports/tests.rs` and `helper_reports/tests.rs`; one live CLI capture checks both budget reports |

The new tests use the actual native comparison, helper evaluator, summarizers,
serializers and `permits_success` predicates. They reuse private typed builders.
One-formal inputs retain exactly one formal; the two-formal alpha case retains XOR
then AND with 15 and the original parameter order. These synthetic HIR expressions
are algorithm inputs, with no claim of compiler extraction or strict admission.
No public test-support API or new captured snapshot is introduced.

Compiler extraction still has the independent eager scalar operator/type/operand
matrix. Live tests retain literal source evidence, unknown calls and mutation,
supported-plus-unavailable owners, native formal/literal differences, full/compact
and text reports, opt-in flags and CLI statuses. The operator/shape unit now always
reaches the helper rejection branch; its old CLI check could accept an empty
accepted-pair population.

| Static compiler-capture inventory | Before | After |
| --- | ---: | ---: |
| Helper CLI tests | 18 | 10 |
| Native-difference CLI tests | 6 | 5 |
| Total for these two groups | 24 | 15 |

The initial coverage comparison found one lost source-line hit: constant
multiplication in the producer hazard assessor, previously reached incidentally
by `x+(1*2)`. A unit test under `producer/hazards/tests.rs` now runs real graph
analysis and hazard assessment. It checks that a folded product becomes the
denominator of a safe division, while zero, target-width overflow and an unknown
factor retain the appropriate limitations. This preserves useful producer
behavior coverage without another compiler capture.

That removes nine compiler captures. Five retired CLI identities and nine new
unit identities change 3,728 executed cases to 3,732, across the same 22 binaries.
The five existing nextest skips remain outside that executed JUnit population.
Explicit case/assertion mappings account for the moves, partial alpha iteration,
strengthened operator/shape case and shared live budget capture.

### Normal timing and resources

The current full baseline control takes 214.185 seconds, consistent with section
10's same-implementation median of 213.799 seconds. The initial candidate passes
one full normal run at 215.044 seconds, before the producer unit is added. Those
single full-suite observations establish no global speedup and are not a timing
claim for the final source. The retired CLI identities account for only 1.989
seconds of summed baseline case duration, before the partial alpha move.

| Focused metric | Baseline | Candidate |
| --- | ---: | ---: |
| Normal nextest median | 1.692 s | 1.040 s |
| Per-trial user + system CPU median | 6.41 s | 4.30 s |
| Largest-process RSS median, reported kbytes | 98,852 | 99,096 |
| Executed cases | 32 | 36 |

Baseline trials take 1.706, 1.679 and 1.692 seconds; final trials take 1.040,
1.047 and 1.004 seconds. The median saving is 0.652 seconds, or 38.534%. The
final focused selection adds the new producer unit; its filter matches no baseline
hazard units, leaving the baseline selection unchanged. CPU is summed within each
trial before taking the median. The RSS observation is effectively unchanged and
measures the largest process, not total concurrent memory.

Measurements run sequentially on local Linux ARM64 with nightly `2026-09-30`,
disabled incremental compilation, two build jobs, four CI-profile workers,
prebuilt nextest metadata and an explicit prebuilt driver. The effective Cargo unit
graphs match across all 271 units; workspace runtime/test profiles remain opt0,
with assertions, overflow checks and unwind behavior unchanged. Tracked/unignored
source inputs and executable manifests remain frozen within each phase. Each
manifest contains 46 paths, including all 22 test binaries and the prebuilt CLI/driver.
Build preparation uses existing caches and is excluded from normal runner times.
These results describe warm execution on this host.

### Coverage and style validation

The unchanged coverage command passes all 3,732 final cases and four doctest
targets with zero doctests each. Its 319.663-second collector duration is separate
from normal timing. Final compiled selections, focused execution and full coverage
JUnit identities agree after the explicit five-retired/nine-added mapping. Inputs
remain frozen throughout collection. The historical coverage baseline differs
from the current control only in this investigation document across its 2,315
input hashes; all implementation bytes match that control.

Both reports contain exactly the same 1,153 source files and 166,494 unique
`SF` + `DA` line keys, with exactly the same 154,251 covered source lines
(92.64658%). No previously covered source line loses a hit. The first candidate's
single lost constant-multiplication hit is restored by the producer unit.

The raw LCOV summary is a separate metric: `LF` remains 169,594, `FNF` 18,428 and
`FNH` 17,152, while `LH` changes from 156,365 to 156,364. This entire difference is
in `producer/hazards.rs`: exported `LH` is 343 versus 342, while all 350 exported
`DA` keys and all 340 covered `DA` lines in that file are identical. Raw summary
coverage is 92.19961% versus 92.19902%; it is not claimed unchanged. Execution
counts and generated function-symbol inventories also differ. No branch-coverage
claim is made.

The unchanged collector excludes test directories and `tests.rs` files equally
before and after. JUnit establishes that the moved units execute; LCOV does not
establish instrumentation or line coverage for those omitted test helpers.

Final formatting, whitespace checks and workspace/all-target Clippy with warnings
denied pass. The supplementary CLI and PDG audits observe all 28 changed/new
functions with zero findings, including the new producer case. These broad audits
remain incomplete and nonclean overall: exit two, with 6,483 functions and 3,295
findings for CLI, and 17,008 functions and 1,424 findings for PDG. No claim is made
that every unrelated broad finding has a proven baseline comparison.

The actual production CI numeric and saved-pattern gates pass with exit zero in
`final-style-gates-v1`. Later edits affect exactly three private unit/integration
test files. `ci-gate-provenance.json` verifies that bridge, including unchanged
production inputs, scripts, configuration and private `cfg(test)` module wiring.
The retained gates are distinct from the current formatting, Clippy and scoped
all-target checks in `post-style-gates-v2`. Max's manual review of changed-test
patterns and complexity is also separate from the automated production gate.
No threshold, suppression or saved-ignore change is used.

Raw JUnits, source/binary fingerprints, resource logs, case mappings, coverage and
review evidence are retained under `target/compiler-unit-boundaries-20261101`.

## 12. Stored-slices CPU profile

The stored-slices integration test still spends substantial time rebuilding and
serializing evidence for its assertions. Low collects the controls, diagnostics
and CPU stacks; Max independently reviews the controls, process participation,
sample accounting and interpretation. This experiment profiles the implementation
at `b6dd64d2a4c060c974af15afbdef90a7f0303a85`; it introduces no optimization.

The exact case is
`variant_capture::input_owner::borrowed::stored_slices_compose_with_checked_consumers_saved_receivers_and_finite_alternatives`,
in `crates/cargo-rstyle/tests/variant_capture/input_owner/borrowed.rs`.
All six contexts execute: aarch64 Linux, x86_64 Linux and aarch64 Darwin, each with
std and no_std. Every capture reports 52 functions, 17 admitted functions and 136
candidate pairs, with all four completion flags true. The six full JSON reports
total approximately 20 MB.

### Controls and resources

The prebuilt test executable runs this case alone, with `--exact
--test-threads=1 --nocapture`, the real prebuilt CLI/driver and the pinned nightly
`2026-09-30`. The existing opt0 profiles, assertions and overflow checks remain
unchanged. Preparation/build time is excluded. All successful runs pass, with
identical source inputs and hashes for the test executable, CLI and driver.

| Execution condition | Libtest seconds | Command wall median |
| --- | --- | ---: |
| Ordinary PATH, three controls | 11.78, 11.67, 11.72 | 11.725 s |
| Pinned toolchain bin on PATH, three controls | 11.51, 11.62, 11.43 | 11.516 s |
| CPU profiler, matching pinned PATH | 12.58 | 12.598 s, one run |

Ordinary controls use 10.21–10.28 seconds of user CPU and 1.45–1.51 seconds of
system CPU. Maximum single-process RSS is approximately 140 MiB. The external
100 ms process-tree sampler observes peak PSS of 272–274 MiB; this is a sampled
concurrent-memory estimate. It misses short-lived processes and can perturb
scheduling. Allocation counts, allocation sizes and retained-object ownership
are unmeasured.

Kernel perf is unavailable on this host. The successful profile uses gperftools
at 100 Hz with libgcc unwinding, inherited by child processes, with PERF telemetry
off and the real CLI invoked directly. Prepending the exact pinned-toolchain bin
directory avoids Rustup launcher execs; the resolved Cargo/rustc hashes and
versions match the ordinary controls. Profiler wall overhead is 9.39% against
the matching pinned-PATH median. Initial profiler attempts through Rustup fail
because inherited SIGPROF interrupts rustc during exec startup; those failed
attempts and an accidentally instrumented control set are retained and excluded.

There are 1,019 workload samples across 202 profiled workload process images.
Three additional auxiliary images are retained and excluded, including two
samples from the outer Python wrapper. All workload stacks resolve
syntactically, and 1,015 contain multiple frames. Exported native-library aliases
remain unreliable, so conclusions use resolved Rust ancestors. These are CPU-time
samples with user-space stacks: ITIMER_PROF includes system CPU charges, with no
kernel-stack attribution. Sample ticks account for about 81% of GNU time CPU and
must not be converted directly into exact seconds or predicted savings.

### Where the samples are

| Process role | Samples | Share of workload |
| --- | ---: | ---: |
| Test harness | 563 | 55.25% |
| CLI | 225 | 22.08% |
| Compiler driver | 225 | 22.08% |
| Cargo | 6 | 0.59% |

The compiler executes inside the driver. Of its 225 samples, 211 have the rstyle
capture callback as an ancestor. Separate rustc probes have zero samples at this
frequency; their startup cost remains unquantified. Harness samples include real
production verification code called by the assertions.

The following test-operation branches partition the harness samples. Percentages
use all 1,019 workload samples, so these rows sum to the harness share.

| Harness operation | Samples | Share of workload |
| --- | ---: | ---: |
| `word_classifiers::verify_admissions` | 186 | 18.25% |
| Stored-slice mutations, including conditional-read and document edits | 185 | 18.16% |
| Dependency mutations | 97 | 9.52% |
| Passive-context assertions, including old-policy rejection | 31 | 3.04% |
| Location assertions and their wire-bundle inspection | 26 | 2.55% |
| Capture helper, including parsing CLI output in the harness | 26 | 2.55% |
| Remaining harness work | 12 | 1.18% |

Within the mutation branches, `float_kernels::reject_resealed` contains 227
samples, or 22.28% of the workload. Its direct-child partition is 131 samples
serializing `serde_json::Value` to bytes, 85 importing/verifying those bytes, ten
dropping the JSON value and one checksumming. The surrounding mutation preparation
also calls `shared_evidence::wire`, which reads and parses the entire bundle and
expands its repeated censuses anew for each mutation. This helper has 77 inclusive
samples across mutation and location checks.

Actual serde_json implementation frames occur in 555 samples, or 54.47% of the
workload: 348 harness, 105 CLI and 102 driver. This union excludes generic type
parameters that merely mention serde_json. JSON string-escaping leaf frames
account for 78 samples, or 7.65%. Report writing contains 64 inclusive CLI samples;
bundle writing contains 68 inclusive driver samples. These function scopes and
the test-operation scopes overlap and must not be added.

Shared production functions also remain visible: `eligibility::classify` has 213
inclusive samples across the harness, CLI and driver; ordinary data-frontend
checking has 143; strict `producer::verify::expected_pair` has 89, all in the
harness. The last function lowers, analyzes and emits the expected graph again.
These counts describe sampled ancestry, not invocation frequency or self time.

### Confirmed repetition and next experiment

A separate diagnostic run takes 12.10 seconds and uses an artifact-preserving
CLI relay. Its 13,018 canonical events form 6,509 complete observation pairs,
keyed by PID and observation ID. Events are parsed once from command stderr;
copies in archived capture logs are excluded.

The trace confirms six compiler captures, 26 bundle imports and 520 producer
verification entries. Six imports belong to generated CLI handoff. The harness
performs six clean strict imports plus fourteen edited-bundle strict imports.
`verify_admissions` already loads once per context and shares its store/analysis
across the admitted functions; it then explicitly verifies documents and admits
each of those functions again. The repetition is the six-context replay and
the edited-bundle imports, rather than a clean bundle load per function.
Of the producer checks, 240 use generated policy and 280 use strict harness
policy. Fifty-six producer failures are expected rejection outcomes. Data-frontend
stages include expected eligibility probes and rejections as well.

Selected inclusive stage totals are 3.070 seconds for CLI compiler capture,
1.444 seconds for CLI handoff, 0.239 seconds for matching and 0.636 seconds for
report writing. Harness bundle decode totals 0.905 seconds, bundle verification
1.039 seconds and producer verification 1.240 seconds. These durations overlap.
The events omit thread IDs and absolute timestamps, preventing general exclusive
wall-time reconstruction. Relay/copying overhead belongs to the diagnostic run.

The next experiment should reduce the test's repeated evidence work:

1. Prepare the mutation baseline once. Move independently expressible semantic
   cases, such as the eight stored-summary edits, into private tests beside their
   actual strict verifier, using small typed inputs. Retain a correctly resealed
   live bundle rejection proving that import calls that verifier. Keep the five
   dependency provenance/Copy/owner mutations and the checked-element
   conditional-read rejection integrated unless their distinct proof boundaries
   are independently preserved in owning unit tests. Explicitly map every case.
2. Keep compiler capture, admission outcomes, matching, strict bundle import and
   a representative independent strict admission in every target/std context.
   Consider replaying the exhaustive per-function strict admission matrix in one
   suitable context only after mapping equivalent authority for its assertions.
   Reuse the loaded store and analysis while unchanged.
3. Retain focused real serialization/import checks. Avoid serializing and importing
   the entire 52-function unit for each isolated helper-summary assertion when
   its actual verifier can be exercised directly.

This recommendation targets measured test orchestration. Shared production JSON
encoding, decoding and frontend classification are subsequent optimization
candidates. The profile does not establish the savings from either change or
justify attributing all harness time to removable work. A candidate needs normal
before/after controls and coverage comparison before any speedup claim.

All evidence is local under `target/stored-slices-profile-20261101`: the complete
interactive `stored-slices-cpu.svg`, role-specific SVGs in `focused/`, raw per-PID
profiles, folded stacks, `root-hotspots.json`, `root-stage-summary.json`, controls,
fingerprints, execution/syscall traces and independent review artifacts. These
large ignored artifacts are retained locally. Only this findings document changes
after the frozen profiling runs; whitespace validation applies to that document.
