# Shared unit bundle version 7

The internal `rstyle.shared-unit/7` JSON bundle contains one unit's owned
frontend and graph records. It is available through
`UnitStore::{write_bundle,from_bundle}` and is the normal driver/CLI handoff. It does not replace the pinned standalone PDG/CFG
schemas or reinterpret their existing manifest hashes.

Version 6 preserves shallow HIR bodies as their readable trees. Bodies reaching
32 nested HIR levels use a flat preorder array of `{node, children}` records:
each `node` retains every original field with an empty `children` array, and
the sibling `children` count supplies its original arity. Export streams borrowed
nodes; import moves the decoded nodes into the exact ordered tree. This avoids
JSON container nesting failures in wide compiler-generated derives without
copying bodies or increasing the JSON recursion limit or any evidence budget.
Missing children, extra roots, hidden nested children and unknown record fields
are rejected. Representation selection is unrelated to semantic admission.

Versions 4 and 5 remain readable with their original nested-body grammar. Flat
bodies require version 6 or 7; merely changing a schema label does not enable them
in historical formats. Shared diagnostic snapshots use
`rstyle.diagnostic.raw-facts/4` with the same body codec and still read version 3
flat bodies and version 2 trees. Neither format change grants generated trust or strict admission. Graph
and source validation and metric identities are unchanged. No new hashes,
checksums, source proofs or artifact scans are introduced.

Version 7 retains a census definition at its first function occurrence. Later
functions that borrow the same immutable census use `census_reference` with a
publication-local dense ID. Import decodes the definition once and shares its
`Arc` with subsequent owners. Separate equal census allocations remain distinct;
there is no equality scan or hash. References must follow their definition and
use the census option key. Historical shared versions 4–6 and raw versions 2–3
retain inline census decoding and reject the new reference kind. This shares
metadata in addition to file bytes, without changing source budgets or validation.

| Field | Meaning |
| --- | --- |
| `schema` | Exact versioned bundle discriminator |
| `namespace` | Local producer allocation namespace, preserved on import |
| `sources` | Immutable original snapshots referenced only inside this bundle |
| `functions` | Frontend records with checked source-range references, without nested helper proofs |
| `graphs` | Unique typed PDG/CFG pairs |
| `roots` | Optional graph handle for each function |
| `helpers` | Function/graph handles, typed summary and bounded `u32` depth |
| `links` | Per-function call occurrences with HIR site and target handle |
| `certified` | Optional helper handle for each function |

`FunctionId`, `HelperId`, `GraphId` and source-table `SourceId` are distinct private-constructor `u32`
newtypes. Every index/count conversion is checked. IDs preserve their referents
through a bundle round trip; they are scoped to that immutable owning store.
They are not globally persistent identities and are not promised to survive
insertions, edits or renumbering. Equivalent explicit renumbering can preserve
graph meaning when all references are updated. Logical definition identity
remains in the compiler evidence, distinct from the allocation namespace.
The namespace is not a content fingerprint. See [unit namespaces](unit-namespaces.md)
for allocation, owner isolation and the deliberately retired digest guarantee.

A function root and its helper certificate reference the same graph handle;
they do not carry duplicate paired documents. Call metadata uses local helper
references, qualified by the containing unit/function frontend reference.
Standalone verification cannot resolve a shared-table reference. Readable
names, source buffers, byte ranges, snippets and expanded provenance remain
available through borrowed function views and paired graph records. Bundle
writing streams typed records directly without expanding nested proof trees.
It materializes call-link IDs and borrowed function/source reference records for
JSON transport. Function source fields contain `snapshot`, `range` and the original
`digest` claim; each snapshot string is written once in `sources`. See
[source dictionary version 3](source-dictionary-v3.md) for the initial function-only format,
and [source options version 4](source-options-v4.md) for the current census/origin sharing.

Import decodes one supplied owned byte snapshot into typed tables, then moves
those tables into the store. It preserves the wire namespace and reconstructs exact call links,
resolves source ranges before frontend inspection and checks dangling/unused
source entries, UTF-8 bounds, duplicate sites, root/certificate agreement, unused graph
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

