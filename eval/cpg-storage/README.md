# CPG storage benchmarks

This standalone Rust experiment compares lossless shared-table JSON, FlatBuffers,
FlexBuffers, and Protobuf on real `compat.cpg` documents. JSON and the fastest
uncompressed binary codec are also measured with Zstandard. It does not change the
production CPG writer, schema, readers, dependencies, or build profiles.

Original JSON is timed plain and with Zstandard as well. Its encode path uses
`serde_json::Value`, as the producer does; its decode path imports the actual
`rstyle-core` CPG `Document` model directly. Timing ranks include all eight variants.
Original JSON retains its document boundaries and shared formats use one interned
archive, so the overall rank includes representation differences as well as codec
cost. It is not an end-to-end analysis speedup measurement.

The [2026 November 5 comparison](reports/2026-11-05/comparison.md) contains measured
results for a fresh rstyle-core emit and a full workspace emit. The
[offline interactive report](reports/2026-11-05/comparison.html) includes dataset
selection, charts, and tables. PNG/SVG charts, CSV measurements, and raw trial
records sit alongside it.

The [fresh self-check comparison](reports/2026-11-05-self-check/comparison.md)
uses the latest committed document route at `88adc996`: all 1,228 saved batch
documents and an actual rstyle-core subset. Its
[interactive report](reports/2026-11-05-self-check/comparison.html) and saved
capture inventory explain the larger disk footprint. The two studies use
different source snapshots and capture routes; they are not a controlled
before/after comparison.

## Build and check

Run from the repository root with its pinned Rust toolchain:

```sh
export PATH=/home/dev/.cargo/bin:$PATH
cargo build --manifest-path eval/cpg-storage/Cargo.toml --release --locked
cargo test --manifest-path eval/cpg-storage/Cargo.toml --release --locked
cargo clippy --manifest-path eval/cpg-storage/Cargo.toml --release --all-targets --locked -- -D warnings
cargo fmt --manifest-path eval/cpg-storage/Cargo.toml --check
```

The nested `[workspace]` keeps codec dependencies and the experimental lockfile
separate from the production workspace. Generated FlatBuffers bindings are
checked in; building does not require `flatc` or `protoc`. Regenerate the
FlatBuffers bindings with **flatc 25.12.19**:

```sh
flatc --rust -o eval/cpg-storage/src eval/cpg-storage/schema/archive.fbs
```

The Protobuf schema mirrors the `prost::Message` annotations in `src/model.rs`.
Pinned codec versions are FlatBuffers/FlexBuffers 25.12.19, prost 0.14.4, and zstd
0.14.3 with libzstd 1.5.7. JSON uses serde_json 1.0.154.

## Capture input

Build and run the producer from the **latest feat/cpg-document-routes checkout**,
including its local changes if those are the intended input. Keep the benchmark
branch separate. In the commands below, `source_root` points to that checkout and
`study_root` is a new local artifact directory:

```sh
source_root=/home/dev/src/rstyle-main/rstyle
study_root="$PWD/target/cpg-storage-new-run"
mkdir -p "$study_root"
cargo build --manifest-path "$source_root/Cargo.toml" -p cargo-rstyle -p rstyle-driver --bins -j 2
CARGO_BUILD_JOBS=2 "$source_root/target/debug/cargo-rstyle" rstyle \
  --manifest-path "$source_root/Cargo.toml" cpg emit --out "$study_root/emitted-workspace"
```

`cpg emit` creates a private `target` directory inside its output. Only the
top-level `*.cpg.json` files are benchmark payloads. They must remain unchanged
throughout the study. The recorded study froze both producer executables, saved
their hashes, the source revision, a source inventory hash, local-change inventory,
patch, and untracked-source archive before emitting.

### Capture the default self-check route

The larger study uses the actual default style route, which records compiler
observations and batches owners by source file (up to 256 owners per batch). It
therefore has different content and boundaries from `cpg emit`. Run without a
subcommand and retain the reported `run_directory`:

```sh
CARGO_BUILD_JOBS=2 RSTYLE_DRIVER="$source_root/target/debug/rstyle-driver" \
  "$source_root/target/debug/cargo-rstyle" rstyle \
  --manifest-path "$source_root/Cargo.toml" --format json > "$study_root/self-check.json"
# Copy the completed run into the study before using it as immutable input.
python3 eval/cpg-storage/inventory.py "$study_root/captured-run" \
  --out "$study_root/capture-inventory.json"
```

`inventory.py` requires the system `libxxhash` library. It checks every CPG file
against the document manifests, retains SHA256 values and actual saved file
sizes, and estimates compact-JSON field and exact repeated-record bytes one
document at a time. These estimates describe existing content; they do not
measure a proposed production layout. The full capture includes generated owners
and observations beyond the authored functions counted by the style summary.

