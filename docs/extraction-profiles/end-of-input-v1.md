# End-of-input helpers: connected target

Select the complete default-feature debug regex-automata 0.4.18 library at
Data-v3 3.51 (`29100a2`), pinned nightly-2026-09-30, aarch64 Linux. Preserve
original declarations, derives, modules, imports and neighbors. Require composed
std/no_std development capture on the three installed targets; native CI remains
separate evidence. Frozen sixty identities/denominators remain unchanged.

| Original function in src/util/alphabet.rs | Byte range | SHA-256 |
| --- | --- | --- |
| Unit::eoi | 5523..5842 | abfcfdd0fe617b67e4c03728c19fbe1312cc45249b787801746054d3f53111e3 |
| ByteClasses::eoi | 12328..12526 | acaf1984025154fcb1aba0cc52af823cda7b142ecfeb86404360e3329b1f62e8 |

Both are expected in scope from source. Unit::as_usize, ByteClasses::empty,
alphabet_len and get_by_unit already admit in the unchanged natural unit.

The complete blocker chain is:

1. Standard assert with a literal diagnostic and an implicit builtin usize
   capture. Preserve condition evaluation, diagnostic dependencies and failure
   behavior; do not erase formatting or authorize custom Display/hidden effects.
   Current expansion leaves source/HIR/field inventories inconsistent.
2. Exact core TryFrom<usize> for u16 and its concrete integer Result. Unlike
   3.51's widening conversion, this operation can fail. Preserve success payload,
   error boundary and immediate unwrap's possible panic; no invented range proof.
   Current type capture and selected-implementation hook cover only u32→usize.
3. Exact usize::checked_sub → Option<usize> → unwrap, composing alphabet_len,
   Unit::eoi and the final tagged value. Option type capture already works;
   operation summaries/unwrap and their complete normal-flow composition do not.

Unknowns: conditional diagnostic operand correspondence, expanded formatting
operations/drop behavior, and helper panic/result composition after these fixes.
Inspect dependencies before extending capture. Reuse existing nodes, concrete
observations and summaries; no new provenance census or cache is justified.

Independent expectations: Unit::eoi(n) preserves n for 0..=256 and panics above
256, including values that fit u16. ByteClasses::eoi uses the last array byte
plus one for every 256-byte array; other entries do not affect the sentinel.
Check all last-byte values and independently varied neighboring entries, domain
edges, checked-sub underflow, narrowing boundaries and scalar saved/helper values.
Custom formatting, side-effecting diagnostics and unsupported Result composition
must remain excluded unless their complete semantics are explicitly supported.

Before implementation, fix development expectations from source and measure the
3.51 baseline. Existing original-crate tests already cover Unit::eoi's domain;
add an independent original ByteClasses oracle. Activation requires complete
Cargo capture, independent checking, strict admission/matching, mutation and
composition tests, existing regressions, measured unchanged natural/frozen
selections, bounded performance controls, and zero rstyle violations/candidates.
No general formatting, Result matching, `?`, iterator or rewrite-safety claim.
