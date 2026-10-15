# Hash purposes and boundary ownership

Historical audit at `9cb2b32`. The implementation work in
[hash-and-matching-plan.md](hash-and-matching-plan.md) supersedes its retention
recommendations; the [version 2 library contract](../extraction-profiles/read-only-library-resolution-v2.md)
retires runtime library artifact fingerprinting.

This audit follows executable fingerprint retirement. It covers the remaining
SHA-256 producers and their consumers in the Rust analysis path; fixture/test
hashes and Python experiment snapshots are called out separately. It does not
change matching hashes (WL colors), compiler crate hashes, or legacy style cache
invalidation. None of these checksums authenticates an artifact against someone
who can replace both data and checksum.

## Bundle transport: consolidated

Previously one bundle was hashed while writing, reread and hashed for the unit
inventory, read and hashed against that inventory, and hashed again against its
manifest during current-run import. Now:

1. `write_sealed_bundle` records the exact bytes successfully written and checks
   final flush before returning the authoritative manifest.
2. `artifact_inventory` copies that seal's digest into the existing bundle entry.
   It does not open the bundle. Manifest and conditional-source inventory files
   retain their ordinary inventory checksums.
3. `load_inventory` reads each artifact into an owned buffer, enforces the exact
   expected file set, rejects duplicates/paths, checks auxiliary file digests and
   requires the bundle entry to agree with the manifest. `verify_owned` checks
   schema, length and SHA-256 of the bundle buffer once.
4. A `VerifiedBundle` has private storage and exposes only borrowed bytes.
   `from_verified_current_run` decodes those same bytes, without another digest.
   It still inspects selected-unit context and validates structure and evidence.

The token establishes transport consistency only. It confers no strict proof or
producer authentication. Existing `from_current_run` still verifies its borrowed
bytes for callers without a verified snapshot; `from_bundle` remains a strict
import regardless of ambient generated policy. Run ledgers, selected invocation,
profile association, source correspondence, graph endpoints, call arguments,
types, paired documents, completeness and budgets are unchanged.

The intentionally removed guarantee is a *second independent producer-side
observation* of the bundle after publication. If the bundle is removed or edited
between publication and inventory creation, inventory creation no longer catches
it immediately; consumer snapshot verification rejects it before decoding or
accepting the handoff. No boundary is authorized using filesystem metadata.

## Unit identity: retained, allocation removed

`unit_store::unit_identity` identifies normalized frontend functions, not transport
bytes. `Builder::new` computes it once when creating an immutable store. It is
serialized in the bundle and in frontend references (`unit.function-reference/1`),
and checked while importing in `Import::into_store`. Borrowing the store or a
function does not recompute it. A new store imported from disk recomputes it once;
that is a distinct boundary, not a repeated lookup. Replacing it with a bundle
checksum would introduce a circular dependency and change reference semantics.

Serialization now writes directly into SHA-256 instead of allocating a temporary
JSON vector. Exact serialized bytes and resulting identity are unchanged.

## Remaining SHA-256 inventory

| Producer / check | Consumer and purpose | Disposition |
| --- | --- | --- |
| driver `hir_extraction::evidence`, source capture and nominal census | Source spans, function bytes, declarations and source inventory; independent source correspondence during import | Retain exact content identity. Full-file, function-range and declaration-range identities refer to different bytes. |
| driver `concrete_types` file/declaration/manifest digests | `concrete_types`, `declaration_origins`, `nominal_binding`, nominal boundary census | Retain persistent nominal/source/build association. File/declaration checks across multiple nominal records can overlap; future consolidation should use a source owner and typed range handles, not a global digest cache. |
| driver `library_artifacts::hash_stream` | `data_library::verify_evidence` binds reviewed core definitions to captured compiler libraries | Retain artifact identity and byte budgets. Capture deduplicates callees by compiler crate identity before collecting library records. This hashes library artifacts, not CLI/driver executables. |
| CLI `clones::digest`, capture manifest identity | Nominal selected-package evidence | Retain source/build-context association. |
| CLI capture unit-directory and ledger keys | Run/unit ledger routing and duplicate-unit detection | Retain deterministic identity; not a content-verification cache. |
| CLI assessment identity (`function_key`) | Matching/report keys for exact evidence and compilation contexts | Retain external identity. Not a substitute for graph validation. |
| CLI artifact digests | Complete handoff inventory | Retain auxiliary checks; consolidate bundle as above. |
| `unit_store::manifest` | Exact published bytes and immutable imported buffer | One streaming producer checksum and one consumer verification. |
| `unit_store::unit_identity` | Persistent frontend references | Retain once per construction/import; stream serialization. |
| `producer::emit` serialized document identities | PDG/CFG byte manifests from the standalone serialized-pair API | Retain checksums for that transport boundary. The shared bundle uses its own seal and does not call this serializer. |
| `producer::verify` source digest | Strict standalone/shared evidence reconstruction | Retain strict-boundary check; generated verification already skips reconstruction. |
| `validate::check_source_digests` | Public compiler-evidence document validation against supplied source bytes | Retain public-boundary validation. A caller's declaration that bytes are immutable is not proof that a claimed digest matched them. |
| `field_locations` source check | Source-backed field observation proof | Retain independently callable verification. Overlap with producer/document verification is a candidate for a future owner-scoped verified-source token. |
| `concrete_types::digest`, `nominal_binding::build_digest` | Nominal declaration proof, function evidence and selected build compatibility | Retain current public checks. Repeated canonical build serialization is another consolidation candidate; no weaker metadata reuse introduced. |
| `data_documents` location ID | Persistent projected location identity | Retain; distinct from source/unit/transport identity. |
| `diagnostics::Observation` evidence digest | Opt-in performance event grouping | Only collected when diagnostics are enabled; no ordinary-path digest introduced. |
| `call_summaries` pinned contract digest | Frozen code-owned contract registry | Retain frozen contract identity. |
| extraction fixtures and test digest helpers | Pinned source packs, independent identity oracles and corruption regressions | Retain; outside ordinary analysis. |
| Python study snapshots / raw evidence digests | Exact experiment drift checks and recorded evidence identity | Retain independent snapshots. No report version substituted for binary bytes. |
| Python CodeNet SHA-1 helpers | Deterministic sampling order, normalized-source duplicate grouping, and sample inventory | Outside clone analysis and checksum trust; retained as experiment bookkeeping. |

There are still overlapping *source* checks in independently callable validators.
They are identified here rather than removed through an unchecked flag. A later
change can bind successful source/range/build verification to the immutable owner
and thread typed handles through those validators. Existing analysis-local proof
and derived-result stores already scope successful reuse by owner and policy;
this change does not extend generated proof into strict import.

## Validation

Regression tests cover publication inventory without a bundle reread, seal schema
and length, altered bytes, changed inventory digest, immutable snapshots surviving
file replacement, and selected-unit inspection still rejecting a verified snapshot.
Existing partial-write/final-flush, corruption, stale/deleted source, strict-import
and matching tests remain. See the executable retirement measurement for the
separately measured first increment; no bundle speedup is claimed here.

Final checks: 645/645 nextest tests passed (39.480 s), doctest wrapper passed,
24/24 Python bookkeeping tests passed, formatting and workspace/all-target Clippy
with `-D warnings` passed. Both style gates reported zero violations/actionable
candidates; the 15 existing reviewed ignores are unchanged. An earlier run caught
a spacing violation, fixed before the final run. Another run overlapped a style
build that replaced the CLI during wrapper execution; final validation ran without
concurrent tool builds.
