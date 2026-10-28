# Fast transport checksums

`rstyle.shared-unit.manifest/2` contains `checksum: "xxh3-64:<16 lowercase hex>"`
and the exact byte count. The old `sha256` field and manifest version are retired.
Capture artifact inventories likewise replace `sha256` with the self-describing
`checksum` value. An older manifest or missing new field fails import; there is
no dual checksum runtime. The logical bundle schema stays `/2` independently.

XXH3-64 detects accidental truncation, changes and stale/mismatched handoff
files under the supported nonadversarial local transport contract. It deliberately
does not preserve SHA-256's cryptographic collision resistance. Neither the old
adjacent SHA digest nor this checksum authenticates an artifact against someone
able to replace both data and checksum. It is not used for semantic identity,
nominal type equivalence, graph admission or cache authorization.

The existing boundary ownership remains: publication updates one streaming state
with the bytes actually written, including partial writes, and requires final
flush before publishing the seal. The bundle's inventory entry copies that seal
without reading the bundle. The importer checks the inventory/manifest agreement
and validates the exact owned bundle buffer once, then decodes that same snapshot.
Auxiliary files each receive the same algorithm at publication and import. File
inventory completeness, run/selected-unit association and strict/current-run
authority remain independent checks. Post-import file replacement cannot mutate
the verified snapshot.

One dependency, `xxhash-rust` with its `xxh3` feature, supplies the streaming and
single-buffer implementations. It can also serve the separate matching lookup
evaluation; this change does not alter matching interning. Source identities,
source-pack fingerprints, compiler evidence, historical Python snapshots and
frozen contract meanings remain SHA-256.

Standalone paired PDG/CFG manifests use the same `checksum` value directly.
There is no historical SHA field alias or fallback reader. Exact paired keys
and the complete two-document inventory remain required; neither checksum
nor byte consistency grants semantic admission.

Regressions cover published XXH3 vectors, streaming/single-buffer equality across
chunk boundaries, partial writes, failed flush, same-length corruption, truncated
or extended data, changed auxiliary files, stale inventory claims, immutable
snapshot ownership and failed selected-unit inspection. Existing SHA-256 vectors
and edited-file checks are retained as test-only CLI digest coverage.

`transport_checksum_bench` compares the same SHA-256 and XXH3 streaming calls on
one MiB of deterministic bytes with 1/16/1024/65536-byte chunks. Prebuild it before
running; its warmup and three raw samples per case are separate from pipeline
timings. Actual pipeline workloads remain the basis for choosing this dependency.

## Measurements and validation

Baseline `d6b9522`; candidate is this commit. Saved baseline binaries:
`/tmp/rstyle-before-fast-checksum/{debug,release}`. The
[312 command samples and checksum microbenchmark](transport-checksum-samples.json)
record embedded producer revisions and all raw wall/RSS data. Aarch64, eight-CPU
quota, 16 GiB, nightly-2026-09-30, locked dependencies. Build/setup is excluded;
there are no concurrent builds/tests/timings. Both analyzer profiles use the same
ordinary check-profile fixtures and source paths. Each trace setting has one
warmup, three ordinary samples and a separate diagnostic run. Three strict
imports per workload/profile also pass. Wall resolution is 0.01 seconds. RSS is
a process proxy rather than allocator event accounting.

Median ordinary no-trace wall seconds:

| Workload | Debug before → after | Release before → after |
| --- | --- | --- |
| minimal | 0.23 → 0.23 | 0.21 → 0.21 |
| read | 1.15 → 1.17 | 0.32 → 0.32 |
| inherent | 1.69 → 1.70 | 0.37 → 0.34 |
| shared-fanout | 0.88 → 0.88 | 0.28 → 0.30 |
| many-pairs | 1.70 → 1.66 | 0.35 → 0.39 |
| larger-graphs | 3.74 → 3.72 | 0.72 → 0.70 |

These command differences do not establish a uniform speedup. All decisions,
exact score inputs, stage outcomes, admissions, exhaustive pairs and completeness
passed parity across profiles and trace settings. No suite-wide gain is claimed.

Isolated streaming checksum median milliseconds per MiB (three samples):

| Chunk bytes | Debug SHA-256 → XXH3 | Release SHA-256 → XXH3 |
| --- | --- | --- |
| 1 | 110.176 → 53.769 | 4.504 → 2.645 |
| 16 | 51.570 → 6.265 | 1.883 → 0.182 |
| 1024 | 45.489 → 2.731 | 1.809 → 0.042 |
| 65536 | 45.390 → 2.610 | 1.805 → 0.042 |

The single fast-hash dependency is retained because it implements the intended
nonsecurity checksum contract and reduces the measured checksum operation in
every tested chunk case. The command workload includes larger unrelated costs;
the microbenchmark ratio is not extrapolated to analysis or tests.

Prebuild commands and separate measurement commands:

```sh
cargo build --locked -p rstyle-pdg --examples
cargo build --release --locked -p cargo-rstyle -p rstyle-driver
cargo build --release --locked -p rstyle-pdg --examples
target/debug/examples/transport_checksum_bench
target/release/examples/transport_checksum_bench
python3 scripts/measure_library_resolution.py after target /tmp/rstyle-fast-checksum-measurement
python3 scripts/measure_broad_pipeline.py after target /tmp/rstyle-fast-checksum-broad
```

671 nextest tests passed, zero skipped; workspace doctests passed; warning-free
workspace/all-target Clippy and formatting passed. Four focused manifest tests,
24 study bookkeeping and two measurement-harness Python tests passed. Both gates
report zero style violations and zero actionable pattern candidates, with 15
existing reviewed ignores unchanged. Logs: `/tmp/rstyle-fast-checksum-*`.
