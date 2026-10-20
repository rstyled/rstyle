# Copied byte windows: Data-v3 3.32.0

This bounded contract connects the safe lz4_flex 0.14.0 `get_batch` expression:

```rust
u32::from_ne_bytes(input[n..n + 4].try_into().unwrap())
```

It applies to the pinned aarch64 compiler/core authority already used for shared
subslice views. Native endianness is little endian in this target contract.
Matching remains `rstyle.ccgraph.data-calls/1.0.0`; thresholds and scoring do not
change. Whole-library capture and unchanged frozen selections determine natural
coverage gains, independently of the development fixtures.

## Supported chain

Each source call remains a separate CALL with its evaluated operand, exact source
span, concrete result type and registered summary. Original invocation/operand
correspondence, compiler/core/target authority and exact definition fingerprints
are checked before eligibility and again during independent document validation.

- `TryInto::try_into` converts a shared `u8` slice to
  `Result<[u8; 4], core::array::TryFromSliceError>`. Its compiler-selected
  implementation must be the pinned blanket `TryInto` implementation. That body
  delegates to `TryFrom`; the concrete shared slice, copied byte array and error
  type select the reviewed core slice-to-array conversion. A familiar method name
  or the trait definition alone is insufficient. Builtin shared receiver
  adjustments must agree with the instantiated signature.
- The conversion returns Ok containing a copy exactly when the evaluated slice
  length is four; otherwise it returns Err. The intermediate has the explicit
  `slice_array_result` type token, never a unit fallback. Inferred and annotated
  local values retain existing definition/read dependencies.
- `Result::unwrap` returns the Ok array payload. Its normal completion requires
  Ok; Err panics. The summary sets `may_panic=true` and does not promise that the
  call always returns. The serialized rule and exceptional-flow assessment retain
  that requirement even if a preceding range looks like a four-byte window.
- `u32::from_ne_bytes` consumes the copied array. Byte positions 0, 1, 2 and 3
  contribute at bit positions 0, 8, 16 and 24 respectively. This operation adds no
  further buffer read. Native conversion of a by-value array also works without
  a preceding slice conversion.

The copy is a conditional physical read on the Ok case, represented as a
conservative may-read and annotated `reads_on=ok`. Its location uses
`rstyle.formal-byte-window/1` with a `ByteWindow` projection naming receiver
operand zero and length four. This is an access-relative location template.
The evaluated receiver pointer/view and its graph dependencies determine the
actual bytes; matching location IDs across accesses does not establish absolute
byte identity or alias independence. Reference counting keeps occurrences
separate. Possible formal roots remain conservative unions.

Saved/nested subslice references, helper-selected shared slice inputs, local
Result/array values and endpoint rebinding compose with the complete call chain.
The existing overloaded-index evaluation order saves the base before endpoints.
The copy consumes that saved view rather than a later binding value. Range bounds
and endpoint arithmetic keep their own normal-completion requirements.

## Explicit limits

Only the copied four-byte Result and native u32 conversion are admitted here.
Borrowed array results, other lengths/integers/endian APIs, Result/Option
construction or branching, payload patterns, `?`, mutable receivers, arbitrary
trait implementations and general library calls remain unsupported. Capture may
observe broader concrete Result shapes without authorizing them.

Wrong-length windows remain representable partial computations: conversion
returns Err, then unwrap has no successful continuation. Admission does not prove
that its normal-completion requirement is satisfiable, nor that a function is
total. Exceptional transfer, formatting on panic and unwinding/cleanup remain
omitted and disclosed. Slice conversion and unwrap cannot be transported by existing helper-body
summaries that claim nonpanicking behavior. A helper containing only the pure
by-value native conversion can use the existing verified helper contract; the
connected regression also exercises that helper after conversion/unwrap.

The primitive byte array, fixed core error type and reviewed conversions have no
user destructors or hidden normal-path effects. This is pinned core semantic
authority, not a claim that Rust enum types or arbitrary conversions are pure.
The five reviewed core source digests live in
`crates/rstyle-pdg/contracts/byte-window-core-source-v1.json` and are checked in
tests. No additional source snapshots or retained caches are introduced.

## Validation expectations

`eval/coverage-development/byte-window-oracle.rs`, also run by workspace tests,
specifies byte significance arithmetically, without using byte-conversion APIs in
the expected-value calculation. It checks offsets, every contributing byte,
outside-window independence, endian/window differences, copied/borrowed
conversion failures, pointer identity and invalid/overflowing endpoints.

The real-Cargo connected regression exercises saved views, helper-selected bases,
local Result/array values and endpoint rebinding through independent verification,
strict admission and equivalent-function matching. It retains exclusions for
wrong-endian calls, other widths, borrowed conversion results and callers needing
unsupported panicking helper summaries. Wrong-length windows retain failure rules.
Missing reads, altered byte order/failure rules, reordered operands and forged
core/selected-implementation identities must fail independent verification.
