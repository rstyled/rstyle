# Evaluation tools and recorded studies

`eval/` contains manual evaluation tools, study corpora, labels and recorded
results. Normal compiler analysis does not run these studies. CI runs Rust
regression tests; it does not automatically rerun the Python evaluation studies.

Active shared regression inputs live in
[`tests/fixtures/`](../tests/fixtures/README.md). Function-family sources,
selections and companion oracles moved there from `eval/coverage-development/`;
advisory case specifications moved from `eval/clones-capability/`. Historical
receipts keep their original paths, revisions, timings and metric versions.

| Location | Current purpose |
| --- | --- |
| [`clones-capability/`](clones-capability/README.md) | Compiler report evaluation tools, authored labels, and historical capability/comparison receipts. `compiler_*.py` tools consume saved compiler JSON reports; comparison readers can reevaluate saved reference/compiler inputs. The retired reference engine is not required. |
| [`hir-coverage/`](hir-coverage/README.md) | Frozen extraction inventory, provenance and historical coverage measurements. |
| [`ccgraph-v1/`](ccgraph-v1/README.md), [`ccgraph-v2/`](ccgraph-v2/README.md) | Preregistered matching studies with fixed corpora, expectations and recorded results. Their original protocols are historical, not current CI gates. |
| `loo/`, `loo_norm_*/`, `normalize/`, `align/` | Recorded threshold, normalization and alignment experiments; see [`scripts/`](../scripts/README.md) for tools and input requirements. Historical scores are not measurements of the current compiler default. |
| `codenet/` (gitignored) | Locally generated CodeNet samples and results. |

Two study corpora also supply regression source snippets:
`clones-capability/corpus/src/` for advisory guard tests, and
`ccgraph-v2/corpus/{lz4,time}/sample/src/lib.rs` for HIR extraction tests. They
remain with their study manifests and provenance so there is one source copy.

Run the saved-report evaluation tool tests independently:

```sh
python3 -m unittest discover -s eval/clones-capability -p 'test_*.py'
```

See each study README before reproducing an experiment. Old command lines may
describe removed interfaces or require retained artifacts; a recorded result is
evidence for its stated revision, not a claim that its original engine remains
runnable. Do not rewrite recorded JSON/CSV to reflect current behavior.
