# CI performance investigation, 2026-10-31

Investigated commit `cdae40c3c2126013a2695ef092ffeba0728b9715`, including the three jobs in [run 41141575742](https://github.com/rstyled/rstyle/actions/runs/41141575742). Evidence collection used the requested Low agent, followed by Max review of the measurements, source paths and conclusions. The final section records the subsequent Rust and CI implementation using the same Low implementation / Max review workflow.

The strongest findings are repeated source-position scans in capture, unnecessary expansion of shared evidence in full reports, large JSON retained inside integration tests, repeated workspace capture between root style and patterns, and dependency feature variants during compilation. These are separate costs and need separate fixes.

## Evidence and limits

Downloaded and evaluated all three JUnit artifacts and all three self-check reports. Each platform passed the same 3,776 tests. Full GitHub console logs could not be downloaded with the unauthenticated CLI; the public artifacts were accessible through `nightly.link`. The old artifacts contain elapsed test times, but no CPU or memory samples.

Fresh measurements used Linux ARM64, the pinned nightly, current-source debug binaries, `CARGO_INCREMENTAL=0`, and two Cargo build jobs. The container exposes 20 host CPUs but has an **8 CPU quota and 16 GiB memory limit**. CPU-heavy experiments ran sequentially. These measurements establish local mechanisms; they do not establish fresh x64 or macOS resource usage.

Raw evidence is retained locally in:

- [Downloaded CI data and baseline](../target/perf-investigation/41141575742/)
- [62 owner workload measurements, including exact commands](../target/perf-investigation/live-20261031/owner-measurements.csv)
- [Owner analysis and semantic comparisons](../target/perf-investigation/live-20261031/OWNER-FINDINGS.md)
- [Nextest concurrency measurements](../target/perf-investigation/live-20261031/nextest-results.json)
- [Complete local investigation directory](../target/perf-investigation/live-20261031/)

`target/` is ignored by Git; this report preserves the conclusions independently of those local files. Each sampled command retains stdout, stderr, watchdog status, GNU time, `/proc` samples and shared cgroup snapshots. MB/GB below are decimal. Sampled process-tree PSS is a lower-bound peak observation; GNU maximum RSS is the largest individual process, not a simultaneous tree total. Sampling misses short processes and runs outside the command tree. Shared cgroup counters include unrelated container activity. CPU and allocation call-stack profiles were not collected; attribution uses stage timings and controlled workloads.

## What the original CI run shows

| Platform | Whole job | Before nextest execution in test step | Nextest execution | Root self-check | Patterns |
| --- | ---: | ---: | ---: | ---: | ---: |
| Linux ARM64 | 979 s | 232.9 s | 357.5 s | 163 s | 95 s |
| Linux x64 | 1,050 s | 214.8 s | 482.6 s | 147 s | 78 s |
| macOS ARM64 | 2,110 s | 429.8 s | 1,072.7 s | 239 s | 121 s |

The 1,187 CLI integration tests account for **95.6–95.9% of aggregate test elapsed time**. The slowest three tests account for only 7–9%: fixing only those tests cannot explain or remove most suite cost. Four nextest slots are occupied about 99% of elapsed execution; this measures scheduling occupancy, not CPU utilization.

macOS is disproportionately slower on short CLI tests: tests taking 0.25–0.5 s on x64 have a median macOS/x64 ratio of 3.19, versus 1.47 for tests above 5 s. Process/tool startup is a plausible contributor, but the old artifacts cannot distinguish it from runner hardware, filesystem or security overhead. A minimal local cached capture still costs 0.209–0.230 s across three warm runs.

## 1. Capture contains material source-position-dependent work

Fresh sparse advisory fixtures contain N functions, one late five-parameter diagnostic, and the existing parameter policy. They use the same capture flags as the demanded-owner regression, without changing the 4,096 payload budget.

| Owners | Fresh live export | Full report | Compact report |
| ---: | ---: | ---: | ---: |
| 128 | 0.573 s | 0.900 MB | 0.181 MB |
| 256 | 0.520 s | 1.765 MB | 0.331 MB |
| 512 | 0.695 s | 3.496 MB | 0.631 MB |
| 1,024 | 1.268 s | 6.969 MB | 1.237 MB |
| 2,048 | 2.826 s | 13.924 MB | 2.455 MB |
| 4,101 | 7.544 s | 27.866 MB | 4.915 MB |

Separate fresh diagnostic captures localize the growth:

| Stage | 1,024 owners | 4,101 owners |
| --- | ---: | ---: |
| `capture.compile` | 0.751 s | 5.966 s |
| `cli.handoff` | 0.139 s | 0.445 s |
| `cli.report_write` | 0.205 s | 0.819 s |

At 4,101 owners, compiling/capturing occupies about 79% of command wall time. Plain Cargo check on the same sources with fresh targets takes only 0.079/0.126 s at 1,024/4,101 owners. The capture slowdown is therefore much larger than ordinary compilation of these fixtures.

A stronger control keeps **1,024 identical functions and the same total source bytes**, moving 4,096 blank lines from after the functions to before them:

| Control | Padding after definitions | Padding before definitions |
| --- | ---: | ---: |
| Capture/compile stage | 0.751 s | 2.093 s |
| Complete capture command | 1.249 s | 2.591 s |
| Plain Cargo check | 0.063 s | 0.078 s |
| GNU maximum individual RSS | 116,512 KiB | 116,632 KiB |

Moving the same padding adds **1.342 s of capture work** with essentially unchanged memory, versus 0.014 s in plain compilation. This strongly supports repeated source-prefix scans as a real hotspot.

The source matches the experiment: [Definitions::add](../crates/rstyle-pdg/src/source_evidence.rs) converts six line/column positions per definition; legacy `byte_offset` scans preceding lines from the beginning on each call. Existing `source_evidence::positions::Positions` already indexes source positions, including tested Unicode/CRLF behavior. Other unpartitioned quadratic candidates are [find_source_scan and inventory_definition](../crates/rstyle-driver/src/hir_extraction/mod.rs): each owner searches a scan list, and each source definition filters all function facts.

This does not attribute all 5.966 s to one function or predict exact savings from an implementation change. Original-buffer restoration occurs once in this fixture and takes about 0.00003 s; it is not the measured bottleneck.

## 2. Full reports repeat shared source evidence

Sparse `fn_params` advisory capture omits the unit census and grows roughly linearly. `facts.source.bytes` contains the individual function, not the entire file. This disproves the initial broader hypothesis that every full report necessarily duplicates the whole source per owner.

Adding `--source-rules` retains the source census while still disabling clone matching. Both full and compact then decode full facts, so this is a cleaner serialization control:

| Owners | Full output | Compact output | Repeated census payload in full report |
| ---: | ---: | ---: | ---: |
| 128 | 1.374 MB | 0.268 MB | 0.383 MB |
| 256 | 3.271 MB | 0.505 MB | 1.324 MB |
| 512 | 8.736 MB | 0.979 MB | 4.876 MB |

At 512 owners, full/compact decode times are 0.0419/0.0417 s and handoff times 0.0563/0.0561 s, while report writing is **0.2498/0.0305 s, an 8.18× difference**. Both outputs preserve the same advisory evaluation and scope; matching is not requested and accepted pair count is zero.

The repeated raw source component grows 0.253 → 1.030 → 4.157 MB as owners double. Full serialization in [report/functions.rs](../crates/cargo-rstyle/src/clones/report/functions.rs) expands each function; [frontend/options.rs](../crates/rstyle-pdg/src/frontend/options.rs) renders its shared census again. With N owners and source size S this adds O(N×S) output/work; S grows with N in these fixtures. The raw unit transport already shares evidence.

Sparse full/compact replay medians are 0.379/0.192 s at 1,024 owners and 1.471/0.741 s at 4,101 owners, across three paired runs. Sparse compact also selects header-only decoding, so those gains are not exclusively serialization. Full text output barely changes replay time (0.370/1.416 s): the full archive is still written first. [Report::publish](../crates/cargo-rstyle/src/clones/report.rs) then loads the whole archive into a String for JSON stdout.

## 3. Pair enumeration is another, distinct quadratic cost

Ordinary clones on the same empty-function fixtures produces many identical-body matches:

| Owners | Output | Command wall | Sampled tree peak PSS |
| ---: | ---: | ---: | ---: |
| 128 | 14.545 MB | 1.649 s | 0.148 GB |
| 256 | 55.453 MB | 5.456 s | 0.381 GB |
| 512 | 216.414 MB | 25.969 s | 1.296 GB |

At 128 owners there are 8,128 evaluated and 8,001 accepted pairs; at 256, 32,640/32,385. Detailed matching and accepted-pair sections dominate output. Compact function records preserve these pairs and therefore barely improve ordinary replay: 5.121 → 4.986 s at 256 owners.

This is a deliberate worst-case control, not proof that pair enumeration dominates the CI suite. It establishes that report census deduplication alone will not bound clone-analysis resource usage. Any future pair budget or summary representation must preserve explicit completeness information.

## 4. Integration-test JSON dominates heavy-test memory

Two representative groups were executed once each at one and four nextest workers, with prebuilt binaries and JUnit retained. All 22 test executions passed.

| Group | Workers | Nextest execution | Command CPU | Sampled tree peak PSS |
| --- | ---: | ---: | ---: | ---: |
| Six small root baseline tests | 1 | 2.542 s | 2.84 s | 0.122 GB |
| Six small root baseline tests | 4 | 0.661 s | 2.75 s | 0.142 GB |
| Five heavy owner/clone tests | 1 | 80.205 s | 79.34 s | 1.411 GB |
| Five heavy owner/clone tests | 4 | 40.148 s | 79.60 s | 2.070 GB |

The heavy group contains both demanded-owner tests, retained-owner scope, and the two slow clone-family tests. Its aggregate test elapsed time barely changes, 80.203 → 79.963 s. Four workers overlap the work; the 40.147 s late-owner lifecycle test becomes the tail. There is no evidence here to reduce worker count.

At the four-worker memory peak, two integration-test processes account for approximately 1.119 GB and 0.811 GB, versus 0.121 GB in the CLI; no driver is present at that peak. At one worker the test process accounts for about 1.125 GB and the driver 0.221 GB. The tests retain and parse report and bundle JSON, including original/clean reports and corruption/resealing operations. PSS cannot separate live allocations from allocator-retained pages; this is not an allocation profile.

Both heavy runs show zero shared-cgroup CPU-throttling, memory-limit-event, OOM and memory-pressure deltas. These runs do not show memory contention causing the elapsed slowdown. The small one-worker run reads some cold pages and the four-worker run does not, so the exact small-test speedup is descriptive rather than a stable estimate.

## 5. Root self-check and patterns repeat workspace capture

Both root routes already request compact reports and skip strict clone matching. The full-report and all-pairs findings do not explain these steps.

| Local diagnostic command | Wall | Capture/compile | Handoff | Report write | Sampled tree peak PSS |
| --- | ---: | ---: | ---: | ---: | ---: |
| `scripts/style.sh --format json` | 53.281 s | 37.124 s | 2.586 s | 1.095 s | 2.592 GB |
| `scripts/style.sh patterns --fail-on-candidates --show-ignored` | 56.379 s | 39.227 s | 6.391 s | 1.137 s | 3.771 GB |
| Direct numeric check after patterns | 34.043 s | 27.902 s | 2.553 s | 1.096 s | 2.590 GB |
| Numeric replay of saved patterns capture | 10.018 s | None | 5.890 s | 1.110 s | 2.483 GB |

The first script has roughly 9.8 s of binary-build work before CLI execution, observed through the shell→CLI process transition. Its capture includes cold dependencies. Patterns starts with warm dependencies and requests PDGs; snapshot bytes grow from about 142.8 to 229.6 MB and parsing from 1.865 to 5.592 s. The peak memory in both live runs is in the driver, about 2.510/3.690 GB respectively.

Both modes use one shared advisory build lane and owner index. [owner_reuse.rs](../crates/cargo-rstyle/src/clones/capture/owner_reuse.rs) requires the previous capture plan to equal the requested plan. `compiler_patterns` differs, so [build_lane.rs](../crates/cargo-rstyle/src/clones/capture/build_lane.rs) explicitly cleans selected packages before recapture when switching modes. The third row is therefore a recapture, despite warm dependencies.

The fourth row used the saved patterns run with PATH, Cargo, rustc and driver all set to unavailable paths. It completed without compilation under strict replay verification. The original patterns presentation archive was preserved before replay rewrote it. The entire numeric advisory report is identical, including all 10,137 authored observation rows, 9,621 generated/unavailable effect rows, per-unit evaluations, diagnostics, coverage and dependency resolution. All six source inventory checksums and build contexts also match; artifact paths and replay configuration legitimately differ. Summary remains zero violations and complete complexity coverage. The [comparison record](../target/perf-investigation/live-20261031/root-report-comparison.json) preserves these checks; this establishes report equality, not byte identity of every raw FnFacts field.

`planning_input_complete` remains false for generated-owner authored edit facts, as in the live report; required-edit and call-graph completeness flags remain true. Replay preserves that existing coverage limit.

This demonstrates a feasible way to share capture between the two checks. It is not a paired measurement of a reordered end-to-end CI job. Replay decodes the larger graph-rich snapshot under strict verification; handoff differences do not isolate verification cost. Patterns also has about 9.5 s outside the existing top-level stage timers; evaluation/review needs an encompassing timer before attributing that residual specifically to clustering.

## 6. Dependency feature variants add compilation work

Sequential builds in a dedicated initially empty target produced:

| Build | Wall | CPU | Memory |
| --- | ---: | ---: | --- |
| Driver only | 49.71 s | 83.30 s | Maximum individual RSS 3,273,652 KiB |
| CLI afterward | 46.55 s | 74.50 s | Sampled tree PSS 3.276 GB |
| Workspace test prebuild afterward | 173.17 s | 321.36 s | Sampled tree PSS 4.118 GB |

Driver-only and CLI/workspace builds select different dependency features:

- `serde_json`: driver uses `default,std`; CLI adds `arbitrary_precision` through [rstyle-ccgraph](../crates/rstyle-ccgraph/Cargo.toml). This applies across platforms.
- `sha2`: ARM64 CLI additionally enables `asm,sha2-asm` through [cargo-rstyle](../crates/cargo-rstyle/Cargo.toml).

Cargo timing reports show normal `rstyle-pdg` compiled for 27.36 s in the driver phase and again for 27.38 s in the CLI phase. The second compilation occupies about 59% of that CLI build's critical path. Core also recompiles for 5.19 s, overlapping PDG; these durations must not simply be added. New dependencies and CLI linking remain necessary work.

Workspace tests introduce another normal-library dependency variant through proc-macro and transitive feature unions. The normal PDG compiles for 40.04 s; its separate `lib(test)` harness compiles for 39.61 s. The test harness is a distinct required artifact, not duplicate normal-library compilation. Preserved fingerprints show changed serde/derive, memchr/serde_json and libc/cpufeatures/sha2 dependency hashes despite unchanged PDG source/features/profile.

Warm driver-only builds, an identical repeat, joint driver+CLI build and workspace test prebuild then take **0.074, 0.050, 0.071 and 0.158 s**, all `Fresh`, with zero rustc invocations. Cargo can retain all these variants. The penalty concerns cold/invalidated variants, not an unavoidable rebuild on every switch. The local feature findings alone do not establish which variants the original CI cache restored.

The CLI/test build windows have shared-cgroup memory-limit-event deltas of 296/3,469, with about 0.113/1.091 s of full memory-pressure stalls. Both have zero CPU-throttling and OOM deltas. Reclaim pressure exists in the shared container, but the observed stalls do not explain most build elapsed time. These build measurements used two jobs and should not be extrapolated to unmeasured CI build concurrency.

## Prioritized implementation plan

1. **Index source positions once per source inventory.** Reuse `Positions` in `Definitions`; then index scan lookup and definition→active-owner lookup, preserving ambiguity checks and original source semantics. Validate Unicode, CRLF, conditional/generated sources and source inventory identity. Repeat fixed-owner padding and 128–4,101 scaling controls; expect the padding penalty to collapse before claiming overall savings.
2. **Run a compact-report experiment on the three large numeric lifecycle tests.** Preserve 4,101/4,098 owners and the 4,096 positive-payload limit. Those scenarios primarily consume advisory state and bundle references, which compact retains; keep explicit full-format contract tests. Rerun identical assertions and the one/four-worker resource comparison before changing their defaults. Scope/drop retained JSON and resealing buffers when no longer needed.
3. **Deduplicate shared evidence in full report serialization.** Reference a shared census/source table or the existing unit bundle while preserving full evidence availability. Measure source-control output/write time and consumer compatibility. Streaming JSON stdout can separately avoid loading the archive into a second whole String; text output alone does not remove archive work.
4. **Share one graph-rich workspace capture between root style and patterns.** Preserve both gates, summaries, coverage, replay verification and report artifacts. The saved-run experiment demonstrates numeric replay compatibility. Measure the actual proposed pipeline from the same clean/warm states; do not reuse the 10 s figure as a whole-job speedup claim.
5. **Align CI prebuild with the full workspace test graph.** Evaluate prebuilding the same targets/features nextest consumes, instead of driver-only then workspace. A joint normal driver+CLI build fixes only the first feature difference; development/test dependencies still matter. Retain verbose Fresh/Dirty logs and Cargo timing reports, and verify which graph variants the CI cache restores.
6. **Collect fresh three-platform resource evidence and bound pair work deliberately.** Run the same small/heavy subsets at 1/2/4 workers with actual runner quotas, native process memory, CPU and filesystem samples. Profile the remaining capture and consumer stages; add stage timers around source inventory and pattern evaluation. Separately decide whether exhaustive pair detail needs a bounded/summary mode with explicit completeness, using the identical-body control for validation.

The investigation scaffolding is [measure_ci.py](../scripts/measure_ci.py), the strengthened [watchdog.py](../scripts/watchdog.py), their behavioral tests, and documented examples in [scripts/README.md](../scripts/README.md). At the end of the investigation, all 12 Python behavioral checks and 22 representative Rust test executions passed. Local collection/analyzer scripts and detailed raw measurements remain in the investigation directory. The investigation itself did not change production Rust behavior, fixture boundaries, payload limits or CI concurrency.

## Implementation following the investigation

The follow-up uses a Low agent for implementation, builds, measurements and style/pattern fixes, with a Max agent independently reviewing correctness and the evidence. Expensive commands run sequentially. New observations are retained separately under [target/perf-implementation-20261031](../target/perf-implementation-20261031/); the original investigation data is preserved.

The implementation covers four changes:

1. `Definitions` builds the existing `Positions` index once per source. Bulk HIR extraction indexes source scans by signature/body span, preserving the earliest original match and conditional-profile gating; individual lookups retain a lightweight linear path. Source inventory buckets active facts by span while keeping source/name/file filters and duplicate-owner ambiguity checks. This removes repeated prefix scans and full-list searches without changing evidence identities.
2. The three large numeric lifecycle regressions explicitly request compact reports. Their 4,101/4,098 owners, 4,096 positive-payload limit, assertions and source/tool-free replay scenarios remain intact. Existing full-detail contract tests remain in the suite.
3. [ci-style.py](../scripts/ci-style.py) captures graph-rich patterns once and strictly replays that exact run for numeric style. It saves the original pattern archive before replay overwrites its presentation, retains both JSON presentations and gate statuses, and treats capture/tool/protocol failures as errors. Numeric findings still block the first CI gate; the next step applies the saved pattern gate. Candidate findings do not prevent numeric evaluation.
4. `scripts/test.sh --nextest` prebuilds the workspace test dependency graph rather than a driver-only graph. CI consumes the resulting normal CLI/driver executables directly, avoiding another production-only workspace build before style. Default CI dependency features match; custom nextest arguments are still forwarded and may select additional build variants. Standalone `scripts/style.sh` keeps its build behavior.

The full-report schema, exhaustive pair details, fixture sizes, policy thresholds, suppressions, ignore ledger and nextest concurrency are outside this batch.

### Source-index results

One fresh observation per size used the same full-report/baseline-export flags as the original scaling run. Fixture source, manifest and policy bytes match exactly. The [comparison record](../target/perf-implementation-20261031/capture-comparison.json) confirms equal observations, evaluations and diagnostic scopes, including the scope's `input_complete` and `effect_inputs_complete` fields, with complete baseline coverage at every size.

| Owners | Before | After |
| ---: | ---: | ---: |
| 128 | 0.573 s | 0.418 s |
| 256 | 0.520 s | 0.474 s |
| 512 | 0.695 s | 0.650 s |
| 1,024 | 1.268 s | 1.025 s |
| 2,048 | 2.826 s | 1.757 s |
| 4,101 | 7.544 s | 3.223 s |

At 4,101 owners, wall time falls 57%, command CPU falls 7.49 → 3.18 s, and sampled tree peak PSS stays 0.266 GB. Maximum individual RSS changes 217,252 → 218,324 KiB. These are single local observations, not confidence intervals or whole-suite speedup estimates.

Separate matching diagnostic runs reduce `capture.compile` from 0.751 → 0.481 s at 1,024 owners and 5.966 → 1.619 s at 4,101. At the larger size, handoff remains 0.445 → 0.454 s and full report writing 0.819 → 0.815 s; those remaining costs were not optimized by source indexing.

The fixed-size padding control now takes 0.975 s with padding after the functions and 0.975 s before them, versus 1.249/2.591 s before the change. Its capture-stage difference shrinks from 1.342 s to 0.015 s (0.461/0.476 s). Individual RSS is 116,704/116,700 KiB. This directly confirms removal of the material source-prefix penalty; it does not attribute the entire batch gain to one index.

### Heavy-test time and memory

The same six-small/five-heavy filters, workspace feature graph, prebuilt binaries and one/four-worker settings were repeated. All 22 selected executions passed; per-run JUnit and resources are retained in the [implementation nextest record](../target/perf-implementation-20261031/nextest-results.json).

| Heavy group workers | Execution before → after | Command CPU before → after | Sampled tree peak PSS before → after |
| ---: | ---: | ---: | ---: |
| 1 | 80.205 → 37.539 s | 79.34 → 37.57 s | 1.411 → 0.770 GB |
| 4 | 40.148 → 15.646 s | 79.60 → 37.58 s | 2.070 → 0.897 GB |

At four workers this is 61% less execution time and 57% lower observed peak PSS. The late-owner lifecycle remains the tail at 15.645 s, down from 40.147 s. Six small tests take 2.398/0.669 s at one/four workers, versus 2.542/0.661 s before; the four-worker control is essentially unchanged.

These measurements combine indexed capture with compact reports in the three numeric lifecycle tests. They do not isolate each change's contribution, and they do not predict a 61% improvement across all tests. CPU throttling and OOM counters do not increase during the heavy runs; process-tree memory remains a sampled observation rather than an allocation profile.

### Shared root-capture results

The separate current-source `cargo rstyle --format json` result exactly matches the shared route's `compiler_advisory`, summary and diagnostics. [The comparison](../target/perf-implementation-20261031/combined-numeric-comparison.json) also records matching completion/execution flags. The existing generated-owner planning limit remains: `planning_input_complete=false`. Pair matching/strict assessment remain unrequested, with their pair-completeness flags false. Both actual numeric and pattern commands pass, including `--fail-on-candidates --show-ignored` and the saved-status reader.

A paired warm experiment starts both routes with warm dependencies and a valid graph-rich capture. Both use this batch's Rust binaries, so the comparison concerns capture routing and reuse. The original route switches from graph to numeric capture, then back to graphs; the new coordinator retains and reuses the graph capture. An intervening pattern replay in the original sequence did not change the capture plan. Max verified matching selections, tool identities, owned-source inventories and advisory results. Both original live phases compile six units; the shared warm route reuses all six and Cargo finishes in 0.16 s.

| Route from a valid graph cache | Wall | Command CPU | Maximum sampled tree PSS across phases |
| --- | ---: | ---: | ---: |
| Separate live numeric + live patterns | 34.677 + 52.334 = 87.011 s | 88.66 s | 3.769 GB |
| Shared capture + strict numeric replay | 26.179 s | 24.04 s | 2.648 GB |

This is a 70% wall-time reduction for that warm-cache condition. The shared route separately took 64.162 s and 3.776 GB after source changes required fresh capture; that observation is not paired with an equivalent original-route run and does not establish a cold-CI speedup. The graph-rich driver peak remains a material cost when recompilation is required. These figures do not predict fresh x64/macOS or whole-job timings.

### Final verification and Max review

The Low agent completed the implementation and all workload execution. The Max agent independently reviewed the final Rust/CI changes, test identities, measurement commands, cache boundaries, report equality and this document, and accepted the batch with no remaining blockers.

- `cargo nextest run --workspace --locked --profile ci`: **3,778 passed**, five existing ignored tests, 251.216 s execution. Retained [JUnit](../target/perf-implementation-20261031/full-nextest/junit.xml) contains zero failures/errors; both added regression tests ran.
- `cargo test --workspace --locked --doc`: exit 0; the four crate groups currently contain zero doctest cases.
- All **22 Python behavioral checks** passed: eight watchdog, four measurement and ten shared-CI protocol checks. CI now runs these checks on every architecture.
- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed. Shell syntax and `git diff --check` also passed.
- Actual `cargo rstyle --format json` and `cargo rstyle patterns --fail-on-candidates --show-ignored` gates passed, as did shared coordinator statuses **numeric 0 / patterns 0** and the saved-pattern reader. The clean numeric check measures 10,142 authored functions. Existing complexity policies and the 12 reviewed pattern ignores are unchanged; no new suppression, threshold relaxation or ignore dismissal was introduced.

The added Rust helpers have one to six observed statements and no escaping mutations. The clean sparse presentation does not expose per-function cyclomatic/cognitive values; the [helper observation record](../target/perf-implementation-20261031/helper-complexity.json) states that limit explicitly. The style gate passes the existing complexity policy. The duplicated indexed-collection lifecycle detected in the first pattern run was refactored into a shared helper before final verification.

Finally, both normal `debug/cargo-rstyle` and `debug/rstyle-driver` executables were moved to unique retained backup paths while dependencies remained warm. Default workspace `cargo test --no-run -vv` recreated both executable paths successfully, with all targets Fresh and no rustc command launched. The [probe record](../target/perf-implementation-20261031/prebuild-recreation.json) and verbose log establish that the new prebuild supplies CI's required binaries. They do not measure a clean installation or quantify cold-build savings.

All new performance measurements are Linux ARM64 observations. Fresh x64/macOS validation remains for the next CI run. Full-report evidence deduplication and exhaustive pair-output scaling remain separate follow-up work.
