# scripts

Repository scripts run from anywhere (shell drivers generally `cd` to the repo root). `install.sh` also works without a checkout and installs from GitHub. Python is stdlib only. Outputs go to [`eval/`](../eval/README.md) or [`site/wl/data`](../site/wl/README.md). Caches live in `/tmp/rx_*`, `/tmp/cn_*`.

## Everyday

| Script | Usage |
| --- | --- |
| `style.sh` | `scripts/style.sh [args]` builds driver + CLI, runs rstyle on this workspace. Must report 0 violations. `scripts/style.sh patterns --fail-on-candidates` is the CI merge-candidate gate: every candidate must be fixed or dismissed in `rstyle-ignore.toml`. |
| `install.sh` | `sh scripts/install.sh [--force]` installs the pinned nightly and builds both binaries from GitHub without needing a local checkout. Requires rustup and a native build toolchain. |
| `test_install.py` | `python3 scripts/test_install.py` checks installer commands, options, paths, and failures using a mocked rustup; no downloads or compilation. |
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

`ccgraph_followup.py` executes the v2 coverage/correctness study using the shared
real-command runner and independently source-authored bounded oracles.
`test_ccgraph_followup.py` tests its expectations, target denominators and project
splits. `ccgraph_followup_evidence.py` verifies all retained artifact digests and
raw/saved semantic aggregates, then packages the complete canonical scope traces.
See `eval/ccgraph-v2/README.md`; target-check exit 1 means a valid missed primary
coverage target, not a compiler failure.

## Bounded performance investigations

Run build and execution as separate commands with separate budgets. The POSIX
stdlib supervisor streams both output channels, retains their exact bytes and
`run.json`, and cleans up the process group with TERM then KILL. Output directories
must be new; use a different directory for every observation. Timeout exits 124,
interruption exits 130, and completed commands retain their exit status. A timeout
is an incomplete observation, never a passing test. Subprocesses must stay in the
launched process group; deliberately detached processes are outside this supervisor.

```sh
python3 scripts/watchdog.py --timeout 120 --output /tmp/rstyle-build-1 -- cargo test --locked -p rstyle-pdg --test data_types --no-run
python3 scripts/watchdog.py --timeout 60 --output /tmp/rstyle-types-1 -- cargo test --locked -p rstyle-pdg --test data_types
python3 -m unittest discover -s scripts -p test_watchdog.py
```

Use a single sandbox-local `CARGO_TARGET_DIR` and record commit, architecture,
toolchain, build profile, cache state, filter and concurrency with observations.
The supervisor cannot expose stderr that an inner CLI test buffers with
`Command::output`; live child-stage instrumentation remains a separate task.

Set `RSTYLE_PERF=1` for opt-in `rstyle-perf` JSON lines on stderr. Initial
instrumentation covers `capture.binary_hash`, `producer.produce`,
`producer.verify` and `helper.verify`. Starts are emitted before work; finishes
and failures include monotonic elapsed seconds, PID, identity and nesting depth.
Depth describes instrumented calls, not the source dependency-chain length.
Helper visits are counted per thread by SHA-256 of exact encoded resolution
bytes, including the serialized dependency. Names are navigation labels, never
proof identities. These observations do not memoize or bypass any validation.
Timing and counts never enter documents or deterministic reports. Detailed
logging and diagnostic hashing add overhead: compare diagnostic runs separately
from uninstrumented timings. This initial instrumentation does not yet cover all
stages or make buffered child stderr visible in CLI tests.

Diagnostic stderr now streams through the Cargo capture adapter and the CLI test
helper while remaining available in failure output and retained capture logs.
Only the exact `RSTYLE_PERF=1` switch passes through producer-environment clearing;
extraction selection and evidence publication settings are still cleared. Output
from concurrent compiler units may interleave; correlate JSON events by PID and
identity. Add `--nocapture` when running a CLI test to expose its tee output.

Set `RSTYLE_DIAGNOSTIC_FACTS_DIR` to retain version-1, phase-tagged snapshots
before helper certification and, when available, after it but before production.
Files use compiler PIDs and exclusive creation; they are flushed before work
continues. They contain owned facts with original provenance, and explicitly do
not certify a complete source-to-pair capture. Add
`RSTYLE_DIAGNOSTIC_CAPTURE_ONLY=1` to stop after raw HIR/unit evidence. The CLI
reports incomplete capture and exits nonzero because no normal producer artifacts
are published. This is an extraction-only diagnostic outcome, not a passed E2E
analysis. These two diagnostic settings are explicitly forwarded to the driver.

The smallest ordinary-library replay seam is currently a PDG example:

```sh
cargo build --locked --workspace --example helper_replay
RSTYLE_PERF=1 python3 scripts/watchdog.py --timeout 120 --output /tmp/helper-replay-1 -- target/debug/examples/helper_replay /path/to/PID-pre-helper-certification.json
```

Build separately. The executable loads one snapshot, then invokes the same
`helper_calls::capture` used by the driver, without Cargo, rustc or source-file
reads. Output is diagnostic scope and cannot claim source-to-pair completeness.
This is an initial helper-stage seam, not the proposed full evidence-bundle format
or the complete `rstyle-analysis` API. Retain the original source bytes alongside
snapshots for regeneration; snapshots keep original provenance paths unchanged.

Normal current-run driver/CLI production now uses the `trusted-generated`
verification policy, retaining structural validation and avoiding duplicate graph
reconstruction and recursive certified-summary derivation. Independent library
calls and replay remain strict by default. To measure the normal production
policy on one's own raw snapshot, add `--generated` to `helper_replay`. The
reported scope remains diagnostic; choosing this policy does not certify capture
completeness. Stage events and the CLI report declare the selected policy.

### Complete clones and strict-import profiling

`python3 scripts/profile_clones.py --profile debug` (or `release`) profiles
minimal and complete read/inherent helper fixtures using already built tools.
Build the workspace and `rstyle-pdg`'s `unit_import_replay` example in that profile
first. The script retains reports, timing and diagnostic logs in a new temporary
directory, checks completeness and pair decisions, and separates ordinary
samples from `RSTYLE_PERF` observations. Commands and measured limits are in
[the production profile](../docs/performance/production-profile.md).
