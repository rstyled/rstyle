# JSON performance investigation, 2026-10-31

Low collected the experiments and Max independently reviewed the source, controls and results. The strongest targets are repeated build metadata in snapshots and repeated parsing/serialization in a test helper. A binary-format migration has not been shown to improve the complete workflow.

The investigation below was collected with scratch prototypes before production changes. The subsequent implementation and its validation are recorded in the final section.

## Input and method

The retained advisory capture contains 4,101 functions: mostly empty bodies and one late owner with five parameters. Its raw-facts/4 snapshot is 23,520,553 bytes, publication phase `compiler-advisory-only`, SHA-256 `3fe1a3ebf65b9f8410676eb485770589964848af17a8113a6db1ebee4684237e`. It is a scaling fixture, not the exact process execution used for the earlier flamegraph.

The scratch Rust probe calls the existing public snapshot readers and writers. Accepted measurements use the repository's dependency versions and development-profile overrides, including `serde_json` 1.0.151 with `arbitrary_precision` and package optimization level 2. Workspace code remains a debug build. Each of 14 modes ran in three separate process batches, with three operations per batch, in forward, reversed and rotated order. All 42 accepted batches passed.

Loop timings include the operation and destruction of its outputs. They exclude input-file reads and any explicitly prepared facts or retained JSON tree. Process CPU and maximum RSS include setup, input buffers and all three operations. RSS measures one probe process, not the earlier CLI/driver/test process tree or individual allocation sizes. These are local Linux ARM64 measurements with warm filesystem caches.

An initial probe used different dependency/profile settings; those results are retained separately and excluded from the accepted timings. Setup and semantic validation also run outside measured loops.

## Snapshot bytes are mostly build metadata

Serialized field-value sizes exclude containing keys and punctuation. `frontend_options` is nested inside `build`; the two rows must not be added.

| Component | Value bytes | Share of complete snapshot |
| --- | ---: | ---: |
| Build metadata | 19,901,790 | 84.6% |
| Nested frontend options | 11,154,357 | 47.4% |
| Function bodies | 1,266,109 | 5.4% |
| Signatures | 1,269,636 | 5.4% |
| Bindings | 9,343 | 0.04% |
| Shared sources | 72,749 | 0.31% |

The same 1,327-byte compiler cfg settings value (`build.cfgs`) appears 4,101 times, totaling 5,442,027 value bytes. Invocation metadata repeats another 3,338,214 bytes, and selected-unit context repeats 2,784,579 bytes. The complete frontend-options maps differ because they also contain owner-specific observations; sharing whole maps would miss the common subfields.

The raw writer repeats build settings for every function in [function_sources/build.rs](../crates/rstyle-pdg/src/unit_store/bundle/function_sources/build.rs). The supported style/2 format already shares invocation and selected-unit options through [options/bundle/sharing.rs](../crates/rstyle-pdg/src/frontend/options/bundle/sharing.rs). The report path also has an existing exact-settings interner in [nominal_binding/builds.rs](../crates/rstyle-pdg/src/nominal_binding/builds.rs).

Two size controls establish different opportunities:

- The supported numeric style projection is **16,615,102 bytes, 29.4% smaller**. It preserves source, signatures, build/options, observations and the original body identity, while explicitly marking bindings uncaptured. It combines option sharing, a different body representation and a different schema; the entire gain cannot be attributed to one change.
- A lossless experimental common-settings overlay is **14,861,383 bytes, 36.8% smaller**. It removes only identical build fields outside frontend options into one common record. Reconstruction preserves the original JSON value, publication phase and full typed facts, including bodies, bindings and all per-owner options. This is a byte-saving control, not a supported decoder, a general context table or a measured runtime improvement.

The body-child-removal control is byte-identical to the original because these bodies are already childless. This fixture does not establish the value of body reduction on body-heavy code.

## A concrete repeated-processing cost in the test helper

[demanded_diagnostic_owners.rs](../crates/cargo-rstyle/tests/compiler_advisory/demanded_diagnostic_owners.rs) parses the original bundle for restoration, then reads and parses it again to remove a late owner's advisory observation. [only.rs](../crates/cargo-rstyle/tests/compiler_advisory/only.rs) serializes the changed value and later serializes the original value for restoration.