Pass every `*.cpg.json` recursively from the saved capture to `run.py`.
Keep compilation-unit directories when copying: basenames are reused between
units. The compression control also assigns globally unique output filenames.

## Run matched trials

Use an available CPU from `os.sched_getaffinity(0)` for optional pinning. The
recorded study used CPU 19. One warmup and seven measured repetitions are the
defaults; every operation runs in a fresh process, sequentially.

```sh
binary="$PWD/eval/cpg-storage/target/release/cpg-storage-bench"
python3 eval/cpg-storage/run.py --binary "$binary" \
  --out "$study_root/core" --cpu 19 \
  "$study_root"/emitted-workspace/rstyle_core-*.cpg.json
python3 eval/cpg-storage/run.py --binary "$binary" \
  --out "$study_root/workspace" --cpu 19 \
  "$study_root"/emitted-workspace/*.cpg.json
```

The runner first validates exact reconstruction and writes a common `model.json`.
It then measures uncompressed shared JSON, FlatBuffers, FlexBuffers, Protobuf, and
shared JSON with Zstandard level 3. It selects the uncompressed binary codec with
the lowest **median encode time plus median full owned-decode time**, then measures
that codec with Zstandard in additional trials. Encode and decode are also
reported separately. Compression uses one thread.

The original-JSON rows join the randomized trials. They process documents
individually, sum the codec times across them, and report the maximum stage RSS.
The typed reader output remains alive through the timing boundary and is dropped
afterward. Canonical full-document SHA256 validation runs outside timing, after
dropping the typed output. Separate content digests validate the original document
set and the shared arena. The raw records retain the native representation and
encode, decode, and combined timing ranks.

Preparation time, validation time, file I/O, and full process wall time are retained
separately from codec timings. Linux stage RSS includes the input model or wire
buffer; resetting `/proc/self/clear_refs` to 5 excludes earlier loading peaks.
Preparation has its own memory peak. Results include the reset status so an
unsupported platform cannot silently claim stage-only memory measurement.

Use a new output directory for another study. `--reuse-prepared` is available when
deliberately rerunning the same prepared input; it verifies original document
hashes. Keep previous trials separately if they need to be retained.

## Generate comparison reports

Python 3.11 or newer is required for the scripts. Install the reporting
dependencies in a local environment, then generate the original-JSON compression
control and reports:

```sh
uv venv "$study_root/report-venv"
uv pip install --python "$study_root/report-venv/bin/python" \
  -r eval/cpg-storage/requirements-report.txt
"$study_root/report-venv/bin/python" eval/cpg-storage/controls.py \
  "$study_root/core" "$study_root/workspace"
"$study_root/report-venv/bin/python" eval/cpg-storage/report.py \
  --producer "$study_root/producer.json" --out "$study_root/reports" \
  "$study_root/core" "$study_root/workspace"
```

For a self-check report, also pass `--capture-inventory "$study_root/capture-inventory.json"`
to `report.py` and set `input_description` in `producer.json` to describe the
route and dataset selection.

`producer.json` records source and binary provenance for the capture. Its required
fields are `branch`, `head`, and `source_tree_sha256`; the retained example includes
the dirty file inventory and frozen producer binary hashes as well. The original
JSON control writes one independent Zstandard frame per input document and checks
its decompressed SHA256. It is a **size control**, excluded from timing rankings.
The original-JSON timing rows use the Rust harness and are included in those ranks;
the Python control remains an independent check on compressed size. A second
size-only control uses one Zstandard frame over an array of all unchanged input
JSON documents, with a SHA256 check of the decompressed array. This separates
compression-boundary effects from explicit interning; no solid-array reader is
timed or included in the rankings.

## Shared representation and scope

Every shared-format codec serializes the same typed arena:

- `strings`: unique original strings, object keys, source contents, and input paths.
- `values`: records containing a kind, scalar payload, child start, and child count.
  JSON and FlexBuffers serialize those records as compact four-element tuples.
- `edges`: numeric references; object entries alternate key-string and value IDs.
- `documents`: an input path and root value ID for each original document.

Strings and JSON substructures share storage only after exact equality checks.
Hash collisions cannot merge distinct contents. All original compiler identities,
compilation contexts, source/provenance fields, array ordering, scalar kinds,
missing fields, and extension data survive. Sharing identical JSON bytes does not
assert that two source entities have the same semantic identity.

This is an explicit **generic shared JSON-value arena**, more aggressive than just
moving unit tables out of function chunks. No binary format contains embedded JSON
blobs. A dedicated typed CPG schema can produce different size and performance
results. Shared prototypes use monolithic archives and full owned decoding; they do
not measure native zero-copy queries, mmap traversal, independently compressed
function chunks, or conversion back into the production reader's types. A
production implementation also needs bounded-memory normalization and chunking.

The core and workspace datasets differ in actual source content. They are not
synthetic repetitions, and the results should not be extrapolated directly to a
previous capture whose contents have not been inventoried.
