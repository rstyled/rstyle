# Immutable shared-reference locals

Data-v3 extraction 3.20.0 adds `rust.shared-reference-locals`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`. Initialized immutable bindings of shared-reference
type can carry references from the existing supported formal, field, borrow,
conditional and helper-result operations. Aliases, shadowing, returns and registered
slice/string metadata calls compose with this value flow. This capability does not
create storage locations for locals or authorize borrowing local storage.

Source origin reconstruction resolves the exact existing local binding identity,
finds its original initializer, requires the same concrete type and requires the
read to follow the complete initializer. Recursive alias resolution therefore
strictly decreases source positions. The binding must be immutable. Existing
source/HIR correspondence and lexical-binding checks remain in force. A conditional
initializer retains the union of its alternative formal origins, while the graph
retains its original control flow and alternatives.

No new producer operations, frontend observations, provenance records, source
snapshots or retained caches are added. Existing declaration value operands and
binding reaching-definition edges carry the reference value. Origin reconstruction
uses the existing origin annotation. Serialized origin verification follows a
binding definition through its unit-typed declaration to the reference initializer;
it continues to compare the graph origins with the independently reconstructed
source origins. The existing whole-local verifier checks the original typed
initializer, exact binding identity and CFG kill/gen reaching definitions.

The real Cargo fixture covers direct aliases, alias chains, shadowed bindings and
saved aliases, conditional initializers, a stored reference field, a supported
formal-field borrow, slice length and string emptiness. Metadata-call admission
continues to require the existing target-specific library authority. Every admitted
pair passes independent document and strict producer verification. Equivalent
functions reach accepted evaluated matching with score 1. Removed reaching-definition
edges and altered serialized origins fail both verifiers. Legacy Data-v2 retains
its local-reference exclusions.

Mutable reference bindings and reference reassignment remain excluded, as do
reference-bearing aggregate locals and field projections through shared-reference
local roots. Mutable-reference types, explicit dereferences, local/temporary borrows,
element/subslice origins and ownership/drop remain separate roadmap work. D12 stays
Partial and the full roadmap remains active. Frozen inventory and final validation
results are recorded in the roadmap continuation log.
