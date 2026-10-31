# Test capture consolidation, 2026-10-31

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

## Matched timing

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

## Coverage and checks

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

## Retained local evidence

[target/test-reduction-20261031](../target/test-reduction-20261031/) contains the
baseline LCOV and JUnit, matched selection and measurement script, three timing
reports per version, frozen source hashes, complete coverage comparison and
validation logs. `target/` is ignored by Git; this document preserves the method
and results independently of those local artifacts.
