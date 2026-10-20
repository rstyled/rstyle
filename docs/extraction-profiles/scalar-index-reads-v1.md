# Scalar index reads

Data-v3 3.30.0 adds `rust.scalar-index-reads`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`.

Built-in indexing admits supported scalar elements of owned arrays and shared
arrays/slices. Independent source checks require exact base/index correspondence,
built-in dispatch, a usize index and matching element/result types. Existing
concrete type and nominal boundaries apply.

`data.value-index` is a computation over an owned array. `data.index` is a physical
read through a shared reference, with no lexical symbol or reaching-definition
edge on the memory read itself. Both retain base operand zero and index operand
one. Local paths and pure field/dereference place chains materialize their base
value after evaluating the index, as Rust does; computed base values retain their
earlier snapshot. Reference flow uses the same ordering.

The `IndexedElement` projection selects operand one of the accessing read.
`rstyle.formal-indexed-element/1` locations are parameterized templates, not a
claim that two reads select equal or disjoint elements. Separate read nodes retain
separate index snapshots and count as separate indexed accesses even when sharing
one template location. Existing formal alias uncertainty remains applicable.

Every index node carries `rstyle.data:index-bounds`: rule
`index-less-than-length`, base operand zero, index operand one, the known array
length or null for a slice, and exceptional flow `omitted`. This records the
normal-completion requirement; it does not prove an index is always in bounds.
General exceptional flow remains separate work.

Independent document verification reconstructs locations, bounds and ordered value
operands; strict producer verification remains required. No capture records,
source snapshots or retained caches are added. Read-only helper results may serve
as bases, but indexed helper bodies remain excluded because existing summaries
cannot transport bounds or claim nonpanicking behavior for these operations.

Aggregate/reference-bearing elements, local storage borrows, writes, ranges and
subslices remain outside this contract. D15 is Partial; the full roadmap remains
active.
