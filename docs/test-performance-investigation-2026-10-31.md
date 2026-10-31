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
