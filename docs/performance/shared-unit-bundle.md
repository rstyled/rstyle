# Shared unit bundle version 1

The internal `rstyle.shared-unit/1` JSON bundle contains one unit's owned
frontend and graph records. It is available through
`UnitStore::{write_bundle,from_bundle}`; driver/CLI publication is the next
integration increment. It does not replace the pinned standalone PDG/CFG
schemas or reinterpret their existing manifest hashes.

| Field | Meaning |
| --- | --- |
| `schema` | Exact versioned bundle discriminator |
| `identity` | SHA256 of serialized raw frontend facts including build context |
| `functions` | Raw frontend records, without attached nested helper proofs |
| `graphs` | Unique typed PDG/CFG pairs |
| `roots` | Optional graph handle for each function |
| `helpers` | Function/graph handles, typed summary and bounded `u32` depth |
| `links` | Per-function call occurrences with HIR site and target handle |
| `certified` | Optional helper handle for each function |

`FunctionId`, `HelperId` and `GraphId` are distinct private-constructor `u32`
newtypes. Every index/count conversion is checked. IDs preserve their referents
through a bundle round trip; they are scoped to that immutable owning store.
They are not globally persistent identities and are not promised to survive
insertions, edits or renumbering. Equivalent explicit renumbering can preserve
graph meaning when all references are updated. Logical definition identity
remains in the compiler evidence, distinct from exact unit evidence identity.
The current unit hash includes function record order and therefore changes if
records are reordered. No unchecked truncated hash or persistent ID registry
is used.

A function root and its helper certificate reference the same graph handle;
they do not carry duplicate paired documents. Call metadata uses local helper
references, qualified by the containing unit/function frontend reference.
Standalone verification cannot resolve a shared-table reference. Readable
names, source buffers, byte ranges, snippets and expanded provenance remain
available through borrowed function views and paired graph records. Bundle
writing streams typed records directly without expanding nested proof trees.
It currently materializes only small call-link ID lists for JSON transport.

Import decodes one supplied owned byte snapshot into typed tables, then moves
those tables into the store. It recomputes raw evidence identity and call links,
checks all bounds, duplicate sites, root/certificate agreement, unused graph
records, inventory completeness and paired-document correspondence. Only the
helper certification relation must be acyclic and fit the existing depth-eight
budget; legitimate CFG cycles retain their existing policy. Graph and source
validation is strict even if import is invoked inside a generated scope.
Unknown serialized trust flags are rejected by closed record schemas. No
serialized ID or flag confers a verified capability.

Successful proof caches are separate analysis contexts; they are neither
serialized nor retained as generated trust after import. Per-use signature,
dispatch, argument and effect checks remain. An imported graph is deserialized
once into the graph table; roots and helper records subsequently borrow it.
This is ordinary owned deserialization, not zero-copy loading.

The bundle is an explicit new format. Legacy standalone facts/pairs remain a
narrow supported adapter, and legacy canonical byte manifests keep their
meaning. The current driver/CLI still use that adapter until the publication
increment; the shared bundle must not yet be described as their ordinary path.
Storage sharing does not bound legitimate pairwise matching work or optional
expanded output. Existing read/depth budgets and graph-validation checks remain
in force; further buffer/layout normalization is separate measured tuning.

`UnitStore::expanded_frontends` is an explicit standalone compatibility/debug
export, separate from shared loading. It retains depth-eight and 8 MiB per-call
bounds and applies a separate 512 MiB aggregate expansion-work budget. Exhaustion
returns an error rather than truncated evidence. The legacy mutable-facts
capture entry point now delegates to shared capture and this exporter instead
of maintaining a second certification runtime. Driver/CLI adoption remains
outstanding, so their current legacy publication still incurs expansion costs.
