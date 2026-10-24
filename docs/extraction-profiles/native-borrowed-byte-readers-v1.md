# Native borrowed byte readers: Data-v3 3.33.0

This contract extends the [copied four-byte reader](copied-byte-windows-v1.md)
to the complete safe lz4_flex 0.14.0 architecture reader, in its original library:

```rust
const USIZE_SIZE: usize = core::mem::size_of::<usize>();
let arr: &[u8; USIZE_SIZE] = input[n..n + USIZE_SIZE].try_into().unwrap();
usize::from_ne_bytes(*arr)
```

Authority is pinned separately for aarch64-unknown-linux-gnu,
x86_64-unknown-linux-gnu and aarch64-apple-darwin (captured LLVM triple
`arm64-apple-macosx`), with 64-bit pointers and little-endian semantics. Each
registry binds the exact compiler commit, core crate hash, definition fingerprints
and reviewed core source digests. Authorities cannot be interchanged across targets. Matching stays
`rstyle.ccgraph.data-calls/1.0.0`; scoring and thresholds do not change. This is
bounded extraction coverage, separate from abstraction activation, rewrite
safety and held-out detector quality.

## Compile-time declaration and named length

The only new nested item is an original top-level function-body `const usize`
initialized by exactly `core::mem::size_of::<usize>()`, with no attributes,
visibility or generic parameters. The driver resolves the initializer in the
constant's own type-checking context. Its concrete generic argument, selected
core definition fingerprint, crate authority and evaluated scalar value must
agree with target width eight. A source spelling, value eight or the enclosing
function's type-checking context is insufficient.

The existing scalar-constant record is reused inside one declaration resolution
with the missing initializer callee/argument evidence. No initializer is lowered
as a runtime call. The complete declaration remains in the independently checked
original-source/HIR inventory as `stmt_size_const`; it is never silently dropped.
Its source, owning function and resolved scalar uses are checked. Named array
annotations must bind to that declaration and agree with the normalized byte
array or borrowed-array Result. The bounded Result annotation permits the exact
payload and inferred error type; capture must still identify core's concrete
TryFromSliceError. Arithmetic lengths, other initializer types, nested-block
constants and arbitrary nested items remain excluded.

## Borrowed conversion, conditional origins and read

`TryInto::try_into` must resolve to the same reviewed blanket implementation as
the copied conversion, instantiated to
`Result<&[u8;8], core::array::TryFromSliceError>`. Builtin receiver adjustments,
source receiver span, core definition and full signature are independently
checked. It returns Ok with a shared array view exactly when slice length is
eight; otherwise Err. It performs **no content read**. Its semantic annotation
explicitly states `payload_origins_on=ok` and `content_read=false`. Origins name
possible Ok payloads; they do not assert that Err carries a reference or that Ok
is reachable.

`Result::unwrap` is a separate typed call with its Ok normal-completion
requirement and possible panic. Its borrowed payload origins follow the exact
evaluated Result operand. Saved local Results, saved views, helper-selected
slices and endpoint rebinding compose through source reaching definitions and
independently through serialized value edges. No origin is guessed from the
latest binding name.

Borrowed array origins append the existing `ByteWindow` projection of receiver
operand zero and length eight to their formal byte-slice referent. The view's
actual pointer/value dependency supplies its offset. This is an access-relative
template, not absolute byte identity or alias independence. Source referent
resolution validates the slice-to-array projection without cloning historical
ordinary referents.

The explicit builtin `*arr` performs the array content read. It requires that
precise converted referent and records its formal roots and parameterized view
location. Distinct dereference occurrences keep distinct read-count keys even
when their location templates match. Range bounds and endpoint arithmetic retain
their independent failure/order requirements. The snapshot regression reads
from formal zero while returning the rebound slice from formal one.

`usize::from_ne_bytes` is a separate pinned core call taking exactly `[u8;8]`.
Bytes zero through seven contribute at bit positions 0 through 56 in steps of
eight. It adds no content read. A pure by-value native decoder helper can use the
existing helper contract; a reader with subslicing or unwrap cannot acquire an
existing nonpanicking helper summary.

## Limits and independent expectations

Borrowed conversion is limited to eight bytes and native decoding to usize;
the copied four-byte/u32 contract is preserved. Borrowed four-byte, other native
widths/endian APIs, general Result/Option construction or branching, payload
patterns, `?`, arbitrary trait implementations, mutable references, general const
evaluation, macros, unsafe and unregistered compiler/target authorities remain
outside this change.

A width-minus-one view is a representable partial computation: conversion is
Err, and unwrap cannot normally return. Admission does not assert totality or a
satisfiable normal-completion condition. Exceptional transfer/unwind/formatting
remain omitted and disclosed. Primitive byte arrays, shared references and the
fixed core error add no user destructor effects.

The [development plan](../../eval/coverage-development/README.md#architecture-reader-target-established-before-support)
specifies intended classifications before support and preserves the original
0/8 development baseline separately. Arithmetic runtime expectations cover each
byte, offsets, invalid bounds, overflow, borrowed pointer identity and saved-base
rebinding. Real-Cargo composition tests also exercise helper-selected roots,
native decoder helpers and repeated windows. Independent corruption tests reject
invented/missing reads, changed failure/endian/origin evidence, forged constant
initializer/argument/value and disagreement between declaration, use and named
length. The unchanged natural selection determines real coverage gains.
