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

## Memory

Rust serialization writes into the compressor without an intermediate whole-JSON
byte buffer. Reading decompresses into the JSON parser without materializing the
entire decoded byte stream. The owned document/analysis output still occupies
memory. Stored seals are verified before decompression and checked again while
reading, and decoded size/checksum are checked as parsing completes.

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
