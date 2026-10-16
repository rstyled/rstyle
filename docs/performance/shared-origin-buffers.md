# Shared original-file buffers

Following `5444e70`, census `UnitFile.bytes` and declaration `Origin.file_bytes`
use immutable `Arc<str>` buffers. During unit normalization, after census owners
are canonicalized by complete context/record equality, each declaration resolves
its exact source path and compares complete bytes and digest claims. Matching
records borrow the census buffer; contradictory origin records fail. No hash
alone establishes equality. Nominal binding's source table also retains shared
buffers rather than cloning entire files.

This shares byte storage, not verification authority. Normal source, declaration,
range, module, package and strict/generated checks still apply. A matching digest
claim need not be valid until checked by the existing boundary validator. The
owner is immutable; compatibility JSON edits detach evidence and normalization
rechecks exact equality. Separate units retain separate owners. Private checked
source handles retain the previous environment-local scope.

The JSON schemas and byte encoding are unchanged. Rust constructors now accept
`Arc<str>` rather than `String` for these two fields; use `.into()` when moving a
string. Edits replace the immutable snapshot and cannot mutate another owner.
Persistent digest meanings and all analysis budgets remain unchanged. Wire JSON
still repeats the original buffers; a source dictionary and checked function
ranges remain future work. Capture still reconstructs/hashes origin files before
unit normalization; this increment does not claim to eliminate that work.

Structural coverage checks buffer address sharing after exact equality,
byte-identical wire output, detached compatibility edits, and rejection of
mismatched path/bytes/digest claims. Existing tests edit snapshots by replacement,
retaining all corruption, source edit, UTF-8/BOM/CRLF and module checks. There is
no wall-time speedup claim for this storage increment. Earlier matched pipeline
measurements remain attributable only to their recorded source revision.

Validation: 66 library unit tests passed before the final helper extraction;
final warning-free workspace/all-target Clippy and full pinned nextest wrapper
passed 656 tests, zero skipped, followed by workspace doctests. Formatting and
diff checks passed. Both final gates passed: zero violations (2,005 functions),
zero actionable pattern candidates, 15 existing ignores unchanged. All 24 Python
study tests and two measurement-harness tests passed. Initial compile/type and
style failures were retained in `/tmp/rstyle-source-buffers-*` logs and fixed;
no assertions or quality thresholds were weakened.
