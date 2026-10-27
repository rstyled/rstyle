# Indirect shared-reference field reads

Data-v3 3.28.0 adds `rust.indirect-field-reads`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`.

Field reads may select multiple physical locations through reference rebindings,
snapshot copies, branch/loop origin unions, shared helper results and conditional
bases. Nested tuple/struct projections preserve their complete physical paths.
Reference-valued fields preserve the origins loaded from every possible location.

The implementation derives locations temporarily from existing source operations,
resolved members and reference origins. It adds no serialized capture records,
source snapshots or retained origin cache. Functions without indirect reads skip
the additional location installation.

An indirect read uses `data.indirect-field`, an evaluated pointer operand and
existing memory access records. Each access has a physical location and operand
position zero, with no lexical symbol. The pointer operand carries the actual
binding/snapshot dependencies; the read must not invent a fresh read of its
original formal. Intermediate nested places use `data.indirect-place` without
loading the aggregate or inventing a lexical binding access.

Independent document verification reconstructs the exact access list and verifies
the evaluated base correspondence, including conditional merge values. Strict
producer verification checks the same interpretation. Missing accesses, invented
lexical symbols and incorrect operand positions are rejected.

This does not introduce borrows of local/temporary storage, pointee writes,
general explicit dereference, indexing or aggregate helper results. Existing
legacy profile exclusions remain required. D12 remains Partial and the complete
feature coverage roadmap remains active.
