# CPG storage benchmark comparison

This study compares a lossless shared-table representation of fresh rstyle CPG output in JSON, FlatBuffers, FlexBuffers, and Protobuf. JSON and the fastest uncompressed binary codec are also tested with Zstandard level 3. The production CPG writer and readers are unchanged.

**FlatBuffers is fastest for encode plus full owned decoding in every measured dataset. Direct Zstandard compression of the original JSON gives the smallest payloads.** The shared arena reduces uncompressed storage, but its extra references and record tuples do not improve compressed size over that simple control. For the current disk-space goal, direct compression is the first production experiment to pursue; the fastest binary codec remains a candidate for a reader designed to consume the shared arena directly.

## Input and source provenance

The producer was built from `feat/cpg-document-routes` at `c0aaaedf30c32ece4e7b52ecca7ba7d4cace4a12` with local changes. The source inventory SHA256 is `a461076a209ebb34780410e40ff8283c3149fd22ca00dc9efeeca3572169f968`. `producer.json` records the exact binary hashes and dirty file inventory; the local artifact directory also retains the source patch, untracked-source archive, frozen binaries, emitted documents, and emit logs.

Input is a fresh `cpg emit` over the actual workspace. The core dataset is its real rstyle-core compilation document; the workspace dataset contains all seven emitted documents. Neither dataset is synthetic or duplicated to manufacture scale. Private Cargo build artifacts are excluded. These inputs do not establish a byte breakdown for the previously estimated 3.1 GB capture.

## Core results

The 1 input document(s) contain **923 functions** and **59,923 syntax nodes**. Their original JSON occupies **38,499,607 bytes (36.72 MiB)**.

**FlatBuffers has the lowest measured encode plus owned-decode time among the binary formats.** Adding Zstandard produces 4.23 MiB, 88.5% below the original JSON payload. The smallest tested shared representation is FlatBuffers + Zstd at 4.23 MiB.

| Representation | Payload MiB | Encode ms | Decode ms | Encode plus decode ms | Encode peak MiB | Decode peak MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Original JSON size control | 36.72 | — | — | — | — | — |
| Original JSON + Zstd size control | 2.98 | — | — | — | — | — |
| Shared JSON | 21.95 | 68.26 | 87.08 | 155.34 | 50.97 | 50.95 |
| Shared JSON + Zstd | 4.77 | 179.84 | 112.90 | 292.74 | 59.01 | 57.84 |
| FlatBuffers | 22.69 | 13.74 | 12.90 | 26.64 | 61.29 | 51.50 |
| FlexBuffers | 20.45 | 102.35 | 105.17 | 207.52 | 83.79 | 49.27 |
| Protobuf | 12.96 | 20.76 | 22.79 | 43.56 | 41.97 | 41.88 |
| FlatBuffers + Zstd | 4.23 | 103.82 | 33.92 | 137.75 | 69.08 | 56.00 |

Times and memory values are medians. The JSON records retain every sample, minimum, maximum, and median absolute deviation. Original JSON controls are size-only and are excluded from the timing ranking.

![Core comparisons](core.png)

The normalization step, including input I/O, hashing, statistics, and exact reconstruction checks, took 1.910 s and reached 693.29 MiB process peak RSS. It is a separate one-time prototype cost, excluded from the per-codec timings.

### Sharing observed

The source contains 1,626,053 JSON value occurrences, represented by 387,008 distinct arena values. Repeated string and object-key bytes total 28.04 MiB before interning and 4.02 MiB afterward. These are logical byte counts, not independently compressed sizes or heap estimates.

| Unit table | Record occurrences | Exact distinct record contents |
| --- | ---: | ---: |
| definitions | 3,935 | 3,935 |
| files | 72 | 72 |
| imports | 855 | 855 |
| namespaces | 331 | 331 |
| types | 8,531 | 8,531 |
| values | 26 | 26 |

Compared with direct compression of the original JSON, FlatBuffers + Zstandard uses 1.420 times the storage. This control matters: shared records and binary encoding do not guarantee a smaller compressed file than repetitive JSON.

## Workspace results

The 7 input document(s) contain **11,016 functions** and **827,278 syntax nodes**. Their original JSON occupies **530,305,917 bytes (505.74 MiB)**.

**FlatBuffers has the lowest measured encode plus owned-decode time among the binary formats.** Adding Zstandard produces 50.33 MiB, 90.0% below the original JSON payload. The smallest tested shared representation is FlatBuffers + Zstd at 50.33 MiB.

