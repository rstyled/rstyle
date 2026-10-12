# scripts

All run from anywhere (they `cd` to the repo root). Python is stdlib only. Outputs go to [`eval/`](../eval/README.md) or [`site/wl/data`](../site/wl/README.md). Caches live in `/tmp/rx_*`, `/tmp/cn_*`.

## Everyday

| Script | Usage |
| --- | --- |
| `style.sh` | `scripts/style.sh [args]` builds driver + CLI, runs rstyle on this workspace. Must report 0 violations. `scripts/style.sh patterns --fail-on-candidates` is the CI merge-candidate gate: every candidate must be fixed or dismissed in `rstyle-ignore.toml`. |
| `install-hooks.sh` | Run once per clone. Sets `core.hooksPath=.githooks` so `pre-commit` runs `cargo fmt --check`, clippy (`-D warnings`) and rstyle. Skips when no Rust files are staged; bypass with `--no-verify`. |
| `test.sh` | `scripts/test.sh [cargo test args]` runs `cargo test --workspace --locked` against a shared `CARGO_TARGET_DIR` (default `~/.cache/rstyle/target`) so worktrees don't rebuild dependencies. |
| `test_codenet.py` | `python3 scripts/test_codenet.py` tests `codenet_sample.py` on a fake package (needs cargo). |
| `wl_site.sh` | `scripts/wl_site.sh [--serve]` regenerates `site/wl/data/*.json`; `--serve` hosts it on `:8765`. |

## Labeled-example evaluation (`rust_refactoring_examples/`)

| Script | What it does |
| --- | --- |
| `eval_examples.sh [thr...]` | Prints which functions cluster at each threshold (default `800 600 500 400 300`). |
| `loo_eval.py [--refresh] [--normalize RULES]` | Leave-one-project-out grid -> `eval/loo*/`. Needs real-crate clones in `/tmp/real` for the noise set. |
| `normalize_ablation.py [config...]` | Per-rule canonicalizer ablation -> `eval/normalize/`. |
| `norm_pairs.sh [rules]` | Raw vs canonical WL / coverage for the labeled pairs (needs a prior `loo_eval.py` or `wl_site.sh` run). |
| `align_ablation.sh` / `.py` | Alignment variants -> `eval/align/*.csv`. |
| `pair_cov_eval.py` | `--pair-coverage` selector test -> `eval/align/pair_cov.csv`. |

## CodeNet (held-out strangers' code)

Input: the `rust_package/` folder (gitignored). Build it first with `recreate_rust_package.py` (below). Protocol: tune on `dev`, run `test` once per final config, never touch `holdout` (needs `--emit-holdout` to create and `--confirm-holdout` to evaluate). Splits are hashed by identical-problem cluster, so they are stable across seeds.

```sh
scripts/recreate_rust_package.py                               # once: -> rust_package/ (+ Project_CodeNet/ intermediates)
scripts/codenet_sample.py --package rust_package --seed 1      # -> eval/codenet/s1/{shards,pairs.csv,manifest.json}
scripts/codenet_eval.py --out eval/codenet/s1 --configs none,all,all+pc650
scripts/codenet_aggregate.py --seeds 1,2,3 --baseline none     # mean/sd over seeds
```

| Script | What it does |
| --- | --- |
| `recreate_rust_package.py` | Builds `rust_package/` from [IBM Project CodeNet](https://github.com/IBM/Project_CodeNet) (Apache-2.0): fetch, extract Rust only, build training files. Run from the repo root. Streams the 8.3 GB tarball with `curl` and keeps only the Rust subset (~2.6 GB code + 1.2 GB metadata). Expect 127210 Rust files, 2139 problems. Flags: `--tar PATH` use or download to a local tarball, `--keep-tar`, `--fetch-only`, `--skip-extract` (reuse `Project_CodeNet/`), `--skip-build` (extract only). |
| `sample_size.py` | Sample size for a proportion at 95/99% (`--population`, `--p`, `--margins`). |
| `codenet_sample.py` | Deterministic labeled sample into cargo crates; tiers `resubmit`, `same_problem`, `identical_problem`, `cross_problem`. |
| `codenet_eval.py` | Runs the miner per shard, scores per tier -> `results_<split>.csv`. Config names: `none`, `all`, `all-<rule>`, rule list, modifiers `+pc<N>`, `+h<N>`, `+guard`. |
| `codenet_score.py` | Library: function-level pairing and IO-helper filtering. |
| `codenet_aggregate.py` | Seed aggregation; `--baseline` adds `delta` and `beyond` (outside seed spread). |
| `codenet_flips.py --a X --b Y` | Paired flag flips between two configs + sign test. |
| `codenet_sep.py` | AUC raw vs canonical for edited copies vs unrelated functions. |
| `codenet_drops.py` | Pairs where canonicalization adds/removes the most unmatched nodes (rule-defect hunting). |

Canonicalizer rules: `ctor pred case exit counter cmp try`.

## Strict CCGraph study

`ccgraph_study.py` validates and executes the frozen independently source-labeled
bounded study under `eval/ccgraph-v1`. It invokes the real `cargo rstyle clones`
command with fresh compiler capture; it never consumes legacy miner predictions.
`test_ccgraph_study.py` independently tests bookkeeping/failure expectations.
`ccgraph_study_evidence.py` packages representative traces and inventories from
preserved validated reports: run it after `ccgraph_study.py summarize`.
See `eval/ccgraph-v1/README.md` for exact commands, exit behavior and retention.