The controlled alternative retains the original bytes, parses one working value, applies the same observation removal, encodes the changed value and restores the original bytes. Both paths compute two transport checksums and encode two manifest seals.

| Helper-core operation | Median seconds | Batch range | Process peak RSS |
| --- | ---: | ---: | ---: |
| Existing two-parse, two-encode pattern | 2.4209 | 2.3883–2.4214 | 410.4–410.5 MiB |
| Retain original bytes, one parse and encode | 1.1995 | 1.1977–1.2111 | 218.7–218.8 MiB |

This halves the measured helper-core time and reduces process high-water RSS by about 47%. File I/O, ledger updates, subprocess replay and diagnostic assertions are excluded, so this is not an end-to-end test or CI speedup.

Untimed validation confirms identical changed bytes and typed facts. Both restores reproduce the original JSON value and full typed facts. Retained-byte restoration is also byte-for-byte exact; the existing JSON-tree re-encoding changes object-key order. An initially overstrict baseline byte-equality assertion was corrected before accepted measurements; its failed setup record is retained separately.

Independent replay validation remains necessary. Retaining bytes within one helper does not authorize reusing a verification result across changed evidence.

## Typed readers and binary encoding

| Operation | Median seconds per operation |
| --- | ---: |
| Existing full typed raw reader | 0.3154 |
| Existing header reader | 0.3154 |
| Supported style reader | 0.2739 |
| Generic JSON-value parse | 0.3982 |
| Generic JSON-value encode, prepared value | 0.7974 |
| Public raw writer, prepared facts | 0.7946 |
| Same-value CBOR parse | 0.9011 |
| Same-value CBOR encode, prepared value | 0.2823 |

The full and header readers perform alike on this sparse-body fixture. Header decoding still validates the body and retains bindings; it does not remove build metadata. The style projection decodes about 13% faster here. Existing raw decoding already uses typed structs, with dynamic JSON values inside options and inventories; it does not first construct a generic tree for every field.

CBOR preserves the identical raw-shaped JSON value but is only **0.55% smaller**: 23,390,898 bytes. With the existing arbitrary-precision representation, its payload includes 94,408 private JSON-number wrapper keys. This does not measure a native-number binary schema. CBOR encodes faster and decodes slower in these controls. JSON uses the workspace's optimized dependency configuration; the scratch CBOR candidate is untuned. These results do not rank codec algorithms or rule out an optimized typed binary integration.

Bincode 1.3.3 encodes this value but rejects decoding because it does not support `deserialize_any`, which both generic values and the existing tree-or-flat body adapter require. A bounded one-fact typed CBOR roundtrip succeeds. No complete binary diagnostic reader/writer or malformed-input rejection equivalence was implemented.

## Recommended implementation order

1. Change the missing-observation helper to retain original bytes and parse once for editing. Preserve all assertions, seal/ledger handling and independent CLI replays; then measure the actual lifecycle test.
2. Prototype a versioned snapshot representation that shares exact build settings and common invocation/context metadata while retaining per-owner observations and complete requested evidence. Reuse existing sharing/interner concepts and test changed settings, invalid references and version rejection before claiming savings in the real pipeline.
3. Reconsider binary encoding after those measurements identify the remaining transport cost. Compare equivalent payloads, validation contracts and optimization settings.

The results do not explain every slow test. The [earlier investigation](performance-investigation-2026-10-31.md) and CPU profiles identify separate proof-verification and graph-algorithm costs.

## Evidence and reproduction

All raw inputs, scripts, scratch sources, matched lock/profile settings, validation records and measurements are retained under [target/perf-json-investigation-20261031](../target/perf-json-investigation-20261031/):

