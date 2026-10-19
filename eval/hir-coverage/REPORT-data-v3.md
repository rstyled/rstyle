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
