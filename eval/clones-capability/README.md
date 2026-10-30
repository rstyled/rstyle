# Compiler capability evaluations

These tools evaluate completed compiler reports against independent authored
labels. They do not run a second analysis engine or prove whole-project accuracy
from a small sample. Context mismatches, missing observations and unavailable
measurements remain explicit.

| Tool | Labels | Checks |
| --- | --- | --- |
| `compiler_quality.py` | `rstyle.compiler-quality-cases/1` | Source diagnostics, mutation, density, suppression and coverage against authored expectations. |
| `compiler_complexity.py` | `rstyle.compiler-complexity-cases/1` | Complexity findings, scores and rankings. `--require-scores` requires numeric scores. |
| `compiler_planning.py` | `rstyle.compiler-parameter-plan-cases/1` | Parameter-plan steps, bundles and original owner relationships. |
| `compiler_proposal_quality.py` | `rstyle.compiler-proposal-quality-cases/1` | Proposal kinds, source roles, quality judgments and usefulness boundaries. |

Each command accepts explicit saved inputs and creates a new output file:

```sh
python3 eval/clones-capability/compiler_quality.py \
  --report /path/to/compiler-report.json \
  --labels eval/clones-capability/project-default-density-reviewed-cases.json \
  --output /path/to/new-evaluation.json
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
```

Use `--help` for each tool's arguments. Labels and policies describe their stated
source/build context; check that context before reusing a label set. Case files
for manual compiler acceptance checks are retained, alongside the target-context
corpus. They are inputs, not captured results or claims about the current revision.

`evaluation_checks.py` contains shared input, diagnostic and label validators.
Old-engine comparison runners, comparison-only tests, old project reducers,
completed study reports and performance receipts have been removed. Their
history remains in Git; there is no active migration comparison workflow.

Regression specifications live in [`tests/fixtures/advisory/`](../../tests/fixtures/advisory/).
Guard tests own their source copies under `crates/cargo-rstyle/tests/guards/fixtures/`.
Neither reads from this evaluation directory.