The bundle is the compiler/CLI handoff format. Standalone typed pair
serialization and strict validation remain available for interchange, but there
is no exporter that reconstructs nested helper proofs. The driver publishes
shared tables whenever a document directory is selected; the old
`RSTYLE_HIR_SHARED_UNIT` publication switch is removed. Raw facts-only driver
output contains observations without attached helper proofs, not a standalone
helper certification capability. Existing graph schemas and canonical byte
manifests retain their meaning.

`--expanded-evidence`, `UnitStore::expanded_frontends` and the mutable
`helper_calls::capture` adapter are retired. Resolve a function through
`UnitStore::analysis().function(index)` and its root through `UnitStore::pair`.
Readable pair serialization preserves references and requires the owning bundle
for helper/frontend resolution. Helper depth/read budgets and independent graph
work budgets remain; the deleted expansion path has no expansion budget/runtime.

The driver publishes `unit.evidence.json` and a separate
`rstyle.shared-unit.manifest/2` seal containing the exact byte count and an
explicit `xxh3-64` checksum. The [transport contract](transport-checksum-v2.md)
documents the retired cryptographic collision-resistance guarantee.
The seal hashes the bytes as they are written and requires successful flush.
CLI artifact loading reads each artifact once, checks the inventory and seal,
and decodes the same owned snapshot. The seal detects changed bytes; it does
not grant trust. `from_current_run` explicitly selects existing generated
structural verification only for independently selected compiler output after
checking the frozen invocation and selected-unit context. Ordinary `from_bundle`
always performs strict validation, including inside a generated ambient scope.

Report functions carry `unit_evidence` with the bundle path, local function
index and unit namespace. `artifact_prefix` and `document_manifest` are
removed; no per-function expanded files are published. The path is navigation,
not persistent identity. A fresh capture receives a new namespace even when its facts are unchanged.
Namespaces do not detect content edits; exact source and graph verification do. Raw facts remain
available for source navigation. The report includes `unit_bundle_schema`;
standalone graph schema revisions remain unchanged. This is an intentional
internal CLI/report/API break, not a new bundle schema revision.

Report function keys now follow the separately versioned
[report-local handle contract](report-function-handles.md). The report's
`unit_contexts` joins its function labels to observed compilation contexts;
these labels make no cross-report content-identity claim. The bundle namespace is separate from these report-local ordinals.

Native-width propagation memoizes successful derived results by function handle
within one owning analysis/builder. It is independent of verification policy;
proof success remains separately partitioned by policy. Errors are not cached.
The evidence-view trait is sealed so external implementations cannot substitute
helper views from another owner. Shared traversal preserves every call occurrence
while reusing the derived callee property.

Frontend data options now use `FrontendOptions` in the Rust API instead of a
plain `BTreeMap<String, Value>`. The sealed `DataEvidence` types select their
own keys and kinds: `data::<T>()` borrows interface/field records, and
`insert_data` moves compiler-created records into their owner. The JSON object
shape, sorted output keys, explicit optional nulls, dense handles and the current namespace are preserved. The typed-options change
itself did not revise the bundle format; the namespace change separately versions it. Existing map producers
can convert with `.into()`. Open/unregistered options remain JSON.

An imported interface/field option is decoded to a typed record once when its
frontend options are loaded; the intermediate JSON tree is dropped. Malformed
records retain their original JSON and decode error without conferring any
capability. Semantic identity, source, type and policy validation still occurs
at every required verification boundary. Optional `get`/index JSON access lazily
renders the requested record. Serialization renders one record at a time to
preserve canonical object-key order, without retaining another whole document.
Ordinary typed analysis neither renders nor redecodes these records.

The mutable JSON compatibility adapter invalidates a changed entry's typed
form. Unit construction normalizes those entries before freezing the owning
store. Standalone analysis of explicitly edited mutable facts may return an
owned `Cow` fallback; loaded/frozen unit analysis borrows. This adapter does
not introduce a second unit certification runtime or a serialized trust flag.