- [Detailed findings](../target/perf-json-investigation-20261031/FINDINGS.md), [accepted measurements](../target/perf-json-investigation-20261031/results-matched.json) and [summary](../target/perf-json-investigation-20261031/summary-matched.json).
- [Input provenance](../target/perf-json-investigation-20261031/input/manifest.json) and [metadata decomposition](../target/perf-json-investigation-20261031/input/build-option-decomposition.json).
- [Input preparation](../target/perf-json-investigation-20261031/prepare_input.py), [lossless overlay control](../target/perf-json-investigation-20261031/factor_build.py), [Rust probe](../target/perf-json-investigation-20261031/probe/src/main.rs) and [bounded runner](../target/perf-json-investigation-20261031/run_probe.py).
- [Corrected helper validation](../target/perf-json-investigation-20261031/measurements-matched/validate-lifecycle-corrected/result.json) and [full-facts overlay validation](../target/perf-json-investigation-20261031/measurements-matched/validate-factored/result.json).

`target/` is ignored by Git; this document preserves the main conclusions independently of those local artifacts. The scratch experiments above did not include a full suite or native macOS/x64 run. Production validation follows below.

## Implemented improvements

The first two recommendations are implemented using Low implementation followed by Max source and measurement review. The original benchmark controls above remain investigation evidence; the measurements below evaluate the production changes.

The missing-observation lifecycle helper now retains the original bundle bytes, parses one editable JSON value, and restores the exact original bytes. Changed and restored bundles still update their manifest and ledger seals, and every CLI replay and diagnostic assertion remains independent. An added assertion checks byte-for-byte restoration.

Raw diagnostic snapshots now use `rstyle.diagnostic.raw-facts/5`. An exact typed settings table stores common compiler settings once; each function retains its own options and references its settings record. Invocation and selected-unit metadata use the existing inline-first option sharing. Full bodies, bindings, source records, per-owner observations and publication phases are preserved. Expanding the measured snapshot's settings and option references reproduces its original raw/4 JSON value exactly, apart from the transport structure and version tag.

The first settings record has a bounded exact-equality shortcut before the serialized-key map lookup. This avoids encoding an identical lookup key for every function in a normal single-context capture. Other settings retain the exact map fallback; heterogeneous settings, status/reason distinctions and vector order remain meaningful. Existing option sharing still searches its distinct records linearly, so this change does not establish a global asymptotic bound for snapshots with many different invocation contexts.

The repository is preproduction, and compatibility with obsolete snapshots is unnecessary. The readers accept only the current raw/5 and style/2 schemas. Raw build references decode directly into a closed typed struct, without a speculative legacy alternative or an intermediate JSON tree. Tests reject obsolete versions, missing or null settings tables, duplicate or unused settings records, invalid references, inline builds, unknown fields and phase mismatches. Current deep bodies, bindings and source ownership remain validated. The strict shared-unit/7 format retains its existing contract; diagnostic snapshots confer no graph, proof or admission authority. Older saved diagnostic captures need regeneration.

The sole checked-in diagnostic fixture was migrated from raw/2 to raw/5. Inverse expansion reproduces the entire original JSON value, including the outer instance/span metadata and original publication phase. Native validation also checks that the migrated snapshot restores the same full typed facts and headers as the old decoder, then roundtrips through the current writer. The test loaders and their proof assertions remain intact.

### Production measurement method

The baseline is commit `0ede40754d1e81b7e2318ecce001ecd1fe79b9cd`. Baseline and candidate CLI, driver and test executables were copied to separate immutable locations with SHA-256 manifests. Timing excludes executable builds. All workloads run serially on Linux ARM64 using the pinned nightly, two Cargo build jobs and no incremental compilation. Lifecycle controls enable identical performance-event instrumentation; isolated codec controls leave it disabled. Measurements use warm filesystem caches.

The actual lifecycle workload is `compiler_advisory::demanded_diagnostic_owners::late_owner_baselines_and_reviews_preserve_clean_missing_and_context_outcomes`, with one libtest thread and all original assertions. GNU maximum RSS is the largest individual process; sampled process-tree PSS is a lower-bound observation, not an allocation profile. The bounded watchdog, exact commands and resource samples are retained for every run.

