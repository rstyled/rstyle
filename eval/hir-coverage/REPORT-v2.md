# Ownership-corrected eligibility inventory v2

Assessment version 2 corrects source ownership without changing the frozen
60-definition selection or extraction semantics. The historical [v1 report](REPORT.md)
remains unchanged. Implementation baseline is
`9ba773fec4b5dea4381974eaf1c20b8e58b623fd`. The successful fresh reproduction was
run from `cf6541e` with the ownership changes in the working tree on
`ccgraph/compositional-data-coverage`, using the pinned aarch64 compiler.

| Selection | All frozen selected | Active selected contexts | Captured records | Assessed selected contexts | Admitted | Excluded | Unknown | Failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| R0 | 60 | 53 | 53 | 53 | 0 | 53 | 7 | 0 |
| hir-expanded-v1 | 60 | 53 | 53 | 53 | 5 | 48 | 7 | 0 |

There are 60 selected function/build contexts in each profile: one actual default
library build per selected package. No selected definition was dropped. Seven
definitions remain unknown: absence from captured HIR does not prove inactivity.
Zero contexts have independently established cfg-inactive status. No independent
expected-in-scope denominator exists for this historical inventory; the report
serializes it as null rather than inventing a classification.

The outer `count_same_bytes` now maps only to its own exact function; its nested
`count_same_bytes_tail` maps separately. The former changes from failed ambiguous
correspondence to excluded. No function becomes admitted through this repair;
there are zero new or lost admissions relative to v1. Complete source bytes,
SHA-256, original file span, identifier, package root and compilation context must
agree. Duplicate records within a context fail; multiple selected contexts are
assessed separately and do not collapse to the first record. Source-level admission
requires admission in every selected context.

Generated successful evidence: `target/hir-coverage/run-m3cko26n/`. Six fresh real
Cargo captures completed and 12 admitted CFG/PDG documents passed exact manifest
and unchanged pinned-schema checks. The inventory command deliberately uses
`--max-pairs 0`; pair enumeration/evaluation is incomplete where pairs exist. This
is eligibility evidence, not a completed source-to-pair detector execution or a
detection-quality result.

The previous attempt at `target/hir-coverage/run-nsd2xf3u/` failed during Cargo
planning when a concurrent tool rebuild replaced the executing orchestrator.
Its compiler failure and partial outputs remain retained. It is not counted as
exclusion or successful empty execution. The successful rerun had no competing
executable rebuilds.

Three retained baseline inventories (`run-m1ksmpyb`, `run-f1rlx74s`,
`run-hop5pn8m`) reproduce the historical 5/60 expanded result. A search of retained
JSON reports under target found no report with 50 functions or six admissions.
The user-observed 6/50 run is therefore unidentified and is not substituted for,
or overwritten by, either inventory version.

Reproduce with `uv run eval/hir-coverage/reproduce.py`. The current assessor emits
`assessment_version: 2`, context outcomes and explicit denominator fields. Its
source-only syntax blocker inventory remains diagnostic; counts are not summed
as predicted coverage gains. New capability acceptance targets and the broader
applicability inventory remain separate work under the compositional data contract.
