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