Successful nominal lexical resolutions and inherent-definition checks now live
in a fixed per-function table within each builder/analysis context. Each entry
has separate strict and trusted-generated slots. Selection uses a private
function handle from that same resolver, never a name, foreign integer or
serialized status. Errors and interrupted checks leave their slots empty.
These derived records are dropped with the context and never enter the bundle.

`EvidenceFacts::nominal_resolution` returns `ResolutionView`, and
`inherent_definition`/`inherent_calls::definition` return an optional borrowed
`Cow<Definition>` for shared functions. Standalone facts own their checked
results. `resolution::inherent_owner` returns an `OwnerView` that borrows the
checked tree through its lexical environment, retaining an owned environment
only for standalone use. This is an intentional Rust API change; the wire
format and evidence identities do not change. Caller-specific dispatch,
signature, receiver adjustment, argument, effect and source-use checks remain
per use. A generated result cannot populate or satisfy a strict slot.

`producer::PairView` borrows the typed PDG and CFG directly and supports readable
serialization without cloning either document. `Pair::as_view` constructs it;
`producer::verify_view` applies the same verifier used by the owning API. Helper,
data, receiver and library document validators accept a view or an owning pair
reference through one implementation. A view carries no trust and may not skip
paired, source, inventory or policy checks.

Standalone strict admission now verifies a `PairView` rather than constructing
an owning pair by cloning both documents. Its imported frontend record remains
alive for the complete admission operation, including target compatibility,
instead of being decoded again. Data-location inventory comparison also borrows
existing locations while preserving duplicate-ID and exact-evidence checks.

Readable matching reports serialize borrowed typed records directly, filtering
nontrace histograms during output. Optional WL partition exports remain typed.
JSON object member order follows the exporter; field values, ordered arrays,
status precedence and nullable decisions retain their existing meaning. This
readable presentation change does not alter evidence hashes or bundle format.

File buffers now use `SourceSnapshot`, sharing their lazy actual content digest
across census, origins and function ranges. Imported owners start with empty
memos. See [snapshot identities](source-snapshot-identity.md). This changes no
wire strings, stored digest claims or verification authority.

CFG content now uses checked `SourceText` ranges. Equal imported content shares
the owning function range after structure checks, preserving all claims and
subsequent semantic verification. See [CFG source ownership](cfg-source-owner.md).

Graph producer configurations now carry the closed `rstyle.matching-build/1`
record containing only overflow, debug-assertion and panic-strategy facts. Full
build/source evidence remains in the owning frontend. No old `captured_build`
runtime fallback remains. See [matching build metadata](matching-build-metadata.md).

Concrete struct/field source text now uses checked immutable ranges; exact
origin bytes and full nominal keys select storage sharing, while semantic
validation stays separate. See [declaration ranges](declaration-ranges.md).

## Census metadata reuse

The version-7/shared and version-4/raw change keeps first-use census definitions
inline and replaces repeated occurrences of the same immutable owner with dense
back-references. It reuses the existing option-bundle inventory lifetime and
source dictionary. Registration holds borrowed owners; lookup uses allocation
identity, without rendering JSON, comparing full files or hashing. Import retains
the decoded census entry and clones its `Arc` for subsequent references. Missing
and malformed original options do not consume IDs. The importer still checks
source references, and semantic ownership checks remain with their consumers.

The [C61 samples](census-transport-samples.json) use the original rstyle-core
library check capture and the same explicit source policy as C60. Compiler advice,
source assessments, guard plans and strict execution status are exactly equal.
The 1,215-function payload has one census definition and 1,214 references, reducing
44,285,215 bytes to 37,819,451 bytes. The inline comparison reconstructs the same
snapshot's prior raw3 representation; it is not a recapture or another policy.

A prebuilt debug `census_replay_samples` probe measured three decodes per process,
inline first then shared. Median decode time was 0.459s versus 0.357s; peak process
RSS was 208,448 versus 159,440 KiB. Input reads and owner counting are outside the
decode timer. This single ordered comparison is a transport observation, not a
whole-analysis speedup claim. The fresh capture took 7.19s and remained incomplete
with exit 2. Historical inline snapshots retain separate recorded owners on load;
they are not merged by content equality. Source budgets and strict clone matching
are unchanged.
