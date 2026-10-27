# Shared dereference reads

Data-v3 3.29.0 adds `rust.shared-dereference-reads`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`.

A built-in explicit `*` may read a supported sized value through a shared reference
whose physical locations are reconstructed from formal origins. The evaluated
pointer may be a formal, a saved or rebound local, a conditional value, a stored
reference or a verified shared helper result. Nested dereferences load each stored
pointer before accessing its referent. Existing branch/loop reference flow applies.

The source registry selects the capability before HIR eligibility. Independent
source checks require original unary dereference syntax, exact evaluated operand
correspondence, built-in dispatch and matching shared referent/result types. The
existing concrete type and nominal boundaries still apply; this is not overloaded
`Deref`, raw pointer access or a general ownership model.

`data.indirect-deref` uses the same physical location/access representation as
indirect field reads. Each access has operand position zero and a physical location,
with no lexical symbol. The evaluated pointer carries its real lexical snapshot
dependencies. A dereference used as a field place uses `data.indirect-place`; the
final field read selects the complete physical path without inventing an aggregate
load. Reference-bearing loaded values derive the component origins of all possible
locations. Existing verified read-only helper summaries compose these reads.

Independent document verification reconstructs accesses, pointer dependencies,
place roles and returned origins. Strict producer verification remains required.
No capture records, source snapshots or retained caches are added.

Local/temporary storage borrows, address-taking through dereference places,
mutable-reference reads/writes, raw pointers,
unsized whole-value loads, overloaded dereference and general implicit multi-hop
field dereference remain outside this contract. Reference-bearing arrays still
require D18 per-element evidence. D14 remains Partial and the full roadmap remains
active.
