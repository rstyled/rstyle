# Field place and value context, version 1

Implementation baseline: `9305432`. This is a semantic correction to the local
`hir-data-v1` field contract, not a change to R0/expanded/helper admission or the
matching algorithm. Earlier disclosed Data fixture vectors remain historical
measurements; corrected measurements must be recorded separately. Passing earlier
fixtures is not authority for inventing aggregate loads or omitting pointer reads.

A field expression used as the base of a further projection computes a place
when its resolved value type is a non-reference aggregate. Retain that source/HIR
projection as `data.place`, with exact location and one base operand, but no value
read, symbol read, write or definition. A terminal field in value context retains
its actual field read. Returning an aggregate field by value reads that field;
projecting through it to a nested field does not read the entire aggregate.

A stored shared-reference field used as a projection base must instead load its
reference value. For `self.link.x`, where link is `&Inner`, the link operation
reads its own exact field location before x is projected through its referent.
For `&self.link.x`, link still has that read; x computes a place and the borrow
returns its address. Loaded reference origins append SharedReferent; borrowing
field storage does not. Existing non-reference-bearing borrow-target restrictions
remain. No field projection is erased, inlined or normalized.

Consecutive implicit shared dereferences on one base, such as field access through
`&&Inner`, require additional referent-storage reads not represented by this slice.
They remain unavailable for field projection until an independently verified
implicit dereference contract exists. Opaque nested shared-reference identities
and outputs that do not project through them retain their existing support.
References stored in intermediate fields are supported only with the explicit
source-backed field-value read described above. No mutable/raw/overloaded
dereferences, interior mutation or disjointness assumptions are added.

Independently derive place/value membership from original field correspondence,
resolved base types and parent operand relationships, not supplied node markers.
Existing source/field/location proof must succeed first. Serialized verification
checks actual place/read roles, exact registered locations, operand positions and
value edges, required reference-value reads and origins, and the absence of
invented aggregate reads/writes. Source-derived membership must agree with every
serialized place marker. Preserve CFG operations, execution edges, fixed-point
reaching definitions and divergence policy. No new panic/exit edge is introduced.

The existing OTHER/CALL projection and nine-feature machinery are unchanged.
Reference counts include actual binding/field reads, not aggregate place
projections. Eliminating an invented read also removes its invented reaching-
definition edge; this is an effect/dependence correction, not threshold tuning or
canonicalization. Existing source/type/field budgets remain exact and never
truncate. Corrupt or inconsistent evidence fails closed.

Independent real Cargo fixtures must distinguish aggregate and reference-valued
projection bases, terminal aggregate reads and stored reference loads, normal
field values and borrowed outputs. Include tuple reference fields, shared aliases,
conditional borrowed outputs, marker/read/location/operand tampering, and explicit
negative consecutive implicit dereferences. Recheck all connected Data families,
unchanged historical profiles, exhaustive pairs, trace invariance and closed
serialized schemas before claiming this correction complete.
