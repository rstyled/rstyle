# Shared unit bundle version 1

The internal `rstyle.shared-unit/1` JSON bundle contains one unit's owned
frontend and graph records. It is available through
`UnitStore::{write_bundle,from_bundle}` and is the normal driver/CLI handoff. It does not replace the pinned standalone PDG/CFG
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
meaning. The CLI requests shared publication explicitly; direct legacy driver users retain
the standalone adapter.
Storage sharing does not bound legitimate pairwise matching work or optional
expanded output. Existing read/depth budgets and graph-validation checks remain
in force; further buffer/layout normalization is separate measured tuning.

`UnitStore::expanded_frontends` is an explicit standalone compatibility/debug
export, separate from shared loading. It retains depth-eight and 8 MiB per-call
bounds and applies a separate 512 MiB aggregate expansion-work budget. Exhaustion
returns an error rather than truncated evidence. The legacy mutable-facts
capture entry point now delegates to shared capture and this exporter instead
of maintaining a second certification runtime. The CLI invokes this exporter only with `--expanded-evidence`, writing separate
readable standalone pairs. Ordinary analysis never expands helper proof trees.


The driver publishes `unit.evidence.json` and a separate
`rstyle.shared-unit.manifest/1` seal containing the exact byte count and SHA256.
The seal hashes the bytes as they are written and requires successful flush.
CLI artifact loading reads each artifact once, checks the inventory and seal,
and decodes the same owned snapshot. The seal detects changed bytes; it does
not grant trust. `from_current_run` explicitly selects existing generated
structural verification only for independently selected compiler output after
checking the frozen invocation and selected-unit context. Ordinary `from_bundle`
always performs strict validation, including inside a generated ambient scope.

Report functions carry a bundle/function reference instead of a mandatory
standalone artifact prefix. Raw facts remain available for source navigation.
The report includes `unit_bundle_schema`; standalone graph schema revisions
remain unchanged. Consumers requiring individual graph artifacts must opt into
`--expanded-evidence`. The optional export obeys the separate expansion budget.

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
shape, sorted output keys, explicit optional nulls, dense handles, strong unit
identity, and `rstyle.shared-unit/1` format are unchanged. Existing map producers
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