The separate codec control uses the same 4,101 typed facts in each version's native input: the retained raw/4 snapshot for the baseline and its equivalent raw/5 snapshot for the final reader. Validation outside timing checks full typed facts, headers and wrong-phase rejection, and independently expands the new representation to the original JSON value. The final writer must reproduce the validated raw/5 input byte for byte, and final readers must reject obsolete raw/4 input. Each reported codec batch performs three operations; input reads and prepared-facts setup are outside the timed loop, while output destruction is included. Process maximum RSS includes setup. The old and new probes use matching dependencies and profile settings.

Implementation evidence is retained under [target/perf-json-implementation-20261031](../target/perf-json-implementation-20261031/), including immutable-binary manifests, [summary data](../target/perf-json-implementation-20261031/summary.json), lifecycle logs, snapshot controls, equality validation and check outputs. Native x64/macOS measurements remain for hosted CI. These changes address repeated transport and helper work; the previously identified graph and proof-verification costs remain separate.

### Measured results

Three fresh baseline/final pairs ran serially, reversing execution order in the middle pair. Every lifecycle run passed all assertions. Binary and input SHA-256 checks matched before and after the complete experiment. Earlier `before`/`after` exploratory measurements are retained separately and excluded from these final results.

| Metric | Baseline | Final | Reduction |
| --- | ---: | ---: | ---: |
| Full snapshot payload | 23,520,553 bytes | 9,096,863 bytes | 61.3% |
| Lifecycle wall time | 15.793 s | 11.455 s | 27.5% |
| Lifecycle accounted user + system CPU | 15.56 s | 11.40 s | 26.7% |
| Lifecycle maximum individual-process RSS | 478.6 MiB | 224.8 MiB | 53.0% |
| Lifecycle sampled process-tree peak PSS | 719.5 MiB | 479.2 MiB | 33.4% |
| Full typed reader, per operation | 0.3128 s | 0.2471 s | 21.0% |
| Header reader, per operation | 0.3117 s | 0.2429 s | 22.1% |
| Prepared-facts writer, per operation | 0.7921 s | 0.3636 s | 54.1% |

Lifecycle wall time ranged from 15.759–15.827 seconds for the baseline and 11.446–11.502 seconds for the final implementation. Lifecycle resource rows are medians across the three runs; CPU totals are summed within each run before taking their median. Codec time rows are medians of three batch means, each containing three operations. Full-reader ranges were 0.3095–0.3145 versus 0.2449–0.2475 seconds; header-reader ranges were 0.3074–0.3132 versus 0.2411–0.2430; writer ranges were 0.7914–0.7944 versus 0.3600–0.3643.

Each lifecycle retained ten diagnostic decodes. Median parse-stage totals fell from 2.524 to 1.656 seconds, while restoration rose from 0.558 to 0.704 seconds because shared settings are cloned into typed facts during restoration. The combined, consecutive parse/restoration stages fell from 3.082 to 2.359 seconds. These phase totals do not sum overlapping parent and child stages.

The lifecycle gain measures the helper and transport changes together. The codec controls isolate supported readers and the writer over equivalent facts. These local Linux ARM64 measurements establish improvements for the measured workloads; they do not establish a complete CI or cross-platform speedup.

### Validation

The full workspace run passed **3,786 tests**, with **5 existing skips**. Its retained [JUnit report](../target/perf-json-implementation-20261031/workspace-fixture-final/junit.xml) records zero failures and errors. Workspace doctests, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` also passed.

Low ran the actual shared-capture numeric and pattern gates, including the saved-pattern status reader, against the final rebuilt executables. Both returned **0**, with **zero violations across 10,155 captured functions** and complexity measured for all 10,155. Capture, assessment, summary and required-edit checks are complete; the reports still mark `planning_input_complete: false`. These are the configured analysis gates. No thresholds, ignore entries, dependencies, workflows or suppression comments were added or relaxed.

Native validation confirms full facts and headers for the migrated checked-in fixture, wrong-phase rejection, and a current-format writer roundtrip. The 4,101-owner control reproduces its validated raw/5 bytes exactly and rejects obsolete raw/4 input in both full and header readers. Rust source and fixture hashes remain unchanged throughout the accepted validation run.
