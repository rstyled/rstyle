# CPG document storage

The document schema stays `compat.cpg/0.1.0`. Storage encoding is independent of
the language frontend and of semantic entity identity. Native writers and the
streaming helper produce the same file format.

Each `.cpg.json` contains one UTF-8 JSON document. Each `.cpg.json.zst` contains
one standard Zstandard frame whose decoded bytes are that same JSON document.
Documents remain individually addressable. No custom compression dictionary is
required. Rust writes level 3 with a frame checksum; readers support streaming
frames that omit the decoded size from their frame header. Compression level and
implementation do not need to match across languages. Readers reject Zstd
windows larger than 128 MiB.

Native Rust captures and `cpg emit` default to Zstandard. Pass the global option
`--cpg-compression none` to write plain JSON. Both forms work with `cpg check`,
`cpg patterns`, and saved-run replay. Legacy plain captures remain readable.

## Sealed captures

`documents.manifest.json` uses schema `rstyle.cpg-documents.manifest/2`:

```json
{
  "schema": "rstyle.cpg-documents.manifest/2",
  "documents": [{
    "file": "document-000000.cpg.json.zst",
    "encoding": "zstd",
    "bytes": 123,
    "checksum": "sha256:<64 lowercase hex digits>",
    "decoded_bytes": 456,
    "decoded_checksum": "sha256:<64 lowercase hex digits>"
  }]
}
```

`encoding` is `json` or `zstd`. `bytes` and `checksum` seal the **stored file**;
`decoded_bytes` and `decoded_checksum` seal the **exact original UTF-8 JSON bytes**,
including whitespace. These are byte consistency checks, not authorization or
compiler evidence identity. Recompression can change the stored checksum while
preserving the decoded checksum. Different JSON serializers need not produce
identical bytes; their documents must conform to the same schema.

Names are unique single filenames, never paths. Their suffix must match the
declared encoding. Documents are listed in input order. The manifest is published
only after every document has completed. A failed producer must return failure
and must not publish a complete manifest. Writers stage files beside their
destination and publish after successful encoder finalization and file close.
Completed document files are never overwritten. This is atomic publication, not
a guarantee of crash durability after power loss.

The Rust reader also accepts schema `/1`: plain JSON with stored `bytes` and
`xxh3-64` checksums and no encoding/decoded metadata. New writers emit `/2`.
Standalone unsealed document files remain valid inputs. When `cpg check` or
`cpg patterns` finds a neighboring manifest, the requested document must be
listed and both seals must match. Compiler capture also checks the manifest
against its existing compilation-unit ledger.

`cpg emit --out DIRECTORY` refreshes standalone exports: it removes previous
`.cpg.json` / `.cpg.json.zst` files and any previous `documents.manifest.json`
before compilation. It writes unsealed document files; successful compiler
captures, native Go emission, and helper ingestion publish sealed manifests.

## Receiving a completed capture

Pass the capture directory to the Rust consumer after the producer exits
successfully or the complete directory has been transferred:

```sh
cargo-rstyle rstyle cpg receive --capture ./capture --format json
cargo-rstyle rstyle cpg check --capture ./capture --format json
cargo-rstyle rstyle cpg patterns --capture ./capture --complexity --format json
```

`--capture` reads every document listed by `documents.manifest.json`, in manifest
order. It requires a nonempty, supported manifest, reads it once, and verifies
each document's stored and decoded seals. Missing or corrupt listed files reject
the capture. Unlisted files are outside that manifest snapshot. The existing
explicit document arguments remain available; they cannot be mixed with
`--capture` in one invocation. Standalone `cpg emit` exports have no manifest and
continue to use explicit document arguments.
Compiler saved-run roots continue to use `--replay-run`; `--capture` names one
manifest-bearing document directory, rather than recursively opening a run root.

`cpg receive` checks CPG profile/layout, unit facts, split-document pairing, and
duplicate function ownership without running style evaluation or pattern mining.
It discards derived records after each bundled unit. Its JSON result is a receipt:

```json
{
  "schema": "rstyle.cpg-receipt/1",
  "status": "accepted",
  "capture": "./capture",
  "validation": {
    "profile": "compat.cpg",
    "version": "0.1.0",
    "graphs": false,
    "runtime_bounds": false
  },
  "transport": {
    "manifest_schema": "rstyle.cpg-documents.manifest/2",
    "manifest_checksum": "sha256:<64 lowercase hex digits>",
    "expected_documents": 2,
    "verified_documents": 2,
    "stored_bytes": 272,
    "decoded_bytes": 2000
  },
  "failures": []
}
```