| Representation | Payload MiB | Encode ms | Decode ms | Encode plus decode ms | Encode peak MiB | Decode peak MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Original JSON size control | 505.74 | — | — | — | — | — |
| Original JSON + Zstd size control | 37.84 | — | — | — | — | — |
| Shared JSON | 269.06 | 261.72 | 497.12 | 758.84 | 588.92 | 588.99 |
| Shared JSON + Zstd | 57.75 | 950.83 | 747.44 | 1698.27 | 650.10 | 648.87 |
| FlatBuffers | 275.42 | 186.98 | 134.23 | 321.21 | 836.60 | 595.14 |
| FlexBuffers | 256.94 | 647.42 | 703.99 | 1351.40 | 1021.29 | 576.66 |
| Protobuf | 159.74 | 247.14 | 236.25 | 483.40 | 479.59 | 479.58 |
| FlatBuffers + Zstd | 50.33 | 671.93 | 337.43 | 1009.36 | 889.63 | 645.74 |

Times and memory values are medians. The JSON records retain every sample, minimum, maximum, and median absolute deviation. Original JSON controls are size-only and are excluded from the timing ranking.

![Workspace comparisons](workspace.png)

The normalization step, including input I/O, hashing, statistics, and exact reconstruction checks, took 11.039 s and reached 6856.48 MiB process peak RSS. It is a separate one-time prototype cost, excluded from the per-codec timings.

### Sharing observed

The source contains 21,492,755 JSON value occurrences, represented by 4,524,680 distinct arena values. Repeated string and object-key bytes total 391.21 MiB before interning and 47.50 MiB afterward. These are logical byte counts, not independently compressed sizes or heap estimates.

| Unit table | Record occurrences | Exact distinct record contents |
| --- | ---: | ---: |
| definitions | 35,953 | 33,653 |
| files | 1,264 | 1,264 |
| imports | 9,682 | 9,682 |
| namespaces | 2,976 | 2,976 |
| types | 78,910 | 73,614 |
| values | 94 | 92 |

Compared with direct compression of the original JSON, FlatBuffers + Zstandard uses 1.330 times the storage. This control matters: shared records and binary encoding do not guarantee a smaller compressed file than repetitive JSON.

## Method and practical limits

- Every codec receives identical strings, scalar values, child references, and document roots. The prototype interns exact strings and JSON substructures into a shared arena. It does not merge compiler entities by display name or discard observations, provenance, source text, or unknown fields.
- Preparation reconstructs every original JSON document and checks semantic JSON equality. Every decoded result is validated outside the timing interval and compared using a SHA256 over every materialized arena field with the prepared model. Original document SHA256 values and deterministic wire hashes are retained.
- Seven measured repetitions follow one warmup. Format order is randomized with seed 20261105. Each encode and decode runs in a separate optimized Rust process pinned to CPU 19. The host is shared; cgroup CPU and memory counters are retained. Whiskers show the observed range, not a confidence interval.
- Fastest means the lowest sum of median encode time and median full owned-decode time among FlatBuffers, FlexBuffers, and Protobuf. The winning codec is then measured with compression in additional trials. Read-heavy workloads can choose a different weighting.
- Timings cover encoding to an owned wire buffer and decoding to the same fully owned shared arena. File I/O, loading the common model, normalization, digest calculation, validation, and buffer destruction are outside those intervals. Compressed variants include the compression/decompression stage. This is not an end-to-end compiler or rstyle-analysis benchmark.
- All compression uses Zstandard level 3 with one thread. Rust codecs use zstd 0.14.3 with libzstd 1.5.7. The original-JSON size control uses python-zstandard 0.25.0 with the same libzstd version, one independent frame per original document, and verified byte-for-byte reconstruction.
- Stage peak RSS is sampled from Linux VmHWM after resetting the high-water mark with clear_refs=5. It includes the loaded common model for encode, or wire buffer for decode. Preparation peaks are reported separately and can exceed codec peaks. Extra retained allocator memory and builder capacity remain part of process RSS.
- FlatBuffers uses generated vectors of fixed-size arena value structs and transfers its native builder buffer without an added full-buffer copy. Protobuf uses prost messages with packed child IDs. FlexBuffers uses its Rust Serde serializer with default key sharing. JSON uses compact value tuples. No binary codec wraps JSON bytes. Pinned versions and schemas accompany the harness.
- Results apply to this common lossless arena and Rust implementations. A domain-specific typed CPG schema, native zero-copy access, independently compressed function chunks, other languages, different compression levels, and on-disk query indexes are not measured here. The current archive is monolithic; production chunking and bounded-memory normalization need a separate design.

## Reproduction and retained measurements

See [the benchmark instructions](../../README.md) for build, capture, trial, and report commands. `measurements.csv` contains medians, ranges, median absolute deviations, sizes, and memory peaks. `raw/<dataset>/trials.jsonl` contains every warmup and measured operation. `raw/<dataset>/results.json` retains input hashes, environment, selection, and all aggregate samples. The HTML report works offline, and PNG/SVG charts are included for sharing.
