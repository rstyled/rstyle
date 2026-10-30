# Current evaluations

This directory contains tools and authored inputs for evaluating the current
compiler path. Rust regression tests own their fixtures independently under
`crates/*/tests/` and [`tests/fixtures/`](../tests/fixtures/README.md).

| Location | Purpose |
| --- | --- |
| [`clones-capability/`](clones-capability/README.md) | Evaluate saved compiler reports against independent labels for findings, density, complexity, parameter plans and abstraction proposals. |
| [`hir-coverage/`](hir-coverage/README.md) | Reproduce the fixed extraction-support inventory, preserving source provenance, compilation contexts and missing-data distinctions. |
| `codenet/` (gitignored) | Locally generated CodeNet samples and evaluation outputs. See [`scripts/`](../scripts/README.md). |

Completed CCGraph studies, old threshold/normalization/alignment results,
old-engine comparisons and recorded migration/performance receipts have been
removed. They remain available in Git history and the historical branch.
They are not current checks or extra production analysis stages.

Run the compiler report evaluation tests with:

```sh
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
```

`coverage-development/` is only a pointer to the relocated function-family
fixtures; local untracked experiments are left untouched. CI runs Rust regression
tests and compiler dog-food checks, not these manual evaluation studies.