The manifest checksum covers its exact bytes at open time. Transport totals count
only successfully decoded and seal-verified documents. They do not claim that the
CPG contents of those documents passed semantic validation: a document with a
valid checksum and unsupported profile still produces a rejected receipt.
Validation fails on the first error; its message is retained in `failures`.
`transport` is null when the manifest could not be opened or accepted.

Acceptance is reported only after the entire input and requested CPG validation
succeed. `receive` exits 0 for acceptance and 2 for rejection. `check` and
`patterns` include the receipt under `capture_receipt` in a successful JSON
analysis report; its validation fields reflect whether graphs and runtime bounds
were requested. Rule findings or candidate findings can produce exit 1 while the
capture remains accepted. Capture-input rejection returns a standalone rejected
receipt and exit 2, with no analysis report. Other command/configuration errors
retain the ordinary CLI error behavior.

Acceptance describes the listed documents and the requested input validation.
Compiler selection/coverage guarantees continue to come from the compiler
capture's ledger. The receipt is a verification result, not a signature or an
authorization token.

Rust library consumers can use `cpg::transport::CaptureReader`. Iteration reads
generic JSON values; `next_document::<T>()` supports typed callers. `progress()`
reports verified totals, and `complete()` refuses a partially consumed or failed
capture. Callers still validate CPG contents before accepting it. `cpg::visit_records`
derives records incrementally and supports consumers that discard each batch.
Its callbacks are provisional until the whole call succeeds.

## Receiving a local pipe

Use an uncompressed stdin pipe when a frontend and the analyzer run on the same
machine. This skips capture files, manifests, and both Zstandard operations:

```sh
cargo-rstyle rstyle cpg receive --stdin --format json
cargo-rstyle rstyle cpg check --stdin --format json
cargo-rstyle rstyle cpg patterns --stdin --complexity --format json
```

These commands read the same JSON text sequence as `cpg ingest`:

1. Write byte `0x1e` (RS), then a JSON object beginning with `{`, then LF.
2. Repeat for every document, writing each object's UTF-8 bytes in arbitrary chunks.
3. Only after generation succeeds, write RS, `null`, LF, and close stdin.
4. Read the result from stdout and wait for the receiver's exit status. Diagnostics
   use stderr. Style findings can return exit 1 with an accepted stream receipt.

An ordinary single JSON document, concatenated JSON objects, and compressed bytes
are not this protocol. Select exactly one input source: `--stdin`, `--input-fd N`,
`--capture`, or explicit document paths. The global `--cpg-compression` option controls saved
output; it does not change stdin's uncompressed encoding. Logs belong on stderr,
not in the document stream. Pipes provide backpressure when the receiver falls
behind; producers should honor blocked writes rather than queue the whole capture.

The [Python example](../examples/cpg-helper/pipe.py) streams existing plain files
with bounded copying buffers. From the repository root, this is a runnable smoke
test; a frontend can instead write its JSON directly to the same pipe:

```sh
set -o pipefail
python3 examples/cpg-helper/pipe.py \
  crates/rstyle-core/src/cpg/testdata/example.cpg.json \
  | cargo-rstyle rstyle cpg check --stdin --format json
```

No byte lengths or checksums need to be calculated before sending. Objects may
contain escaped `\u001e`; a raw RS inside JSON is invalid. Each object ends at the
next RS or EOF, so a producer must send the next separator for the receiver to
finish that record. Empty streams, missing completion markers, read errors,
malformed JSON/UTF-8, trailing non-whitespace after terminal null, and invalid CPG
contents reject the run. The receiver never accepts a valid prefix. A producer
failure closes stdin without the terminal record. Frontends must also check their
own generation result; a terminal record is their declaration of successful
completion, not evidence of compiler coverage.

Standalone `receive` results use a separate schema, `rstyle.cpg-stream-receipt/1`:

```json
{
  "schema": "rstyle.cpg-stream-receipt/1",
  "status": "accepted",
  "source": "stdin",
  "validation": {
    "profile": "compat.cpg",
    "version": "0.1.0",
    "graphs": false,
    "runtime_bounds": false
  },
  "transport": {
    "framing": "rfc7464",
    "encoding": "json",
    "received_documents": 2,
    "received_bytes": 2009,
    "complete": true
  },
  "failures": []
}
```

The byte count includes consumed separators and whitespace, including the terminal
record. The document count includes only records that were fully read and parsed;
their CPG validation can still fail. On early rejection these are consumed-prefix
totals, not the total bytes the producer attempted to send. `complete` means the
transport reached terminal null and EOF; acceptance additionally requires CPG
validation, including deferred split-document validation. There are no manifest
seals or independently verified expected totals. File receipt schema
`rstyle.cpg-receipt/1` remains unchanged.

As with directory input, `check` and `patterns` embed the receipt under
`capture_receipt`, and an input rejection returns only a rejected receipt with
exit 2. `receive` validates style inputs without evaluating style rules or mining
patterns. All results are final only after the full stream and requested CPG
validation succeed.

Rust embedders can use `cpg::transport::SequenceReader::new(bufread)` and pass it
to `cpg::visit_records`. `next_document::<T>()` supports typed parsing, and
`next_with` supports consumers such as the storage helper that copy bytes directly
into an encoder. Call `complete()` after iteration; callbacks and derived batches
remain provisional until the complete stream and CPG validation succeed.

### Choosing an inherited descriptor

On Unix, `--input-fd N` reads the same uncompressed sequence from an already-open,
readable descriptor inherited from the launcher. It is available on `receive`,
`check`, and `patterns`. This leaves stdin available for a different purpose;
stdout still carries the receiver's receipt or analysis report, and stderr carries
diagnostics. `--stdin` remains the portable choice across platforms.

For example, Bash can hand a pipe to fd 3 and reset stdin to `/dev/null`. Redirection
order matters: duplicate the pipe before replacing stdin.

```sh
set -o pipefail
python3 examples/cpg-helper/pipe.py \
  crates/rstyle-core/src/cpg/testdata/example.cpg.json \
  | cargo-rstyle rstyle cpg check --input-fd 3 --format json 3<&0 </dev/null
```

The receiver creates a close-on-exec duplicate and owns only that copy. It keeps
the supplied descriptor open; both descriptors refer to the same stream position.
Use a blocking descriptor and give the stream to one consumer. `--input-fd 0` is
also supported. A closed descriptor or a read failure returns a rejected receipt
with exit 2. Descriptor receipts retain `rstyle.cpg-stream-receipt/1` and identify
the input with `"source": "fd"` and `"input_fd": N`; transport is null if the
descriptor could not be duplicated. Stdin and saved-capture receipt shapes stay
unchanged. Negative or out-of-range numbers and conflicting input flags are CLI
argument errors.

Choosing an fd number does not create or transfer the pipe. A Rust supervisor
launching Go creates the pipe and arranges inheritance of its write end at the
frontend's chosen descriptor. Rust reads the other end directly with
`SequenceReader`; the descriptor numbers in the two processes need not match.
Close unused pipe ends in both processes so EOF reaches the reader.
Go can wrap its inherited write descriptor with `os.NewFile` and stream JSON to
it, keeping stdout for other output. The supervisor consumes the stream while Go
runs and requires both successful stream/CPG validation and a successful Go exit
before accepting the overall run. The receiver CLI alone does not observe the
producer's exit status.

## Memory

Rust serialization writes into the compressor without an intermediate whole-JSON
byte buffer. Reading decompresses into the JSON parser without materializing the
entire decoded byte stream. The owned document/analysis output still occupies
memory. Stored seals are verified before decompression and checked again while
reading, and decoded size/checksum are checked as parsing completes.

Document-based `check` and `patterns` derive each bundled unit's records before
reading the next document. They release the raw JSON and typed unit at that point;
they retain function records, source snippets, requested graphs/bounds, and the
identities needed for cross-unit analysis and duplicate detection. `receive`
discards each derived bundled batch while retaining duplicate-detection identities.
Thus ingestion does not retain the whole capture's bundled JSON trees, but its
memory still depends on the largest unit and the analysis data it must retain.

