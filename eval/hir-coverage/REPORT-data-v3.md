# Data-v3 natural coverage refresh

The unchanged frozen 60-context selection was assessed under Data-v3 on
2026-10-19, after the capture repairs in `68aa095`. Compiler, sources, dependency
locks and default Cargo settings match the historical inventory. No detection
predictions were evaluated; `--max-pairs 0` deliberately leaves pair enumeration
incomplete when pairs exist. Historical reports and selection remain unchanged.

| Profile | Selected | Assessed | Admitted | Excluded | Unknown | Failed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| R0 (reproduced before final capture repairs) | 60 | 53 | 0 | 53 | 7 | 0 |
| Expanded (reproduced before final capture repairs) | 60 | 53 | 5 | 48 | 7 | 0 |
| Data-v3, extraction 3.0.0 | 60 | 53 | 8 | 45 | 7 | 0 |

Successful Data-v3 evidence is `target/hir-coverage/run-25mgywk_/` (started at
`15684a9` with the capture repairs in the working tree). All three packages
complete; 18 admitted CFG/PDG documents pass exact sealed-bundle and schema
checks. The complete generated `hir-data-v3.json` retains every selected source
identity/context and every reported exclusion reason/source blocker combination;
`summary.json` retains their combinations. Seven uncaptured contexts remain
unknown; none is independently proven inactive. There is no independently frozen
expected-in-scope denominator and no quality/calibration claim.

Admitted selected functions:

- `lz4_flex`: `token_from_literal`, `token_from_literal_and_match_length`,
  `get_maximum_output_size`.
- `time-core`: `cold_path`, `likely`, `unlikely`, `is_leap_year`, `days_in_year`.
- `regex-syntax`: none.

Relative to expanded, Data-v3 adds `likely`, `unlikely`, and `days_in_year`;
there are no lost selected admissions. R0/expanded comparison evidence is
`target/hir-coverage/run-94t0821v/`; that run's Data stage failed and is not used
as a successful Data observation. Earlier failed attempts remain incomplete
execution, including array-budget, receiver/source, and declaration-range errors.

Reproduce with `uv run eval/hir-coverage/reproduce.py`; use `--profiles hir-data-v3`
for a focused assessment. Future feature changes must rerun this selection and
record gains from measured admissions, rather than source-only predictions.

## Scalar OR-pattern increment, extraction 3.1.0

Full fresh run `target/hir-coverage/run-wjnhsac7/`, based on `66bf87c` plus the
OR-pattern implementation, reproduces R0 0/53/7 and expanded 5/48/7
(admitted/excluded/unknown). Data-v3 measures **9 admitted, 44 excluded, 7 unknown**
out of the same 60 contexts; 53 are assessed, none failed, none proven inactive.
Twenty admitted CFG/PDG documents pass the unchanged schema checks.
`time-core::weeks_in_year` is newly admitted relative to extraction 3.0.0, and
no selected admission is lost. Other earlier admissions remain the same.
The complete generated assessment and summary retain exclusion combinations and
unknowns. This is feature/coverage evidence, not completed detector pair evaluation
or calibrated quality evidence. The contract remains bounded normal flow.

Final focused rerun `target/hir-coverage/run-4v0dzgj6/` verifies the narrowed
unsupported-pattern capture used by `1f0ef93`: all 60 selected source identities
and outcomes match the full 3.1.0 run (9 admitted, 44 excluded, 7 unknown).
The full final workspace suite passes 714 tests in 27 suites (5 existing ignored
tests); repository-built `cargo rstyle` reports 0 violations across 2,131
functions, and `cargo rstyle patterns --fail-on-candidates` reports 0 remaining
candidates with the 15 existing ignore entries unchanged. No new performance
benchmark is claimed; the implementation avoids per-element array expansion,
duplicated OR arm/guard lowering, and new provenance fields.

## Integer range-pattern increment, extraction 3.2.0

Focused fresh run `target/hir-coverage/run-6s9q_4up/`, started at `5171938` with
the implementation later committed as `849a910`, completes all three packages.
Data-v3 retains **9 admitted, 44 excluded, 7 unknown out of 60**, with 53 assessed,
0 failed and 0 proven inactive. All 60 selected source identities and outcomes
match the 3.1.0 inventory: this slice adds no measured admission in the frozen
selection. Twenty admitted CFG/PDG documents pass the unchanged schema checks.
The full assessment retains exclusion reasons, source blocker combinations and
unknowns. No detector-quality or completed pair-evaluation claim follows.

The workspace suite passes 717 tests across 28 suites (5 existing ignored tests).
The final source-bound consistency regression also passes; Clippy with warnings
denied passes. Repository-built rstyle reports 0 violations across 2,143
functions; patterns reports 0 remaining candidates with existing ignores unchanged.
The range representation remains compact regardless of interval cardinality;
no additional provenance fields or caches were introduced.

## Resolved scalar constant increment, extraction 3.3.0

Focused fresh run `target/hir-coverage/run-qobq35og/`, started at `f393971` with
the scalar constant implementation in the working tree, completes all three
packages. Data-v3 retains **9 admitted, 44 excluded, 7 unknown out of 60**,
with 53 assessed, 0 failed and 0 proven inactive. All 60 selected source
identities and outcomes match the 3.2.0 inventory. This increment adds no measured
admission in the frozen selection. Twenty-two admitted CFG/PDG documents pass
the unchanged schema checks; that document count includes functions outside the
frozen selection and does not change its denominator. The full assessment retains
exclusions, blocker combinations and unknowns; pair evaluation remains incomplete.

The workspace suite passes 720 tests (5 existing ignored tests); Clippy with
warnings denied passes. Repository-built rstyle reports 0 violations across
2,155 functions and patterns reports 0 remaining candidates with all 15 existing
ignores unchanged. Constants reuse the existing resolution record and rustc's
cached evaluator. No formal performance benchmark or detector-quality claim is made.

## Constant-pattern increment, extraction 3.4.0

Focused fresh run `target/hir-coverage/run-nhkwv78q/`, started at `52966e7` with
the constant-pattern implementation in the working tree, completes all three
packages. Data-v3 retains **9 admitted, 44 excluded, 7 unknown out of 60**,
with 53 assessed, 0 failed and 0 proven inactive. All 60 selected identities
and outcomes match 3.3.0; no frozen admission is gained. Twenty-two admitted
CFG/PDG documents pass the unchanged schema checks, including functions outside
the frozen selection. Exclusion combinations and unknowns remain in the complete
assessment. Pair evaluation remains incomplete; no quality claim follows.

The workspace passes 722 tests (5 existing ignores). The final mixed
constant/literal/range fixture and Clippy with warnings denied pass. Repository-built
rstyle reports 0 violations across 2,158 functions; patterns reports 0 remaining
candidates with the same 15 existing ignores. The implementation reuses constant
records and existing selector/guard flow without new provenance fields, caches or
alternative expansion. No formal performance benchmark is claimed.
