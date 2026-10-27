# Bounded shared subslice views

Data-v3 3.31.0 adds `rust.shared-subslice-views`. Matching remains
`rstyle.ccgraph.data-calls/1.0.0`.

The initial contract admits exclusive, explicitly bounded `&base[start..end]`
views of shared slices with supported scalar elements. Bases may be saved or
rebound references, nested views or existing verified shared helper results.
Arrays, open/inclusive ranges, str/UTF-8 views, mutable views, local-storage borrows and arbitrary
Index implementations remain outside this contract. Authority is registered for
the pinned aarch64 compiler/core artifact; other targets remain unavailable.

Original source must contain range-index syntax with both usize endpoints and
exact base/range/endpoint correspondence. The captured overloaded Index and core
Range definition identities must match the pinned core artifact. The original
range carrier and expansion observation remain in capture, but no general Range
value support is implied. Graph lowering evaluates the base, start and end once
in that order and retains them in operand positions zero, one and two. The
range carrier is verified structural syntax; it is not lowered as a fabricated
unit-valued constructor. A subsequent shared borrow retains its exact place
operand and type.

`data.subslice-place` is a computation of a view, with no invented byte read,
write or lexical definition. Its `rstyle.data:subslice-bounds` annotation records
`start-less-or-equal-end-less-or-equal-length`, the three operand positions and
exceptional flow `omitted`. It is a normal-completion requirement, not an assertion
that every input is in bounds. Base snapshots are preserved even when endpoint
expressions rebind the original reference. This differs from built-in scalar
indexing and is established by an independent Rust runtime/MIR oracle in
`eval/coverage-development/subslice-order.rs`.

Reference origin sets retain the possible formal roots. Exact view offsets and
lengths remain in graph dependencies and bounds metadata, including through saved
and nested views. Subsequent scalar element reads use the existing access-relative
indexed location templates; their evaluated pointer is the view, not an assertion
of an absolute formal offset or alias independence. This does not add a location
for constructing a view. Existing helper *results* may be bases; helper bodies
containing view construction remain excluded because current summaries cannot
transport their bounds and failure behavior.

Independent document checks reconstruct the view inventory, effects, bounds,
ordered snapshot operands and borrow dependencies. Strict producer replay remains
required. Missing bounds, duplicated endpoint roles and invented reads are
rejected. No new capture record type, source snapshot or retained cache is added.

This advances the subslice portion of the safe lz4_flex byte-reader chain. Concrete
TryInto/Result/unwrap, native-byte conversion and the native-width local constant
remain unfinished. D16 is Partial; this is not end-to-end support for either
natural reader function or for general checked access and exceptional flow.