`--stdin` and `--input-fd` use the same incremental derivation without staging any capture files.
Transport buffers have bounded size, but each parsed document still occupies
memory. Bundled units release their raw JSON before the next document is read;
`receive` discards their derived batches, whereas analysis retains required facts
and graphs. Split-layout documents remain buffered until EOF. Streaming the pipe
does not bound a frontend's own graph construction or whole-document serialization.

Split-layout documents can arrive in arbitrary order. They are currently retained
until EOF so their unit references and canonical digests can be paired; each
completed split group is then derived separately. This first implementation does
not bound the aggregate raw-JSON memory of split-layout captures. The native Rust
and Go writers produce bundled documents.

For a reproducible Linux receive-memory smoke check, run
`python3 scripts/measure_cpg_receive.py --binary target/debug/cargo-rstyle
--output NEW_DIRECTORY`. It grows the count of bundled documents while fixing
per-document source padding and measures the `receive` and `check` processes.
This isolates document retention; it is not a representative graph benchmark or
a bound on end-to-end frontend/analysis memory.

Go's native writer emits the envelope and unit arrays incrementally, using its
standard JSON encoder for each array record. Its serialization buffer is bounded
by the largest encoded record, not the entire package document. One package CPG
is built and written at a time; ASTs, type information, shared observations,
package tables, and the current package's CPG objects are separate memory costs.
Large source strings or individual function records can still buffer.

Both implementations propagate encoder, finalization, flush, and close failures.
Compatibility tests compare decoded document content and the exact bytes covered
by each seal, rather than expecting identical compressed output.

## Streaming helper for any frontend

`cargo-rstyle rstyle cpg ingest --out NEW_DIRECTORY` reads a JSON text sequence
from stdin and produces the same sealed capture as a native writer. It needs no
compiler driver. Use the installed executable as a subprocess; a frontend does
not need a Rust toolchain or language-specific FFI bindings. The global
`--cpg-compression none` option selects plain output for debugging.

The sequence follows RFC 7464 framing with an application-level terminal record:

1. Write byte `0x1e` (RS), then one JSON object beginning with `{`, then LF.
2. Repeat for each document. Stream each object's bytes in arbitrary chunks.
3. Write `0x1e`, `null`, LF, and close stdin to commit the run.
4. Wait for exit status zero. A failed producer closes stdin without the terminal
   record and waits for the helper to clean up before reporting failure.

Every input document must be an object; the terminal null is not a document.
The bytes between separators, including whitespace/LF, are preserved exactly.
A record's first byte is `{`; leading whitespace before it is not accepted.
Raw RS cannot occur in a valid JSON string. After terminal null only whitespace
and EOF are permitted. Empty runs are rejected. This is a syntax/UTF-8 transport
check; the ordinary CPG reader still validates the profile, schema and facts.

The helper stages the whole output directory beside its destination. It publishes
that directory and its complete manifest only after the terminal record and EOF.
Invalid JSON, UTF-8, missing completion, write errors, and existing destinations
fail without replacing a completed run. Abrupt termination of the helper itself
can leave its hidden staging directory; normal input failures clean it up.

The input protocol has bounded byte buffers: it does not assemble a document or
require its size in advance. Manifest entries grow with document count. The syntax
validator discards values and validates raw UTF-8 incrementally, including inside
large source strings. This does not bound memory that a frontend uses to build its
own graph or to call a whole-document serializer.

See the [TypeScript adapter](../examples/cpg-helper/typescript/capture.ts) and
[Swift adapter](../examples/cpg-helper/swift/Capture.swift). They stream input and
wait for helper completion. The TypeScript adapter accepts async iterables of
UTF-8 byte chunks for each document; the Swift adapter exposes
`beginDocument`, `write`, and `endDocument` inside `withCPGCapture`. Both leave
compression and manifest details in the helper. Callers must generate JSON
incrementally to avoid a whole-graph `JSON.stringify` or `JSONEncoder.encode`.

Run the Node adapter tests with Node's TypeScript stripping support:

```sh
RSTYLE_CPG_BIN="$PWD/target/debug/cargo-rstyle" node --test examples/cpg-helper/typescript/capture.test.mjs
```

The independent Go writer and helper share the same consumer checks. In the Go
repository, run `make cpg-transport-check RSTYLE_CPG=/path/to/cargo-rstyle
NATIVE_OUT=NEW_DIRECTORY` against the compression-aware CLI. The Go repository's
pinned legacy facts consumers remain separate from this cross-language check.
